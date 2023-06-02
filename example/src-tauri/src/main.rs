#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[taurpc::rpc_struct]
struct User {
    user_id: i32,
    first_name: String,
    last_name: String,
    test: Vec<String>,
}

#[taurpc::procedures]
trait Api {
    fn test();

    fn test_event(input1: String, user: u8) -> String;
}

#[derive(Clone)]
struct ApiImpl;

impl Api for ApiImpl {
    fn test(self) {
        println!("called `test`");
    }

    fn test_event(self, input1: String, user: u8) -> String {
        println!("called `test_event` {}, {}", input1, user);
        input1
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(ApiImpl.into_handler()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
