#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Manager, Runtime};
use tokio::time::sleep;

#[taurpc::rpc_struct]
struct User {
    uid: i32,
    first_name: String,
    last_name: String,
}
#[taurpc::procedures]
trait Api {
    async fn update_state(new_value: String);

    async fn get_window<R: Runtime>(window: tauri::Window<R>);

    async fn get_app_handle<R: Runtime>(app_handle: tauri::AppHandle<R>);

    async fn test_io(user: User) -> Option<User>;

    async fn with_sleep();
}

#[derive(Clone)]
struct ApiImpl {
    state: GlobalState,
}

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn update_state(self, new_value: String) {
        let mut data = self.state.lock().unwrap();
        println!("Before {:?}", data);
        *data = new_value;
        println!("After {:?}", data);
    }

    async fn get_window<R: Runtime>(self, window: tauri::Window<R>) {
        println!("{}", window.label());
    }

    async fn get_app_handle<R: Runtime>(self, app_handle: tauri::AppHandle<R>) {
        let app_dir = app_handle.path_resolver().app_config_dir();
        println!("{:?}, {:?}", app_dir, app_handle.package_info());
    }

    async fn test_io(self, user: User) -> Option<User> {
        Some(user)
    }

    async fn with_sleep(self) {
        sleep(Duration::from_millis(2000)).await;
    }
}

type GlobalState = Arc<Mutex<String>>;

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(
            ApiImpl {
                state: Arc::new(Mutex::new("some state value".to_string())),
            }
            .into_handler(),
        ))
        .setup(|app| {
            #[cfg(debug_assertions)]
            app.get_window("main").unwrap().open_devtools();
            Ok(())
        })
        .manage(Mutex::new("some state value".to_string()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
