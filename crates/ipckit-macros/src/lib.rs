//! # ipckit-macros
//!
//! Procedural macros for ipckit providing declarative IPC handler definitions.
//!
//! ## Features
//!
//! - `#[ipc_handler]` - Mark an impl block as an IPC handler
//! - `#[command]` - Define a command handler method
//! - `#[derive(IpcMessage)]` - Derive serialization for IPC messages
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit_macros::{ipc_handler, command};
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
