//! This crate provides a typesafe IPC layer for Tauri's commands and events.
//! TauRPC should be used instead of [Tauri's IPC system](https://tauri.app/v1/references/architecture/inter-process-communication/),
//! which does not provide TypeScript types for your commands or events.
//!
//! Go the the [GitHub](https://github.com/MatsDK/TauRPC/#readme) page to get started.

pub extern crate serde;
pub extern crate specta;

use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio::sync::broadcast::Sender;

pub use taurpc_macros::{ipc_type, procedures, resolvers};

mod export;
use export::export_files;

use serde::Serialize;
use tauri::{AppHandle, Invoke, InvokeError, Manager, Runtime};

/// A trait, which is automatically implemented by `#[taurpc::procedures]`, that is used for handling incoming requests
/// and the type generation.
pub trait TauRpcHandler<R: Runtime>: Sized {
    /// Handle a single incoming request
    fn handle_incoming_request(self, invoke: Invoke<R>);

    /// Handle a single incoming request
    fn spawn(self) -> Sender<Arc<Invoke<tauri::Wry>>>;

    /// Get info about that handler that is necessary for generating and exporthing the types on runtime.
    /// Returns (trait_name, trait_path_prefix, export_path)
    fn handler_info() -> (String, String, Option<String>);

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
pub fn create_ipc_handler<H>(procedures: H) -> impl Fn(Invoke<tauri::Wry>) + Send + Sync + 'static
where
    H: TauRpcHandler<tauri::Wry> + Send + Sync + 'static + Clone,
{
    let (trait_name, path_prefix, export_path) = H::handler_info();
    export_files(export_path, vec![(path_prefix, trait_name)]);

    move |invoke: Invoke<tauri::Wry>| {
        let cmd = invoke.message.command();

        match cmd {
            "TauRPC__setup" => invoke.resolver.respond(Ok(H::args_map())),
            _ => procedures.clone().handle_incoming_request(invoke),
        }
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
#[derive(Debug, Clone)]
pub struct EventTrigger {
    app_handle: AppHandle,
    path_prefix: String,
    scope: Windows,
}

impl EventTrigger {
    pub fn new(app_handle: AppHandle, path_prefix: String) -> Self {
        Self {
            app_handle,
            path_prefix,
            scope: Default::default(),
        }
    }

    pub fn new_scoped(app_handle: AppHandle, path_prefix: String, scope: Windows) -> Self {
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
        let event_name = if self.path_prefix.len() == 0 {
            proc_name.to_string()
        } else {
            format!("{}.{}", self.path_prefix, proc_name)
        };
        let event = Event { event_name, event };
        match &self.scope {
            Windows::All => self.app_handle.emit_all("TauRpc_event", event),
            Windows::One(label) => self.app_handle.emit_to(&label, "TauRpc_event", event),
            Windows::N(labels) => {
                for label in labels {
                    self.app_handle
                        .emit_to(&label, "TauRpc_event", event.clone())?;
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
pub struct Router {
    handlers: HashMap<String, Sender<Arc<Invoke<tauri::Wry>>>>,
    export_path: Option<String>,
    args_map_json: HashMap<String, String>,
    handler_paths: Vec<(String, String)>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            handlers: Default::default(),
            args_map_json: Default::default(),
            export_path: None,
            handler_paths: vec![],
        }
    }

    /// Add routes to the router, accepts a struct for which a `#[taurpc::procedures]` trait is implemented
    ///
    /// ```rust
    ///   let router = Router::new()
    ///     .merge(ApiImpl.into_handler())
    ///     .merge(EventsImpl.into_handler());
    /// ```
    pub fn merge<H: TauRpcHandler<tauri::Wry>>(mut self, handler: H) -> Self {
        let (trait_name, path_prefix, export_path) = H::handler_info();
        if let Some(path) = export_path {
            self.export_path = Some(path)
        }

        self.handler_paths.push((path_prefix.clone(), trait_name));
        self.args_map_json
            .insert(path_prefix.clone(), H::args_map());
        self.handlers.insert(path_prefix, handler.spawn());
        self
    }

    /// Create a handler out of the router that allows your IPCs to be called from the frontend,
    /// and generate the corresponding types. Use this inside `.invoke_handler()` on the tauri::Builder.
    ///
    /// ```rust
    ///   tauri::Builder::default()
    ///     .invoke_handler(router.into_handler())
    ///     .run(tauri::generate_context!())
    ///     .expect("error while running tauri application");
    /// ```
    pub fn into_handler(self) -> impl Fn(Invoke<tauri::Wry>) {
        export_files(self.export_path.clone(), self.handler_paths.clone());

        move |invoke: Invoke<tauri::Wry>| {
            let cmd = invoke.message.command();

            match cmd {
                "TauRPC__setup" => {
                    let map = serde_json::to_string(&self.args_map_json).unwrap();
                    invoke.resolver.respond(Ok(map))
                }
                _ => self.on_command(invoke),
            }
        }
    }

    fn on_command(&self, invoke: Invoke<tauri::Wry>) {
        let cmd = invoke.message.command();
        if !cmd.starts_with("TauRPC__") {
            return;
        }

        // Remove `TauRPC__`
        let prefix = cmd[8..].to_string();
        let mut prefix = prefix.split(".").collect::<Vec<_>>();
        // Remove the actual name of the command
        prefix.pop().unwrap();

        match self.handlers.get(&prefix.join(".")) {
            Some(handler) => {
                handler.send(Arc::new(invoke)).unwrap();
            }
            None => invoke
                .resolver
                .invoke_error(InvokeError::from(format!("`{cmd}` not found"))),
        };
    }
}
