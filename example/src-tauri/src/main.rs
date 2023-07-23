#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Manager, Runtime};
use taurpc::Windows;
use tokio::{sync::oneshot, time::sleep};

#[taurpc::ipc_type]
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

// #[taurpc::procedures(event_trigger = ApiEventTrigger, export_to = "../bindings.ts")]
#[taurpc::procedures(event_trigger = ApiEventTrigger)]
trait Api {
    async fn update_state(new_value: String);

    async fn get_window<R: Runtime>(window: tauri::Window<R>);

    async fn get_app_handle<R: Runtime>(app_handle: tauri::AppHandle<R>);

    async fn test_io(user: User, arg: u32) -> User;

    async fn test_option() -> Option<()>;

    async fn test_result(user: User) -> Result<User, Error>;

    // #[taurpc(skip)]
    async fn with_sleep();

    #[taurpc(alias = "method_with_alias")]
    async fn with_alias();
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

    async fn test_io(self, user: User, arg: u32) -> User {
        user
    }

    async fn test_option(self) -> Option<()> {
        Some(())
    }

    async fn test_result(self, user: User) -> Result<User, Error> {
        Err(Error::Other("Some error message".to_string()))
        // Ok(user)
    }

    async fn with_sleep(self) {
        sleep(Duration::from_millis(2000)).await;
    }

    async fn with_alias(self) {
        println!("method with alias called");
    }
}

type GlobalState = Arc<Mutex<String>>;

#[tokio::main]
async fn main() {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let app_handle = rx.await.unwrap();
        let trigger = ApiEventTrigger::new(app_handle);

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            trigger
                .send_to(Windows::One("main".to_string()))
                .update_state("message scoped".to_string())?;

            trigger.update_state("message".to_string())?;
            // trigger.with_alias()?;
        }

        Ok::<(), tauri::Error>(())
    });

    tauri::Builder::default()
        .invoke_handler(taurpc::create_ipc_handler(
            ApiImpl {
                state: Arc::new(Mutex::new("state".to_string())),
            }
            .into_handler(),
        ))
        .setup(|app| {
            #[cfg(debug_assertions)]
            app.get_window("main").unwrap().open_devtools();

            tx.send(app.handle()).unwrap();

            Ok(())
        })
        // .manage(Arc::new(Mutex::new(String::from("state"))))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
