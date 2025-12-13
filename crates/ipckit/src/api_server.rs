//! # API Server
//!
//! A lightweight HTTP-over-Socket API layer providing RESTful-style interfaces.
//! Similar to Docker Engine API design, this module implements HTTP parsing
//! and routing on top of the Socket Server.
//!
//! ## Features
//!
//! - Lightweight HTTP/1.1 parsing (no heavy framework dependencies)
//! - RESTful routing with path parameters
//! - JSON request/response bodies
//! - Streaming responses (SSE)
//! - Middleware support
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit::{ApiServer, Request, Response};
//!
//! let mut server = ApiServer::new(Default::default())?;
//!
//! server.router()
//!     .get("/v1/tasks", |_req| Response::ok(json!([])))
//!     .get("/v1/tasks/{id}", |req| {
//!         let id = req.params.get("id").unwrap();
//!         Response::ok(json!({"id": id}))
//!     })
//!     .post("/v1/tasks", |req| {
//!         Response::created(json!({"id": "new-task"}))
//!     });
//!
//! server.run()?;
//! ```

use crate::socket_server::{
    Connection, ConnectionHandler, Message, SocketClient, SocketServer, SocketServerConfig,
};
use crate::IpcError;
use parking_lot::RwLock;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::sync::Arc;

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

impl Method {
    /// Parse method from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Method::GET),
            "POST" => Some(Method::POST),
            "PUT" => Some(Method::PUT),
            "DELETE" => Some(Method::DELETE),
            "PATCH" => Some(Method::PATCH),
            "OPTIONS" => Some(Method::OPTIONS),
            "HEAD" => Some(Method::HEAD),
            _ => None,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
            Method::OPTIONS => "OPTIONS",
            Method::HEAD => "HEAD",
        }
    }
}

/// HTTP request.
#[derive(Debug)]
pub struct Request {
    /// HTTP method
    pub method: Method,
    /// Request path (without query string)
    pub path: String,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (parsed as JSON if Content-Type is application/json)
    pub body: Option<JsonValue>,
    /// Raw body bytes
    pub raw_body: Vec<u8>,
    /// Path parameters (extracted from route matching)
    pub params: HashMap<String, String>,
}

impl Request {
    /// Create a new request.
    pub fn new(method: Method, path: &str) -> Self {
        Self {
            method,
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            raw_body: Vec::new(),
            params: HashMap::new(),
        }
    }

    /// Get a query parameter.
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(|s| s.as_str())
    }

    /// Get a path parameter.
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }

    /// Get a header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    /// Get the Content-Type header.
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Check if the request accepts JSON.
    pub fn accepts_json(&self) -> bool {
        self.header("accept")
            .map(|s| s.contains("application/json") || s.contains("*/*"))
            .unwrap_or(true)
    }

    /// Parse the request from raw HTTP data.
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let mut reader = BufReader::new(data);
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(ParseError::InvalidRequestLine);
        }

        let method = Method::parse(parts[0]).ok_or(ParseError::InvalidMethod)?;
        let full_path = parts[1];

        // Parse path and query string
        let (path, query) = if let Some(idx) = full_path.find('?') {
            let path = &full_path[..idx];
            let query_str = &full_path[idx + 1..];
            (path.to_string(), parse_query_string(query_str))
        } else {
            (full_path.to_string(), HashMap::new())
        };

        // Parse headers
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some(idx) = line.find(':') {
                let key = line[..idx].trim().to_lowercase();
                let value = line[idx + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Parse body
        let mut raw_body = Vec::new();
        if let Some(len_str) = headers.get("content-length") {
            if let Ok(len) = len_str.parse::<usize>() {
                raw_body.resize(len, 0);
                reader.read_exact(&mut raw_body)?;
            }
        }

        // Try to parse body as JSON
        let body = if !raw_body.is_empty() {
            let content_type = headers.get("content-type").map(|s| s.as_str());
            if content_type
                .map(|s| s.contains("application/json"))
                .unwrap_or(false)
            {
                serde_json::from_slice(&raw_body).ok()
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            method,
            path,
            query,
            headers,
            body,
            raw_body,
            params: HashMap::new(),
        })
    }
}

/// Parse error.
#[derive(Debug)]
pub enum ParseError {
    InvalidRequestLine,
    InvalidMethod,
    IoError(std::io::Error),
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::IoError(e)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidRequestLine => write!(f, "Invalid request line"),
            ParseError::InvalidMethod => write!(f, "Invalid HTTP method"),
            ParseError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if let Some(idx) = pair.find('=') {
            let key = urlencoding_decode(&pair[..idx]);
            let value = urlencoding_decode(&pair[idx + 1..]);
            params.insert(key, value);
        } else if !pair.is_empty() {
            params.insert(urlencoding_decode(pair), String::new());
        }
    }
    params
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// HTTP response.
#[derive(Debug)]
pub struct Response {
    /// HTTP status code
    pub status: u16,
    /// Status message
    pub status_message: String,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: ResponseBody,
}

/// Response body type.
#[derive(Debug)]
pub enum ResponseBody {
    /// JSON response
    Json(JsonValue),
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Text response
    Text(String),
    /// Empty response
    Empty,
}

impl Response {
    /// Create a new response with status code.
    pub fn new(status: u16) -> Self {
        Self {
            status,
            status_message: status_message(status).to_string(),
            headers: HashMap::new(),
            body: ResponseBody::Empty,
        }
    }

    /// Create a 200 OK response with JSON body.
    pub fn ok(body: JsonValue) -> Self {
        let mut resp = Self::new(200);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(body);
        resp
    }

    /// Create a 201 Created response with JSON body.
    pub fn created(body: JsonValue) -> Self {
        let mut resp = Self::new(201);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(body);
        resp
    }

    /// Create a 204 No Content response.
    pub fn no_content() -> Self {
        Self::new(204)
    }

    /// Create a 400 Bad Request response.
    pub fn bad_request(message: &str) -> Self {
        let mut resp = Self::new(400);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(serde_json::json!({
            "error": "Bad Request",
            "message": message
        }));
        resp
    }

    /// Create a 401 Unauthorized response.
    pub fn unauthorized(message: &str) -> Self {
        let mut resp = Self::new(401);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(serde_json::json!({
            "error": "Unauthorized",
            "message": message
        }));
        resp
    }

    /// Create a 403 Forbidden response.
    pub fn forbidden(message: &str) -> Self {
        let mut resp = Self::new(403);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(serde_json::json!({
            "error": "Forbidden",
            "message": message
        }));
        resp
    }

    /// Create a 404 Not Found response.
    pub fn not_found() -> Self {
        let mut resp = Self::new(404);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(serde_json::json!({
            "error": "Not Found"
        }));
        resp
    }

    /// Create a 500 Internal Server Error response.
    pub fn internal_error(message: &str) -> Self {
        let mut resp = Self::new(500);
        resp.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        resp.body = ResponseBody::Json(serde_json::json!({
            "error": "Internal Server Error",
            "message": message
        }));
        resp
    }

    /// Set a header.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the body as JSON.
    pub fn json(mut self, body: JsonValue) -> Self {
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self.body = ResponseBody::Json(body);
        self
    }

    /// Set the body as text.
    pub fn text(mut self, body: &str) -> Self {
        self.headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
        self.body = ResponseBody::Text(body.to_string());
        self
    }

    /// Set the body as bytes.
    pub fn bytes(mut self, body: Vec<u8>, content_type: &str) -> Self {
        self.headers
            .insert("Content-Type".to_string(), content_type.to_string());
        self.body = ResponseBody::Bytes(body);
        self
    }

    /// Convert response to HTTP bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let body_bytes = match &self.body {
            ResponseBody::Json(v) => serde_json::to_vec(v).unwrap_or_default(),
            ResponseBody::Bytes(b) => b.clone(),
            ResponseBody::Text(s) => s.as_bytes().to_vec(),
            ResponseBody::Empty => Vec::new(),
        };

        let mut output = format!("HTTP/1.1 {} {}\r\n", self.status, self.status_message);

        // Add headers
        for (key, value) in &self.headers {
            output.push_str(&format!("{}: {}\r\n", key, value));
        }

        // Add content-length
        output.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
        output.push_str("\r\n");

        let mut bytes = output.into_bytes();
        bytes.extend(body_bytes);
        bytes
    }
}

fn status_message(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

/// Path segment for pattern matching.
#[derive(Debug, Clone)]
enum PathSegment {
    /// Static path segment
    Static(String),
    /// Parameter path segment {:name}
    Param(String),
    /// Wildcard {*rest}
    Wildcard(String),
}

/// Path pattern for route matching.
#[derive(Debug, Clone)]
pub struct PathPattern {
    segments: Vec<PathSegment>,
    #[allow(dead_code)]
    original: String,
}

impl PathPattern {
    /// Parse a path pattern.
    pub fn parse(pattern: &str) -> Self {
        let segments: Vec<PathSegment> = pattern
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.starts_with("{*") && s.ends_with('}') {
                    PathSegment::Wildcard(s[2..s.len() - 1].to_string())
                } else if s.starts_with('{') && s.ends_with('}') {
                    PathSegment::Param(s[1..s.len() - 1].to_string())
                } else {
                    PathSegment::Static(s.to_string())
                }
            })
            .collect();

        Self {
            segments,
            original: pattern.to_string(),
        }
    }

    /// Match a path against this pattern.
    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_segments: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut params = HashMap::new();
        let mut path_idx = 0;

        for seg in self.segments.iter() {
            match seg {
                PathSegment::Static(s) => {
                    if path_idx >= path_segments.len() || path_segments[path_idx] != s {
                        return None;
                    }
                    path_idx += 1;
                }
                PathSegment::Param(name) => {
                    if path_idx >= path_segments.len() {
                        return None;
                    }
                    params.insert(name.clone(), path_segments[path_idx].to_string());
                    path_idx += 1;
                }
                PathSegment::Wildcard(name) => {
                    // Consume all remaining segments
                    let rest: Vec<&str> = path_segments[path_idx..].to_vec();
                    params.insert(name.clone(), rest.join("/"));
                    return Some(params);
                }
            }
        }

        // Check if we consumed all path segments
        if path_idx == path_segments.len() {
            Some(params)
        } else {
            None
        }
    }
}

/// Route handler function type.
pub type HandlerFn = Box<dyn Fn(Request) -> Response + Send + Sync>;

/// A route definition.
struct Route {
    method: Method,
    pattern: PathPattern,
    handler: HandlerFn,
}

/// Middleware function type.
pub type MiddlewareFn =
    Box<dyn Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync>;

/// API router.
pub struct Router {
    routes: Vec<Route>,
    middlewares: Vec<MiddlewareFn>,
    not_found_handler: Option<HandlerFn>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            middlewares: Vec::new(),
            not_found_handler: None,
        }
    }

    /// Register a GET route.
    pub fn get<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.route(Method::GET, path, handler)
    }

    /// Register a POST route.
    pub fn post<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.route(Method::POST, path, handler)
    }

    /// Register a PUT route.
    pub fn put<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.route(Method::PUT, path, handler)
    }

    /// Register a DELETE route.
    pub fn delete<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.route(Method::DELETE, path, handler)
    }

    /// Register a PATCH route.
    pub fn patch<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.route(Method::PATCH, path, handler)
    }

    /// Register a route with a specific method.
    pub fn route<F>(&mut self, method: Method, path: &str, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.routes.push(Route {
            method,
            pattern: PathPattern::parse(path),
            handler: Box::new(handler),
        });
        self
    }

    /// Add middleware.
    pub fn middleware<F>(&mut self, middleware: F) -> &mut Self
    where
        F: Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync + 'static,
    {
        self.middlewares.push(Box::new(middleware));
        self
    }

    /// Set custom 404 handler.
    pub fn not_found<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.not_found_handler = Some(Box::new(handler));
        self
    }

    /// Handle a request.
    pub fn handle(&self, mut req: Request) -> Response {
        // Find matching route
        for route in &self.routes {
            if route.method == req.method {
                if let Some(params) = route.pattern.matches(&req.path) {
                    req.params = params;

                    // Apply middlewares
                    if self.middlewares.is_empty() {
                        return (route.handler)(req);
                    } else {
                        let handler = &route.handler;
                        let mut chain: Box<dyn Fn(Request) -> Response + '_> = Box::new(handler);

                        for middleware in self.middlewares.iter().rev() {
                            let next = chain;
                            chain = Box::new(move |r| middleware(r, &*next));
                        }

                        return chain(req);
                    }
                }
            }
        }

        // No route found
        if let Some(ref handler) = self.not_found_handler {
            handler(req)
        } else {
            Response::not_found()
        }
    }
}

/// API Server configuration.
#[derive(Debug, Clone)]
pub struct ApiServerConfig {
    /// Socket server configuration
    pub socket_config: SocketServerConfig,
    /// Enable CORS
    pub enable_cors: bool,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            socket_config: SocketServerConfig::default(),
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// API Server handler for socket connections.
#[derive(Clone)]
struct ApiHandler {
    router: Arc<RwLock<Router>>,
    config: ApiServerConfig,
}

impl ConnectionHandler for ApiHandler {
    fn on_message(&self, _conn: &mut Connection, msg: Message) -> crate::Result<Option<Message>> {
        // Get the raw HTTP data from the message
        let data = if let Some(binary_data) = msg.as_binary() {
            binary_data
        } else if let Some(text) = msg.as_text() {
            text.as_bytes().to_vec()
        } else {
            // Try to serialize the payload as the request
            serde_json::to_vec(&msg.payload).unwrap_or_default()
        };

        // Parse request from message data
        let request = match Request::parse(&data) {
            Ok(req) => req,
            Err(e) => {
                let resp = Response::bad_request(&e.to_string());
                return Ok(Some(Message::binary(resp.to_bytes())));
            }
        };

        // Handle CORS preflight
        if request.method == Method::OPTIONS && self.config.enable_cors {
            let resp = self.cors_preflight_response();
            return Ok(Some(Message::binary(resp.to_bytes())));
        }

        // Route the request
        let mut response = self.router.read().handle(request);

        // Add CORS headers
        if self.config.enable_cors {
            self.add_cors_headers(&mut response);
        }

        Ok(Some(Message::binary(response.to_bytes())))
    }
}

impl ApiHandler {
    fn cors_preflight_response(&self) -> Response {
        let origin = if self.config.cors_origins.contains(&"*".to_string()) {
            "*".to_string()
        } else {
            self.config.cors_origins.join(", ")
        };

        Response::new(204)
            .header("Access-Control-Allow-Origin", &origin)
            .header(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, PATCH, OPTIONS",
            )
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization",
            )
            .header("Access-Control-Max-Age", "86400")
    }

    fn add_cors_headers(&self, response: &mut Response) {
        let origin = if self.config.cors_origins.contains(&"*".to_string()) {
            "*".to_string()
        } else {
            self.config.cors_origins.join(", ")
        };

        response
            .headers
            .insert("Access-Control-Allow-Origin".to_string(), origin);
    }
}

/// API Server.
pub struct ApiServer {
    config: ApiServerConfig,
    router: Arc<RwLock<Router>>,
}

impl ApiServer {
    /// Create a new API server.
    pub fn new(config: ApiServerConfig) -> Self {
        Self {
            config,
            router: Arc::new(RwLock::new(Router::new())),
        }
    }

    /// Get mutable reference to the router.
    pub fn router(&self) -> impl std::ops::DerefMut<Target = Router> + '_ {
        self.router.write()
    }

    /// Run the server (blocking).
    pub fn run(self) -> crate::Result<()> {
        let handler = ApiHandler {
            router: Arc::clone(&self.router),
            config: self.config.clone(),
        };

        let server = SocketServer::new(self.config.socket_config)?;
        server.run(handler)
    }

    /// Start the server in a background thread.
    pub fn spawn(self) -> std::thread::JoinHandle<crate::Result<()>> {
        std::thread::spawn(move || self.run())
    }
}

/// API Client for making requests to the API server.
pub struct ApiClient {
    socket_path: String,
}

impl ApiClient {
    /// Create a new API client.
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
        }
    }

    /// Connect to the default socket.
    pub fn connect() -> Self {
        Self::new(&SocketServerConfig::default().path)
    }

    /// Make a GET request.
    pub fn get(&self, path: &str) -> crate::Result<JsonValue> {
        self.request(Method::GET, path, None)
    }

    /// Make a POST request.
    pub fn post(&self, path: &str, body: Option<JsonValue>) -> crate::Result<JsonValue> {
        self.request(Method::POST, path, body)
    }

    /// Make a PUT request.
    pub fn put(&self, path: &str, body: Option<JsonValue>) -> crate::Result<JsonValue> {
        self.request(Method::PUT, path, body)
    }

    /// Make a DELETE request.
    pub fn delete(&self, path: &str) -> crate::Result<JsonValue> {
        self.request(Method::DELETE, path, None)
    }

    /// Make a request.
    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<JsonValue>,
    ) -> crate::Result<JsonValue> {
        let mut client = SocketClient::connect(&self.socket_path)?;

        // Build HTTP request
        let body_bytes = body
            .as_ref()
            .map(|b| serde_json::to_vec(b).unwrap_or_default())
            .unwrap_or_default();

        let request_str = format!(
            "{} {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            method.as_str(),
            path,
            body_bytes.len()
        );

        let mut request_bytes = request_str.into_bytes();
        request_bytes.extend(body_bytes);

        // Send as binary message
        let msg = Message::binary(request_bytes);
        client.send(&msg)?;

        // Read response
        let response = client.recv()?;

        // Extract response body
        if let Some(binary_data) = response.as_binary() {
            if let Some(body_start) = find_body_start(&binary_data) {
                let body = &binary_data[body_start..];
                serde_json::from_slice(body).map_err(|e| IpcError::Serialization(e.to_string()))
            } else {
                Ok(JsonValue::Null)
            }
        } else if let Some(text) = response.as_text() {
            serde_json::from_str(text).map_err(|e| IpcError::Deserialization(e.to_string()))
        } else {
            // Try to return the payload directly
            Ok(response.payload)
        }
    }
}

fn find_body_start(data: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(3) {
        if &data[i..i + 4] == b"\r\n\r\n" {
            return Some(i + 4);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_pattern_static() {
        let pattern = PathPattern::parse("/v1/tasks");
        assert!(pattern.matches("/v1/tasks").is_some());
        assert!(pattern.matches("/v1/tasks/").is_some());
        assert!(pattern.matches("/v1/other").is_none());
    }

    #[test]
    fn test_path_pattern_param() {
        let pattern = PathPattern::parse("/v1/tasks/{id}");

        let params = pattern.matches("/v1/tasks/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        let params = pattern.matches("/v1/tasks/abc").unwrap();
        assert_eq!(params.get("id"), Some(&"abc".to_string()));

        assert!(pattern.matches("/v1/tasks").is_none());
        assert!(pattern.matches("/v1/tasks/123/extra").is_none());
    }

    #[test]
    fn test_path_pattern_wildcard() {
        let pattern = PathPattern::parse("/files/{*path}");

        let params = pattern.matches("/files/a/b/c").unwrap();
        assert_eq!(params.get("path"), Some(&"a/b/c".to_string()));

        let params = pattern.matches("/files/single").unwrap();
        assert_eq!(params.get("path"), Some(&"single".to_string()));
    }

    #[test]
    fn test_router() {
        let mut router = Router::new();
        router.get("/v1/tasks", |_| Response::ok(serde_json::json!([])));
        router.get("/v1/tasks/{id}", |req| {
            let id = req.params.get("id").unwrap();
            Response::ok(serde_json::json!({"id": id}))
        });

        let req = Request::new(Method::GET, "/v1/tasks");
        let resp = router.handle(req);
        assert_eq!(resp.status, 200);

        let req = Request::new(Method::GET, "/v1/tasks/123");
        let resp = router.handle(req);
        assert_eq!(resp.status, 200);

        let req = Request::new(Method::GET, "/not/found");
        let resp = router.handle(req);
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_response_to_bytes() {
        let resp = Response::ok(serde_json::json!({"key": "value"}));
        let bytes = resp.to_bytes();
        let text = String::from_utf8_lossy(&bytes);

        assert!(text.contains("HTTP/1.1 200 OK"));
        assert!(text.contains("Content-Type: application/json"));
        assert!(text.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_request_parse() {
        let raw = b"GET /v1/tasks?limit=10 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = Request::parse(raw).unwrap();

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/v1/tasks");
        assert_eq!(req.query.get("limit"), Some(&"10".to_string()));
    }
}
