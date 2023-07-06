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

// create the error type that represents all errors possible in our program
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Other: `{0}`")]
    Other(String),
}

// we must manually implement serde::Serialize
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[taurpc::procedures]
trait Api {
    async fn update_state(new_value: String);

    async fn get_window<R: Runtime>(window: tauri::Window<R>);

    async fn get_app_handle<R: Runtime>(app_handle: tauri::AppHandle<R>);

    async fn test_io(user: User) -> User;

    async fn test_option() -> Option<()>;

    async fn test_result(user: User) -> Result<User, Error>;

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
        app_handle
            .emit_all("test_event", String::from("Some payload value"))
            .unwrap();
        let app_dir = app_handle.path_resolver().app_config_dir();
        println!("{:?}, {:?}", app_dir, app_handle.package_info());
    }

    async fn test_io(self, user: User) -> User {
        user
    }

    async fn test_option(self) -> Option<()> {
        Some(())
    }

    async fn test_result(self, user: User) -> Result<User, Error> {
        // Err(Error::Other("Some error message".to_string()))
        Ok(user)
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
        // .manage(Arc::new(Mutex::new(String::from("state"))))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
