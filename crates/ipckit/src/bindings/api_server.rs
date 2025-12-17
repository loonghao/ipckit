//! Python bindings for API Server

use crate::api_server::{ApiClient, ApiServerConfig, Request, Response};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

use super::json_utils::{json_value_to_py, py_to_json_value};

// Suppress deprecation warnings for PyObject (Py<PyAny> alias)
#[allow(dead_code)]
type PyObject = Py<PyAny>;

/// Python wrapper for ApiServerConfig.
#[pyclass(name = "ApiServerConfig")]
#[derive(Clone)]
pub struct PyApiServerConfig {
    inner: ApiServerConfig,
}

#[pymethods]
impl PyApiServerConfig {
    #[new]
    #[pyo3(signature = (socket_path=None, enable_cors=true, cors_origins=None))]
    fn new(
        socket_path: Option<String>,
        enable_cors: bool,
        cors_origins: Option<Vec<String>>,
    ) -> Self {
        let mut config = ApiServerConfig::default();

        if let Some(path) = socket_path {
            config.socket_config.path = path;
        }

        config.enable_cors = enable_cors;

        if let Some(origins) = cors_origins {
            config.cors_origins = origins;
        }

        Self { inner: config }
    }

    #[getter]
    fn socket_path(&self) -> String {
        self.inner.socket_config.path.clone()
    }

    #[setter]
    fn set_socket_path(&mut self, path: String) {
        self.inner.socket_config.path = path;
    }

    #[getter]
    fn enable_cors(&self) -> bool {
        self.inner.enable_cors
    }

    #[setter]
    fn set_enable_cors(&mut self, value: bool) {
        self.inner.enable_cors = value;
    }

    #[getter]
    fn cors_origins(&self) -> Vec<String> {
        self.inner.cors_origins.clone()
    }

    #[setter]
    fn set_cors_origins(&mut self, origins: Vec<String>) {
        self.inner.cors_origins = origins;
    }

    fn __repr__(&self) -> String {
        format!(
            "ApiServerConfig(socket_path='{}', enable_cors={}, cors_origins={:?})",
            self.inner.socket_config.path, self.inner.enable_cors, self.inner.cors_origins
        )
    }
}

/// Python wrapper for Request.
#[pyclass(name = "Request")]
pub struct PyRequest {
    /// HTTP method
    #[pyo3(get)]
    pub method: String,
    /// Request path
    #[pyo3(get)]
    pub path: String,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (as Python object)
    pub body: Option<serde_json::Value>,
    /// Path parameters
    pub params: HashMap<String, String>,
}

#[pymethods]
impl PyRequest {
    /// Get query parameters as dict.
    #[getter]
    fn query(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.query {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get headers as dict.
    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get path parameters as dict.
    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.params {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get body as Python object.
    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &self.body {
            Some(v) => json_value_to_py(py, v),
            None => Ok(py.None()),
        }
    }

    /// Get a query parameter.
    fn query_param(&self, name: &str) -> Option<String> {
        self.query.get(name).cloned()
    }

    /// Get a path parameter.
    fn path_param(&self, name: &str) -> Option<String> {
        self.params.get(name).cloned()
    }

    /// Get a header value.
    fn header(&self, name: &str) -> Option<String> {
        self.headers.get(&name.to_lowercase()).cloned()
    }

    fn __repr__(&self) -> String {
        format!("Request(method='{}', path='{}')", self.method, self.path)
    }
}

impl PyRequest {
    #[allow(dead_code)]
    fn from_rust(req: &Request) -> Self {
        Self {
            method: req.method.as_str().to_string(),
            path: req.path.clone(),
            query: req.query.clone(),
            headers: req.headers.clone(),
            body: req.body.clone(),
            params: req.params.clone(),
        }
    }
}

/// Python wrapper for Response.
#[pyclass(name = "Response")]
#[derive(Clone)]
pub struct PyResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Option<serde_json::Value>,
}

#[pymethods]
impl PyResponse {
    /// Create a new response with status code.
    #[new]
    #[pyo3(signature = (status=200))]
    fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Create a 200 OK response with JSON body.
    #[staticmethod]
    fn ok(body: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_body = py_to_json_value(body)?;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(Self {
            status: 200,
            headers,
            body: Some(json_body),
        })
    }

    /// Create a 201 Created response with JSON body.
    #[staticmethod]
    fn created(body: &Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let json_body = py_to_json_value(body)?;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(Self {
            status: 201,
            headers,
            body: Some(json_body),
        })
    }

    /// Create a 204 No Content response.
    #[staticmethod]
    fn no_content() -> Self {
        Self {
            status: 204,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Create a 400 Bad Request response.
    #[staticmethod]
    fn bad_request(message: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Self {
            status: 400,
            headers,
            body: Some(serde_json::json!({
                "error": "Bad Request",
                "message": message
            })),
        }
    }

    /// Create a 404 Not Found response.
    #[staticmethod]
    fn not_found() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Self {
            status: 404,
            headers,
            body: Some(serde_json::json!({"error": "Not Found"})),
        }
    }

    /// Create a 500 Internal Server Error response.
    #[staticmethod]
    fn internal_error(message: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Self {
            status: 500,
            headers,
            body: Some(serde_json::json!({
                "error": "Internal Server Error",
                "message": message
            })),
        }
    }

    /// Set a header and return self for chaining.
    fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Set the body as JSON.
    fn set_json(&mut self, body: &Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let json_body = py_to_json_value(body)?;
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self.body = Some(json_body);
        Ok(())
    }

    #[getter]
    fn status(&self) -> u16 {
        self.status
    }

    fn __repr__(&self) -> String {
        format!("Response(status={})", self.status)
    }
}

impl PyResponse {
    #[allow(dead_code)]
    fn to_rust(&self) -> Response {
        let mut resp = Response::new(self.status);
        resp.headers = self.headers.clone();
        if let Some(body) = &self.body {
            resp.body = crate::api_server::ResponseBody::Json(body.clone());
        }
        resp
    }
}

/// Python wrapper for ApiClient.
#[pyclass(name = "ApiClient")]
pub struct PyApiClient {
    inner: ApiClient,
}

#[pymethods]
impl PyApiClient {
    /// Create a new API client.
    ///
    /// Args:
    ///     socket_path: Path to the socket file
    ///     timeout_ms: Optional connection timeout in milliseconds
    #[new]
    #[pyo3(signature = (socket_path, timeout_ms=None))]
    fn new(socket_path: &str, timeout_ms: Option<u64>) -> Self {
        let inner = match timeout_ms {
            Some(ms) => ApiClient::with_timeout(socket_path, std::time::Duration::from_millis(ms)),
            None => ApiClient::new(socket_path),
        };
        Self { inner }
    }

    /// Connect to the default socket.
    #[staticmethod]
    fn connect() -> Self {
        Self {
            inner: ApiClient::connect(),
        }
    }

    /// Connect to the default socket with a timeout.
    ///
    /// Args:
    ///     timeout_ms: Connection timeout in milliseconds
    #[staticmethod]
    fn connect_timeout(timeout_ms: u64) -> Self {
        Self {
            inner: ApiClient::connect_timeout(std::time::Duration::from_millis(timeout_ms)),
        }
    }

    /// Set the connection timeout.
    ///
    /// Args:
    ///     timeout_ms: Timeout in milliseconds, or None to disable timeout
    fn set_timeout(&mut self, timeout_ms: Option<u64>) {
        self.inner
            .set_timeout(timeout_ms.map(std::time::Duration::from_millis));
    }

    /// Get the connection timeout in milliseconds.
    ///
    /// Returns:
    ///     Optional timeout in milliseconds, or None if no timeout is set
    fn get_timeout(&self) -> Option<u64> {
        self.inner.get_timeout().map(|d| d.as_millis() as u64)
    }

    /// Make a GET request.
    fn get(&self, py: Python<'_>, path: &str) -> PyResult<Py<PyAny>> {
        let path_owned = path.to_string();
        let result = std::thread::scope(|_| self.inner.get(&path_owned));
        result
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
            .and_then(|v| json_value_to_py(py, &v))
    }

    /// Make a POST request.
    #[pyo3(signature = (path, body=None))]
    fn post(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, pyo3::PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let json_body = match body {
            Some(b) => Some(py_to_json_value(b)?),
            None => None,
        };

        let path_owned = path.to_string();
        let result = std::thread::scope(|_| self.inner.post(&path_owned, json_body));
        result
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
            .and_then(|v| json_value_to_py(py, &v))
    }

    /// Make a PUT request.
    #[pyo3(signature = (path, body=None))]
    fn put(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, pyo3::PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let json_body = match body {
            Some(b) => Some(py_to_json_value(b)?),
            None => None,
        };

        let path_owned = path.to_string();
        let result = std::thread::scope(|_| self.inner.put(&path_owned, json_body));
        result
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
            .and_then(|v| json_value_to_py(py, &v))
    }

    /// Make a DELETE request.
    fn delete(&self, py: Python<'_>, path: &str) -> PyResult<Py<PyAny>> {
        let path_owned = path.to_string();
        let result = std::thread::scope(|_| self.inner.delete(&path_owned));
        result
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
            .and_then(|v| json_value_to_py(py, &v))
    }

    fn __repr__(&self) -> String {
        match self.inner.get_timeout() {
            Some(t) => format!("ApiClient(timeout={}ms)", t.as_millis()),
            None => "ApiClient()".to_string(),
        }
    }
}
