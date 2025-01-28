//! This crate provides a typesafe IPC layer for Tauri's commands and events.
//! TauRPC should be used instead of [Tauri's IPC system](https://tauri.app/v1/references/architecture/inter-process-communication/),
//! which does not provide TypeScript types for your commands or events.
//!
//! Go the the [GitHub](https://github.com/MatsDK/TauRPC/#readme) page to get started.

pub extern crate serde;
pub extern crate specta;
pub extern crate specta_macros;
pub use specta_typescript::Typescript;

use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio::sync::broadcast::Sender;

use serde::Serialize;
use tauri::ipc::{Invoke, InvokeError};
use tauri::{AppHandle, Emitter, Runtime};

pub use taurpc_macros::{ipc_type, procedures, resolvers};

mod export;
use export::export_types;

/// A trait, which is automatically implemented by `#[taurpc::procedures]`, that is used for handling incoming requests
/// and the type generation.
pub trait TauRpcHandler<R: Runtime>: Sized {
    const TRAIT_NAME: &'static str;

    /// This handler's prefix in the TypeScript router.
    const PATH_PREFIX: &'static str;

    /// Bindings export path optionally specified by the user.
    const EXPORT_PATH: Option<&'static str>;

    /// Handle a single incoming request
    fn handle_incoming_request(self, invoke: Invoke<R>);

    /// Spawn a new `tokio` thread that listens for and handles incoming request through a `tokio::broadcast::channel`.
    /// This is used for when you have multiple handlers inside a router.
    fn spawn(self) -> Sender<Arc<Invoke<R>>>;

    /// Returns a json object containing the arguments for the methods.
    /// This is used on the frontend to ensure the arguments are send with their correct idents to the backend.
    fn args_map() -> String;
}

/// Creates a handler that allows your IPCs to be called from the frontend with the coresponding
/// types. Accepts a struct in which your `taurpc::procedures` trait is implemented.
/// If you have nested routes, look at [taurpc::Router](https://docs.rs/taurpc/latest/taurpc/struct.Router.html).
///
///
///  # Examples
/// ```rust
/// #[taurpc::procedures]
/// trait Api {
///     async fn hello_world();
/// }
///
/// #[derive(Clone)]
/// struct ApiImpl;
/// #[taurpc::resolvers]
/// impl Api for ApiImpl {
///     async fn hello_world(self) {
///         println!("Hello world");
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///   tauri::Builder::default()
///     .invoke_handler(
///       taurpc::create_ipc_handler(ApiImpl.into_handler());
///     )
///     .run(tauri::generate_context!())
///     .expect("error while running tauri application");
/// }
/// ```
pub fn create_ipc_handler<H, R: Runtime>(
    procedures: H,
) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static
where
    H: TauRpcHandler<R> + Send + Sync + 'static + Clone,
{
    let args_map = HashMap::from([(H::PATH_PREFIX.to_string(), H::args_map())]);
    #[cfg(debug_assertions)] // Only export in development builds
    export_types(
        H::EXPORT_PATH,
        vec![(H::PATH_PREFIX, H::TRAIT_NAME)],
        args_map,
        specta_typescript::Typescript::default(),
    );

    move |invoke: Invoke<R>| {
        procedures.clone().handle_incoming_request(invoke);
        true
    }
}

#[derive(Serialize, Clone)]
struct Event<S> {
    event: S,
    event_name: String,
}

/// Enum used for triggering scoped events instead of on all windows.
/// Use the `send_to(scope: Windows)` method on your event trigger struct.
#[derive(Default, Debug, Clone)]
pub enum Windows {
    #[default]
    All,
    One(String),
    N(Vec<String>),
}

/// A structure used for triggering [tauri events](https://tauri.app/v1/guides/features/events/) on the frontend.
/// By default the events are send to all windows with `emit_all`, if you want to send to a specific window by label,
/// use `new_scoped` or `new_scoped_from_trigger`.
#[derive(Debug)]
pub struct EventTrigger<RT: Runtime> {
    app_handle: AppHandle<RT>,
    path_prefix: String,
    scope: Windows,
}

impl<RT: Runtime> Clone for EventTrigger<RT> {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            path_prefix: self.path_prefix.clone(),
            scope: self.scope.clone(),
        }
    }
}

impl<RT: Runtime> EventTrigger<RT> {
    pub fn new(app_handle: AppHandle<RT>, path_prefix: String) -> Self {
        Self {
            app_handle,
            path_prefix,
            scope: Default::default(),
        }
    }

    pub fn new_scoped(app_handle: AppHandle<RT>, path_prefix: String, scope: Windows) -> Self {
        Self {
            app_handle,
            path_prefix,
            scope,
        }
    }

    pub fn new_scoped_from_trigger(trigger: Self, scope: Windows) -> Self {
        Self {
            app_handle: trigger.app_handle,
            path_prefix: trigger.path_prefix,
            scope,
        }
    }

    pub fn call<S: Serialize + Clone>(&self, proc_name: &str, event: S) -> tauri::Result<()> {
        let event_name = if self.path_prefix.is_empty() {
            proc_name.to_string()
        } else {
            format!("{}.{}", self.path_prefix, proc_name)
        };
        let event = Event { event_name, event };
        match &self.scope {
            Windows::All => self.app_handle.emit("TauRpc_event", event),
            Windows::One(label) => self.app_handle.emit_to(label, "TauRpc_event", event),
            Windows::N(labels) => {
                for label in labels {
                    self.app_handle
                        .emit_to(label, "TauRpc_event", event.clone())?;
                }
                Ok(())
            }
        }
    }
}

/// Used for merging nested trait implementations. This is used when you have multiple trait implementations,
/// instead of `taurpc::create_ipc_handler()`. Use `.merge()` to add trait implementations to the router.
/// The trait must be have the `#[taurpc::procedures]` and the nested routes should have `#[taurpc::procedures(path = "path")]`.
///
///  # Examples
/// ```rust
/// #[taurpc::procedures]
/// trait Api { }
///
/// #[derive(Clone)]
/// struct ApiImpl;
///
/// #[taurpc::resolveres]
/// impl Api for ApiImpl { }
///
/// #[taurpc::procedures(path = "events")]
/// trait Events { }
///
/// #[derive(Clone)]
/// struct EventsImpl;
///
/// #[taurpc::resolveres]
/// impl Events for EventsImpl { }
///
/// #[tokio::main]
/// async fn main() {
///   let router = Router::new()
///     .merge(ApiImpl.into_handler())
///     .merge(EventsImpl.into_handler());
///
///   tauri::Builder::default()
///     .invoke_handler(router.into_handler())
///     .run(tauri::generate_context!())
///     .expect("error while running tauri application");
/// }
/// ```
#[derive(Default)]
pub struct Router<R: Runtime> {
    handlers: HashMap<String, Sender<Arc<Invoke<R>>>>,
    export_path: Option<&'static str>,
    args_map_json: HashMap<String, String>,
    handler_paths: Vec<(&'static str, &'static str)>,
    export_config: specta_typescript::Typescript,
}

impl<R: Runtime> Router<R> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            export_path: None,
            args_map_json: HashMap::new(),
            handler_paths: Vec::new(),
            export_config: specta_typescript::Typescript::default(),
        }
    }

    /// Overwrite `specta` default TypeScript export options, look at the docs for
    /// `specta_typescript::Typescript` for all the configuration options.
    ///
    /// Example:
    /// ```rust
    ///    let router = Router::new()
    ///        .export_config(
    ///            specta_typescript::Typescript::default()
    ///                .header("// My header\n")
    ///                .bigint(specta_typescript::BigIntExportBehavior::String),
    ///        )
    ///        .merge(...);
    /// ```
    pub fn export_config(mut self, config: specta_typescript::Typescript) -> Self {
        self.export_config = config;
        self
    }

    /// Add routes to the router, accepts a struct for which a `#[taurpc::procedures]` trait is implemented
    ///
    /// ```rust
    ///    let router = Router::new()
    ///      .merge(ApiImpl.into_handler())
    ///      .merge(EventsImpl.into_handler());
    /// ```
    pub fn merge<H: TauRpcHandler<R>>(mut self, handler: H) -> Self {
        if let Some(path) = H::EXPORT_PATH {
            self.export_path = Some(path)
        }

        self.handler_paths.push((H::PATH_PREFIX, H::TRAIT_NAME));
        self.args_map_json
            .insert(H::PATH_PREFIX.to_string(), H::args_map());
        self.handlers
            .insert(H::PATH_PREFIX.to_string(), handler.spawn());
        self
    }

    /// Create a handler out of the router that allows your IPCs to be called from the frontend,
    /// and generate the corresponding types. Use this inside `.invoke_handler()` on the tauri::Builder.
    ///
    /// ```rust
    ///    tauri::Builder::default()
    ///      .invoke_handler(router.into_handler())
    ///      .run(tauri::generate_context!())
    ///      .expect("error while running tauri application");
    /// ```
    pub fn into_handler(self) -> impl Fn(Invoke<R>) -> bool {
        #[cfg(debug_assertions)] // Only export in development builds
        export_types(
            self.export_path.clone(),
            self.handler_paths.clone(),
            self.args_map_json.clone(),
            self.export_config.clone(),
        );

        move |invoke: Invoke<R>| self.on_command(invoke)
    }

    fn on_command(&self, invoke: Invoke<R>) -> bool {
        let cmd = invoke.message.command();
        if !cmd.starts_with("TauRPC__") {
            return false;
        }

        // Remove `TauRPC__`
        let prefix = cmd[8..].to_string();
        let mut prefix = prefix.split('.').collect::<Vec<_>>();
        // Remove the actual name of the command
        prefix.pop().unwrap();

        match self.handlers.get(&prefix.join(".")) {
            Some(handler) => {
                let _ = handler.send(Arc::new(invoke));
            }
            None => invoke
                .resolver
                .invoke_error(InvokeError::from(format!("`{cmd}` not found"))),
        };

        true
    }
}
