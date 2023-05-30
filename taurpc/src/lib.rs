use tauri::{Invoke, Runtime};

pub use serde::Serialize;
pub use ts_rs::TS;

pub use taurpc_macros::{procedures, rpc_struct};

pub trait TauRpcHandler {
    fn generate_ts_types();

    fn handle_incoming_request(self);
}

// #[derive(TS, Serialize)]
// #[ts(export_to = "../../types/")]
// struct User {
//     user_id: i32,
//     first_name: String,
//     last_name: String,
// }

// #[derive(Serialize, TS)]
// #[serde(tag = "procedure", content = "data")]
// #[ts(export_to = "../../types/index.ts")]
// enum ComplexEnum {
//     A((String,)),
//     B((String, u32)),
//     U { name: String },
//     V((User,)),
// }

pub fn create_rpc_handler<H, R>(procedures: H) -> impl Fn(Invoke<R>) + Send + Sync + 'static
where
    H: TauRpcHandler + Send + Sync + 'static + Clone,
    R: Runtime,
{
    H::generate_ts_types();

    move |invoke: Invoke<R>| {
        procedures.clone().handle_incoming_request();
        let cmd = invoke.message.command();
        println!("{cmd}");
    }
}
