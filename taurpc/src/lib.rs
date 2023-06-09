use tauri::{Invoke, Runtime};

pub use serde::{Deserialize, Serialize};
pub use ts_rs::TS;

pub use taurpc_macros::{procedures, resolvers, rpc_struct};

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

/// Creates a handler that allows your RPCs to be called from the frontend with the coresponding
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
///   let _handler = taurpc::create_rpc_handler(ApiImpl.into_handler());
/// }
/// ```
pub fn create_rpc_handler<H, R>(procedures: H) -> impl Fn(Invoke<R>) + Send + Sync + 'static
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
