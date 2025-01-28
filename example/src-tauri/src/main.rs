// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::Arc, time::Duration};
use tauri::{AppHandle, Manager, Runtime};
use taurpc::{Router, Windows};
use tokio::{
    sync::{oneshot, Mutex},
    time::sleep,
};

#[doc = "Doc comments are also generated"]
#[taurpc::ipc_type]
// #[derive(serde::Serialize, serde::Deserialize, specta::Type, Clone)]
struct User {
    /// The user's id
    uid: i32,
    /// The user's first name
    first_name: String,
    /// The user's last name
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

// #[taurpc::procedures(event_trigger = ApiEventTrigger)]
#[taurpc::procedures(event_trigger = ApiEventTrigger, export_to = "../src/lib/bindings.ts")]
trait Api {
    async fn update_state<R: Runtime>(app_handle: AppHandle<R>, new_value: String);

    async fn get_window<R: Runtime>(window: tauri::Window<R>);

    async fn get_app_handle<R: Runtime>(app_handle: tauri::AppHandle<R>);

    async fn test_io(user: User) -> User;

    async fn test_option() -> Option<()>;

    async fn test_result(user: User) -> Result<User, Error>;

    // #[taurpc(skip)]
    async fn with_sleep();

    #[taurpc(alias = "method_with_alias")]
    async fn with_alias();

    #[taurpc(event)]
    async fn ev(updated_value: String);

    async fn vec_test(arg: Vec<String>);

    async fn multiple_args(arg: Vec<String>, arg2: String);

    async fn test_bigint(num: i64) -> i64;
}

#[derive(Clone)]
struct ApiImpl {
    state: GlobalState,
}

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn update_state<R: Runtime>(self, app_handle: AppHandle<R>, new_value: String) {
        let mut data = self.state.lock().await;
        println!("Before {:?}", data);
        *data = new_value;
        println!("After {:?}", data);

        let uppercase = data.to_uppercase();

        TauRpcEventsEventTrigger::new(app_handle)
            .state_changed(uppercase)
            .unwrap();
    }

    async fn get_window<R: Runtime>(self, window: tauri::Window<R>) {
        println!("Window: {}", window.label());
    }

    async fn get_app_handle<R: Runtime>(self, app_handle: tauri::AppHandle<R>) {
        let app_dir = app_handle.path().app_config_dir();
        println!("App Handle: {:?}, {:?}", app_dir, app_handle.package_info());
    }

    async fn test_io(self, user: User) -> User {
        user
    }

    async fn test_option(self) -> Option<()> {
        Some(())
    }

    async fn test_result(self, _user: User) -> Result<User, Error> {
        Err(Error::Other("Some error message".to_string()))
        // Ok(user)
    }

    async fn with_sleep(self) {
        sleep(Duration::from_millis(2000)).await;
    }

    async fn with_alias(self) {
        println!("method with alias called");
    }

    async fn vec_test(self, _arg: Vec<String>) {}

    async fn multiple_args(self, _arg: Vec<String>, _arg2: String) {}

    async fn test_bigint(self, num: i64) -> i64 {
        num
    }
}

#[taurpc::procedures(path = "events", export_to = "../src/lib/bindings.ts")]
trait Events {
    #[taurpc(event)]
    async fn test_ev();

    #[taurpc(event)]
    async fn state_changed(new_state: String);

    #[taurpc(event)]
    async fn vec_test(args: Vec<String>);

    #[taurpc(event)]
    async fn multiple_args(arg1: u16, arg2: Vec<String>);
}

#[derive(Clone)]
struct EventsImpl;

#[taurpc::resolvers]
impl Events for EventsImpl {}

#[taurpc::procedures(path = "api.ui", export_to = "../src/lib/bindings.ts")]
trait UiApi {
    async fn trigger();

    #[taurpc(event)]
    async fn test_ev();
}

#[derive(Clone)]
struct UiApiImpl;

#[taurpc::resolvers]
impl UiApi for UiApiImpl {
    async fn trigger(self) {
        println!("Trigger ui event")
    }
}

type GlobalState = Arc<Mutex<String>>;

#[tokio::main]
async fn main() {
    run(tauri::Builder::default(), |_app| {}).await
}

async fn run<R: Runtime, F: FnOnce(&tauri::App<R>) + Send + 'static>(
    builder: tauri::Builder<R>,
    _setup: F,
) {
    let (tx, rx) = oneshot::channel::<AppHandle<R>>();

    tokio::spawn(async move {
        let app_handle = rx.await.unwrap();
        let api_trigger = ApiEventTrigger::new(app_handle.clone());
        let events_trigger = TauRpcEventsEventTrigger::new(app_handle.clone());
        let ui_trigger = TauRpcUiApiEventTrigger::new(app_handle);

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            api_trigger
                .send_to(Windows::One("main".to_string()))
                .update_state::<R>("message scoped".to_string())?;

            api_trigger.update_state::<R>("message".to_string())?;

            events_trigger.vec_test(vec![String::from("test"), String::from("test2")])?;

            events_trigger.multiple_args(0, vec![String::from("test"), String::from("test2")])?;

            events_trigger.test_ev()?;
            ui_trigger.test_ev()?;
        }

        #[allow(unreachable_code)]
        Ok::<(), tauri::Error>(())
    });

    let router = Router::new()
        .export_config(
            specta_typescript::Typescript::default()
                .header("// My header\n\n")
                // Make sure prettier is installed before using this.
                // .formatter(specta_typescript::formatter::prettier)
                .bigint(specta_typescript::BigIntExportBehavior::String),
        )
        .merge(
            ApiImpl {
                state: Arc::new(Mutex::new("state".to_string())),
            }
            .into_handler(),
        )
        .merge(EventsImpl.into_handler())
        .merge(UiApiImpl.into_handler());

    // tauri::Builder::default()
    //     .invoke_handler(router.into_handler())
    //     // .invoke_handler(taurpc::create_ipc_handler(
    //     //     ApiImpl {
    //     //         state: Arc::new(Mutex::new("state".to_string())),
    //     //     }
    //     //     .into_handler(),
    //     // ))
    //     .setup(|app| {
    //         #[cfg(debug_assertions)]
    //         app.get_window("main").unwrap().open_devtools();

    //         tx.send(app.handle().clone()).unwrap();

    //         Ok(())
    //     })
    //     .run(tauri::generate_context!())
    //     .expect("error while running tauri application");
    builder
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(router.into_handler())
        .setup(|app| {
            #[cfg(debug_assertions)]
            tx.send(app.handle().clone()).unwrap();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
