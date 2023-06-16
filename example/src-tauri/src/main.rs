#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use tauri::{Manager, Runtime};

#[taurpc::rpc_struct]
struct User {
    user_id: i32,
    first_name: String,
    last_name: String,
    test: Vec<String>,
}

#[taurpc::procedures]
trait Api {
    fn test_state(input: String, state: tauri::State<GlobalState>);

    fn test_window<R: Runtime>(window: tauri::Window<R>);

    fn test_app_handl<R: Runtime>(app_handle: tauri::AppHandle<R>);

    fn test_event(input1: String, user: u8) -> Option<User>;
}

#[derive(Clone)]
struct ApiImpl;

impl Api for ApiImpl {
    fn test_state(self, input: String, state: tauri::State<GlobalState>) {
        let mut data = state.lock().unwrap();
        println!("{:?}", data);
        *data = input;
        println!("called `test`");
    }

    fn test_window<R: Runtime>(self, window: tauri::Window<R>) {
        println!("{}", window.label());
    }

    fn test_app_handl<R: Runtime>(self, app_handle: tauri::AppHandle<R>) {
        let app_dir = app_handle.path_resolver().app_config_dir();
        println!("{:?}, {:?}", app_dir, app_handle.package_info());
    }

    fn test_event(self, input1: String, user: u8) -> Option<User> {
        println!("called `test_event` {}, {}", input1, user);
        Some(User {
            first_name: input1.clone(),
            last_name: input1,
            test: vec![],
            user_id: 0,
        })
    }
}

type GlobalState = Arc<Mutex<String>>;

fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(ApiImpl.into_handler()))
        .setup(|app| {
            #[cfg(debug_assertions)]
            app.get_window("main").unwrap().open_devtools();
            Ok(())
        })
        .manage(Arc::new(Mutex::new(String::from("default value"))))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
