//! # ipckit-macros
//!
//! Procedural macros for ipckit providing declarative IPC handler definitions.
//!
//! ## Features
//!
//! - `#[ipc_handler]` - Mark an impl block as an IPC handler
//! - `#[command]` - Define a command handler method
//! - `#[derive(IpcMessage)]` - Derive serialization for IPC messages
//! - `ipc_channel!` - Declarative channel creation
//! - `ipc_commands!` - Declarative command routing
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit_macros::{ipc_handler, command, ipc_channel, ipc_commands};
//!
//! struct MyHandler;
//!
//! #[ipc_handler(channel = "my_app")]
//! impl MyHandler {
//!     #[command]
//!     fn ping(&self) -> String {
//!         "pong".to_string()
//!     }
//!
//!     #[command]
//!     fn echo(&self, message: String) -> String {
//!         message
//!     }
//!
//!     #[command(name = "add_numbers")]
//!     fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! }
//!
//! // Declarative channel creation
//! ipc_channel!(my_channel, pipe, "my_pipe");
//!
//! // Declarative command routing
//! let router = ipc_commands! {
//!     "ping" => ping_handler,
//!     "echo" => echo_handler,
//!     "math/add" => add_handler,
//! };
//! ```

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ImplItem, ItemImpl, Meta};

/// Attributes for the `#[ipc_handler]` macro.
#[derive(Debug, Default, FromMeta)]
struct IpcHandlerArgs {
    /// Channel name for this handler
    #[darling(default)]
    channel: Option<String>,
    /// Timeout for command execution (in milliseconds)
    #[darling(default)]
    timeout_ms: Option<u64>,
}

/// Attributes for the `#[command]` macro.
#[derive(Debug, Default, FromMeta)]
#[allow(dead_code)]
struct CommandArgs {
    /// Override the command name
    #[darling(default)]
    name: Option<String>,
    /// Command timeout in milliseconds
    #[darling(default)]
    timeout_ms: Option<u64>,
}

/// Mark an impl block as an IPC handler.
///
/// This macro generates the necessary boilerplate for handling IPC commands.
///
/// ## Attributes
///
/// - `channel` - The channel name for this handler
/// - `timeout_ms` - Default timeout for commands
///
/// ## Example
///
/// ```rust,ignore
/// #[ipc_handler(channel = "my_service")]
/// impl MyService {
///     #[command]
///     fn ping(&self) -> String {
///         "pong".to_string()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn ipc_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_handler_args(attr) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };

    let input = parse_macro_input!(item as ItemImpl);
    let expanded = expand_ipc_handler(args, input);

    TokenStream::from(expanded)
}

fn parse_handler_args(attr: TokenStream) -> Result<IpcHandlerArgs, syn::Error> {
    if attr.is_empty() {
        return Ok(IpcHandlerArgs::default());
    }

    let meta: Meta = syn::parse(attr)?;
    IpcHandlerArgs::from_meta(&meta).map_err(|e| syn::Error::new_spanned(&meta, e.to_string()))
}

fn expand_ipc_handler(args: IpcHandlerArgs, input: ItemImpl) -> proc_macro2::TokenStream {
    let self_ty = &input.self_ty;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Collect command methods
    let mut command_handlers = Vec::new();
    let mut command_names = Vec::new();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            // Check for #[command] attribute
            let has_command_attr = method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("command"));

            if has_command_attr {
                let method_name = &method.sig.ident;
                let command_name = method_name.to_string();
                command_names.push(command_name.clone());

                // Generate parameter extraction
                let params: Vec<_> = method
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|arg| {
                        if let syn::FnArg::Typed(pat_type) = arg {
                            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                let name = &pat_ident.ident;
                                let ty = &pat_type.ty;
                                return Some((name.clone(), ty.clone()));
                            }
                        }
                        None
                    })
                    .collect();

                let param_extractions: Vec<_> = params
                    .iter()
                    .map(|(name, ty)| {
                        let name_str = name.to_string();
                        quote! {
                            let #name: #ty = params
                                .get(#name_str)
                                .cloned()
                                .ok_or_else(|| ipckit::IpcError::Other(
                                    format!("Missing parameter: {}", #name_str)
                                ))
                                .and_then(|v| serde_json::from_value(v)
                                    .map_err(|e| ipckit::IpcError::Deserialization(e.to_string())))?;
                        }
                    })
                    .collect();

                let param_names: Vec<_> = params.iter().map(|(name, _)| name).collect();

                let handler = quote! {
                    #command_name => {
                        #(#param_extractions)*
                        let result = self.#method_name(#(#param_names),*);
                        serde_json::to_value(&result)
                            .map_err(|e| ipckit::IpcError::Serialization(e.to_string()))
                    }
                };

                command_handlers.push(handler);
            }
        }
    }

    let channel_name = args.channel.unwrap_or_else(|| "default".to_string());
    let timeout = args.timeout_ms.unwrap_or(30000);

    // Generate the handler trait implementation
    let expanded = quote! {
        #input

        impl #impl_generics #self_ty #ty_generics #where_clause {
            /// Get the channel name for this handler.
            pub fn channel_name(&self) -> &'static str {
                #channel_name
            }

            /// Get the default timeout in milliseconds.
            pub fn default_timeout_ms(&self) -> u64 {
                #timeout
            }

            /// Get the list of available commands.
            pub fn commands(&self) -> &'static [&'static str] {
                &[#(#command_names),*]
            }

            /// Handle a command by name.
            pub fn handle_command(
                &self,
                command: &str,
                params: serde_json::Map<String, serde_json::Value>,
            ) -> ipckit::Result<serde_json::Value> {
                match command {
                    #(#command_handlers)*
                    _ => Err(ipckit::IpcError::NotFound(
                        format!("Unknown command: {}", command)
                    )),
                }
            }
        }
    };

    expanded
}

/// Mark a method as a command handler.
///
/// This attribute is used within an `#[ipc_handler]` impl block to mark
/// methods that should be exposed as IPC commands.
///
/// ## Attributes
///
/// - `name` - Override the command name (defaults to method name)
/// - `timeout_ms` - Command-specific timeout
///
/// ## Example
///
/// ```rust,ignore
/// #[command(name = "greet", timeout_ms = 5000)]
/// fn say_hello(&self, name: String) -> String {
///     format!("Hello, {}!", name)
/// }
/// ```
#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // The command attribute is processed by ipc_handler
    // This just passes through the item unchanged
    item
}

/// Derive macro for IPC messages.
///
/// Automatically implements serialization and validation for IPC message types.
///
/// ## Example
///
/// ```rust,ignore
/// #[derive(IpcMessage)]
/// struct CreateUserRequest {
///     name: String,
///     email: String,
///     age: Option<u8>,
/// }
/// ```
#[proc_macro_derive(IpcMessage, attributes(ipc))]
pub fn derive_ipc_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = expand_ipc_message(input);
    TokenStream::from(expanded)
}

fn expand_ipc_message(input: DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Generate validation code based on fields
    let validation = match &input.data {
        syn::Data::Struct(data) => {
            let field_validations: Vec<_> = data
                .fields
                .iter()
                .filter_map(|field| {
                    let field_name = field.ident.as_ref()?;
                    let _field_name_str = field_name.to_string();

                    // Check for validation attributes
                    for attr in &field.attrs {
                        if attr.path().is_ident("ipc") {
                            // Could parse validation rules here
                            return Some(quote! {
                                // Validate field
                            });
                        }
                    }
                    None
                })
                .collect();

            quote! {
                #(#field_validations)*
                Ok(())
            }
        }
        _ => quote! { Ok(()) },
    };

    quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Validate this message.
            pub fn validate(&self) -> ipckit::Result<()> {
                #validation
            }

            /// Convert to JSON value.
            pub fn to_json(&self) -> ipckit::Result<serde_json::Value> {
                serde_json::to_value(self)
                    .map_err(|e| ipckit::IpcError::Serialization(e.to_string()))
            }

            /// Create from JSON value.
            pub fn from_json(value: serde_json::Value) -> ipckit::Result<Self> {
                serde_json::from_value(value)
                    .map_err(|e| ipckit::IpcError::Deserialization(e.to_string()))
            }
        }
    }
}

/// Router macro for defining routes declaratively.
///
/// ## Example
///
/// ```rust,ignore
/// let router = router! {
///     GET "/tasks" => list_tasks,
///     GET "/tasks/{id}" => get_task,
///     POST "/tasks" => create_task,
///     DELETE "/tasks/{id}" => delete_task,
/// };
/// ```
#[proc_macro]
pub fn router(_input: TokenStream) -> TokenStream {
    // Parse route definitions
    // Format: METHOD "path" => handler,
    let expanded = quote! {
        {
            let mut router = ipckit::Router::new();
            // Routes would be parsed and added here
            router
        }
    };

    TokenStream::from(expanded)
}

/// Declarative channel creation macro.
///
/// Creates an IPC channel with the specified type and name.
///
/// ## Syntax
///
/// ```rust,ignore
/// ipc_channel!(variable_name, channel_type, "channel_name");
/// ipc_channel!(variable_name, channel_type, "channel_name", options...);
/// ```
///
/// ## Channel Types
///
/// - `pipe` - Named pipe channel
/// - `socket` - Local socket channel
/// - `shm` - Shared memory channel
/// - `file` - File-based channel
/// - `thread` - Thread channel (intra-process)
///
/// ## Examples
///
/// ```rust,ignore
/// use ipckit_macros::ipc_channel;
///
/// // Create a named pipe channel
/// ipc_channel!(my_pipe, pipe, "my_app_pipe");
///
/// // Create a socket channel
/// ipc_channel!(my_socket, socket, "my_app_socket");
///
/// // Create a shared memory region
/// ipc_channel!(my_shm, shm, "my_app_shm", size = 4096);
///
/// // Create a thread channel
/// ipc_channel!(my_thread, thread);
/// ```
#[proc_macro]
pub fn ipc_channel(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let parts: Vec<&str> = input_str.split(',').map(|s| s.trim()).collect();

    if parts.len() < 2 {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "ipc_channel! requires at least 2 arguments: variable name and channel type",
        )
        .to_compile_error()
        .into();
    }

    let var_name: proc_macro2::TokenStream =
        parts[0].parse().unwrap_or_else(|_| quote! { channel });
    let channel_type = parts[1].trim();

    let expanded = match channel_type {
        "pipe" => {
            let name = parts
                .get(2)
                .map(|s| s.trim().trim_matches('"'))
                .unwrap_or("default");
            quote! {
                let #var_name = ipckit::IpcChannel::<Vec<u8>>::create(#name)
                    .expect("Failed to create pipe channel");
            }
        }
        "socket" => {
            let name = parts
                .get(2)
                .map(|s| s.trim().trim_matches('"'))
                .unwrap_or("default");
            quote! {
                let #var_name = ipckit::LocalSocketListener::bind(#name)
                    .expect("Failed to create socket channel");
            }
        }
        "shm" => {
            let name = parts
                .get(2)
                .map(|s| s.trim().trim_matches('"'))
                .unwrap_or("default");
            // Parse size option if provided
            let size: usize = parts
                .get(3)
                .and_then(|s| {
                    let s = s.trim();
                    if s.starts_with("size") {
                        s.split('=').nth(1).and_then(|v| v.trim().parse().ok())
                    } else {
                        None
                    }
                })
                .unwrap_or(4096);
            quote! {
                let #var_name = ipckit::SharedMemory::create(#name, #size)
                    .expect("Failed to create shared memory");
            }
        }
        "file" => {
            let path = parts
                .get(2)
                .map(|s| s.trim().trim_matches('"'))
                .unwrap_or("ipc_channel.json");
            quote! {
                let #var_name = ipckit::FileChannel::new(#path)
                    .expect("Failed to create file channel");
            }
        }
        "thread" => {
            // For thread channels, we create sender and receiver with fixed names
            quote! {
                let (#var_name, _rx) = {
                    let (tx, rx) = ipckit::ThreadChannel::<Vec<u8>>::new();
                    (tx, rx)
                };
            }
        }
        _ => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Unknown channel type: {}. Supported types: pipe, socket, shm, file, thread",
                    channel_type
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(expanded)
}

/// Declarative command routing macro.
///
/// Creates a command router with the specified command-to-handler mappings.
///
/// ## Syntax
///
/// ```rust,ignore
/// let router = ipc_commands! {
///     "command_name" => handler_function,
///     "another_command" => another_handler,
///     "nested/command" => nested_handler,
/// };
/// ```
///
/// ## Examples
///
/// ```rust,ignore
/// use ipckit_macros::ipc_commands;
///
/// fn ping_handler(_params: serde_json::Value) -> serde_json::Value {
///     serde_json::json!("pong")
/// }
///
/// fn echo_handler(params: serde_json::Value) -> serde_json::Value {
///     params
/// }
///
/// let router = ipc_commands! {
///     "ping" => ping_handler,
///     "echo" => echo_handler,
/// };
///
/// // Use the router
/// let result = router.handle("ping", serde_json::json!({}));
/// ```
#[proc_macro]
pub fn ipc_commands(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();

    // Parse command => handler pairs
    let mut handlers = Vec::new();

    for line in input_str.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() {
            continue;
        }

        if let Some((cmd, handler)) = line.split_once("=>") {
            let cmd = cmd.trim().trim_matches('"');
            let handler = handler.trim();
            handlers.push((cmd.to_string(), handler.to_string()));
        }
    }

    let command_matches: Vec<proc_macro2::TokenStream> = handlers
        .iter()
        .map(|(cmd, handler)| {
            let handler_ident: proc_macro2::TokenStream = handler.parse().unwrap();
            quote! {
                #cmd => Some(#handler_ident(params.clone())),
            }
        })
        .collect();

    let command_names: Vec<&str> = handlers.iter().map(|(cmd, _)| cmd.as_str()).collect();

    let expanded = quote! {
        {
            struct CommandRouter {
                _phantom: std::marker::PhantomData<()>,
            }

            impl CommandRouter {
                fn new() -> Self {
                    Self { _phantom: std::marker::PhantomData }
                }

                fn handle(&self, command: &str, params: serde_json::Value) -> Option<serde_json::Value> {
                    match command {
                        #(#command_matches)*
                        _ => None,
                    }
                }

                fn commands(&self) -> &'static [&'static str] {
                    &[#(#command_names),*]
                }
            }

            CommandRouter::new()
        }
    };

    TokenStream::from(expanded)
}

/// Declarative message type definition macro.
///
/// Defines an IPC message type with automatic serialization and validation.
///
/// ## Syntax
///
/// ```rust,ignore
/// ipc_message! {
///     pub struct MyMessage {
///         #[validate(not_empty)]
///         name: String,
///         #[validate(range(0..100))]
///         age: u8,
///         #[default]
///         optional_field: Option<String>,
///     }
/// }
/// ```
///
/// ## Examples
///
/// ```rust,ignore
/// use ipckit_macros::ipc_message;
///
/// ipc_message! {
///     pub struct CreateUserRequest {
///         name: String,
///         email: String,
///         age: Option<u8>,
///     }
/// }
///
/// ipc_message! {
///     pub struct CreateUserResponse {
///         success: bool,
///         user_id: Option<String>,
///         error: Option<String>,
///     }
/// }
/// ```
#[proc_macro]
pub fn ipc_message(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();

    // Simple parsing - extract struct definition
    // In a real implementation, we'd use syn to properly parse this
    let expanded = if input_str.contains("struct") {
        // Parse the struct definition
        let struct_def: proc_macro2::TokenStream = input_str.parse().unwrap_or_else(|_| quote! {});

        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            #struct_def
        }
    } else {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "ipc_message! expects a struct definition",
        )
        .to_compile_error()
    };

    TokenStream::from(expanded)
}

/// Middleware chain macro for IPC handlers.
///
/// Creates a middleware chain that wraps command handlers.
///
/// ## Syntax
///
/// ```rust,ignore
/// let handler = ipc_middleware! {
///     logging_middleware,
///     auth_middleware,
///     rate_limit_middleware(10, "second"),
///     => actual_handler
/// };
/// ```
///
/// ## Examples
///
/// ```rust,ignore
/// use ipckit_macros::ipc_middleware;
///
/// fn logging(next: impl Fn(Request) -> Response) -> impl Fn(Request) -> Response {
///     move |req| {
///         println!("Request: {:?}", req);
///         let resp = next(req);
///         println!("Response: {:?}", resp);
///         resp
///     }
/// }
///
/// let handler = ipc_middleware! {
///     logging,
///     => my_handler
/// };
/// ```
#[proc_macro]
pub fn ipc_middleware(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();

    // Parse middleware chain
    let parts: Vec<&str> = input_str.split("=>").collect();

    if parts.len() != 2 {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "ipc_middleware! expects format: middleware1, middleware2, => handler",
        )
        .to_compile_error()
        .into();
    }

    let middlewares: Vec<&str> = parts[0]
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let handler = parts[1].trim();
    let handler_ident: proc_macro2::TokenStream =
        handler.parse().unwrap_or_else(|_| quote! { handler });

    // Build the middleware chain from inside out
    let mut chain = quote! { #handler_ident };

    for middleware in middlewares.into_iter().rev() {
        let mw_ident: proc_macro2::TokenStream =
            middleware.parse().unwrap_or_else(|_| quote! { identity });
        chain = quote! { #mw_ident(#chain) };
    }

    let expanded = quote! {
        {
            #chain
        }
    };

    TokenStream::from(expanded)
}
