//! This crate provides a typesafe IPC layer for Tauri's commands and events.
//! TauRPC should be used instead of [Tauri's IPC system](https://tauri.app/v1/references/architecture/inter-process-communication/),
//! which does not provide TypeScript types for your commands or events.
//!
//! Go the the [GitHub](https://github.com/MatsDK/TauRPC/#readme) page to get started.

pub extern crate serde;
pub extern crate specta;

pub use taurpc_macros::{ipc_type, procedures, resolvers};

mod utils;
pub use utils::export_files;

use serde::Serialize;
use tauri::{AppHandle, Invoke, Manager, Runtime};

/// A trait, which is automatically implemented by `#[taurpc::procedures]`, that is used for handling incoming requests
/// and the type generation.
pub trait TauRpcHandler<R: Runtime> {
    /// Response types enum
    type Resp: Serialize;

    /// Handle a single incoming request
    fn handle_incoming_request(self, invoke: Invoke<R>);

    /// Generates and exports TS types on runtime.
    fn generate_ts_types();

    /// Returns a json object containing the arguments for the methods.
    /// This is used on the frontend to ensure the arguments are send with their correct idents to the backend.
    fn setup() -> String;
}

/// Creates a handler that allows your IPCs to be called from the frontend with the coresponding
/// types. Accepts a struct in which your `taurpc::procedures` trait is implemented.
///
///  # Examples
/// ```rust
/// #[taurpc::procedures]
/// trait Api {
///     fn hello_world();
/// }
///
/// #[derive(Clone)]
/// struct ApiImpl;
/// impl Api for ApiImpl {
///     fn hello_world(self) {
///         println!("Hello world");
///     }
/// }
///
/// fn main() {
///   let _handler = taurpc::create_ipc_handler(ApiImpl.into_handler());
/// }
/// ```
pub fn create_ipc_handler<H, R>(procedures: H) -> impl Fn(Invoke<R>) + Send + Sync + 'static
where
    H: TauRpcHandler<R> + Send + Sync + 'static + Clone,
    R: Runtime,
{
    H::generate_ts_types();

    move |invoke: Invoke<R>| {
        let cmd = invoke.message.command();

        match cmd {
            "TauRPC__setup" => invoke.resolver.respond(Ok(H::setup())),
            _ => procedures.clone().handle_incoming_request(invoke),
        }
    }
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
    scope: Windows,
}

impl EventTrigger {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            scope: Default::default(),
        }
    }

    pub fn new_scoped(app_handle: AppHandle, scope: Windows) -> Self {
        Self { app_handle, scope }
    }

    pub fn new_scoped_from_trigger(trigger: Self, scope: Windows) -> Self {
        Self {
            app_handle: trigger.app_handle,
            scope,
        }
    }

    pub fn call<S: Serialize + Clone>(&self, req: S) -> tauri::Result<()> {
        match &self.scope {
            Windows::All => self.app_handle.emit_all("TauRpc_event", req),
            Windows::One(label) => self.app_handle.emit_to(&label, "TauRpc_event", req),
            Windows::N(labels) => {
                for label in labels {
                    self.app_handle
                        .emit_to(&label, "TauRpc_event", req.clone())?;
                }
                Ok(())
            }
        }
    }
}
