// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{sync::Arc, time::Duration};
use tauri::{ipc::Channel, AppHandle, EventTarget, Manager, Runtime, WebviewWindow, Window};
use taurpc::Router;
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
#[derive(Debug, thiserror::Error, specta::Type)]
#[serde(tag = "type", content = "data")]
enum Error {
    #[error(transparent)]
    Io(
        #[from]
        #[serde(skip)]
        std::io::Error,
    ),

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

#[taurpc::ipc_type]
struct Update {
    progress: u8,
}

// #[taurpc::procedures(event_trigger = ApiEventTrigger)]
#[taurpc::procedures(event_trigger = ApiEventTrigger, export_to = "../src/lib/bindings.ts")]
trait Api {
    async fn update_state(app_handle: AppHandle<impl Runtime>, new_value: String);

    async fn get_window<R: Runtime>(window: Window<R>);
    // async fn get_window<R: Runtime>(#[window] win: Window<R>);

    async fn get_webview_window<R: Runtime>(webview_window: WebviewWindow<R>);

    async fn get_app_handle<R: Runtime>(app_handle: AppHandle<R>);
    // async fn get_app_handle(#[app_handle] ah: AppHandle<impl Runtime>);

    async fn test_io(_user: User) -> User;

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

    async fn with_channel(on_event: Channel<Update>);
}

#[derive(Clone)]
struct ApiImpl {
    state: GlobalState,
}

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn update_state(self, app_handle: AppHandle<impl Runtime>, new_value: String) {
        let mut data = self.state.lock().await;
        println!("Before {:?}", data);
        *data = new_value;
        println!("After {:?}", data);

        let uppercase = data.to_uppercase();

        TauRpcEventsEventTrigger::new(app_handle)
            .state_changed(uppercase)
            .unwrap();
    }

    async fn get_window<R: Runtime>(self, window: Window<R>) {
        println!("Window: {}", window.label());
    }

    async fn get_webview_window<R: Runtime>(self, webview_window: WebviewWindow<R>) {
        println!("WebviewWindow: {}", webview_window.label());
    }

    async fn get_app_handle<R: Runtime>(self, app_handle: AppHandle<R>) {
        println!(
            "App Handle: {:?}, {:?}",
            app_handle.path().app_config_dir(),
            app_handle.package_info()
        );
    }

    async fn test_io(self, user: User) -> User {
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

    async fn vec_test(self, _arg: Vec<String>) {}

    async fn multiple_args(self, _arg: Vec<String>, _arg2: String) {}

    async fn test_bigint(self, num: i64) -> i64 {
        num
    }

    async fn with_channel(self, on_event: Channel<Update>) {
        for progress in [15, 20, 35, 50, 90] {
            on_event.send(Update { progress }).unwrap();
        }
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
    let (tx, rx) = oneshot::channel::<AppHandle>();

    tokio::spawn(async move {
        let app_handle = rx.await.unwrap();
        let events_trigger = TauRpcEventsEventTrigger::new(app_handle.clone());
        let ui_trigger = TauRpcUiApiEventTrigger::new(app_handle);

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            events_trigger.vec_test(vec![String::from("test"), String::from("test2")])?;

            events_trigger
                .send_to(EventTarget::Any)
                .vec_test(vec![String::from("test"), String::from("test2")])?;

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
                .formatter(specta_typescript::formatter::prettier)
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

    // Without router
    // tauri::Builder::default()
    //     .invoke_handler(router.into_handler())
    //     // .invoke_handler(taurpc::create_ipc_handler(
    //     //     ApiImpl {
    //     //         state: Arc::new(Mutex::new("state".to_string())),
    //     //     }
    //     //     .into_handler(),
    //     // ))
    //     .setup(|app| {
    //         tx.send(app.handle().clone()).unwrap();
    //         Ok(())
    //     })
    //     .run(tauri::generate_context!())
    //     .expect("error while running tauri application");
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(router.into_handler())
        .setup(|app| {
            #[cfg(debug_assertions)]
            app.get_webview_window("main").unwrap().open_devtools();

            tx.send(app.handle().clone()).unwrap();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
