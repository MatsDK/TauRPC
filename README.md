# TauRPC

[![](https://img.shields.io/npm/v/taurpc)](https://www.npmjs.com/package/taurpc) [![](https://img.shields.io/crates/v/taurpc)](https://crates.io/crates/taurpc) [![](https://img.shields.io/docsrs/taurpc)](https://docs.rs/taurpc/) ![](https://img.shields.io/crates/l/taurpc)

This package is a Tauri extension to give you a fully-typed IPC layer for [Tauri commands](https://v2.tauri.app/develop/calling-rust/#commands) and [events](https://v2.tauri.app/develop/calling-rust/#event-system).

The TS types corresponding to your pre-defined Rust backend API are generated on runtime, after which they can be used to call the backend from your TypeScript frontend framework of choice. This crate provides typesafe bidirectional IPC communication between the Rust backend and TypeScript frontend.
[Specta](https://github.com/oscartbeaumont/specta) is used under the hood for the type-generation. The trait-based API structure was inspired by [tarpc](https://github.com/google/tarpc).

# Usage🔧

First, add the following crates to your `Cargo.toml`:

```toml
# src-tauri/Cargo.toml

[dependencies]
taurpc = "0.5.0"

specta = { version = "=2.0.0-rc.22", features = ["derive"] }
# specta-typescript = "0.0.9"
tokio = { version = "1", features = ["full"] }
```

Then, declare and implement your IPC methods and resolvers. If you want to use your API for Tauri's events, you don't have to implement the resolvers, go to [Calling the frontend](https://github.com/MatsDK/TauRPC/#calling-the-frontend)

```rust
// src-tauri/src/main.rs

#[taurpc::procedures]
trait Api {
    async fn hello_world();
}

#[derive(Clone)]
struct ApiImpl;

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn hello_world(self) {
        println!("Hello world");
    }
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(taurpc::create_ipc_handler(ApiImpl.into_handler()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

The `#[taurpc::procedures]` trait will generate everything necessary for handling calls and the type-generation. Now, you should run `pnpm tauri dev` to generate and export the TS types.
The types will by default be exported to `bindings.ts` in the root of your project, but you can specify an export path by doing this `#[taurpc::procedures(export_to = "../src/types.ts")]`.

Then on the frontend install the taurpc package.

```bash
pnpm install taurpc
```

Now on the frontend you import the generated types, if you specified the `export_to` attribute on your procedures you should import your from there.
With these types a typesafe proxy is generated that you can use to invoke commands and listen for events.

```typescript
import { createTauRPCProxy } from '../bindings.ts'

const taurpc = createTauRPCProxy()
await taurpc.hello_world()
```

The types for taurpc are generated once you start your application, run `pnpm tauri dev`. If the types are not picked up by the LSP, you may have to restart typescript to reload the types.

You can find a complete example (using Svelte) [here](https://github.com/MatsDK/TauRPC/tree/main/example).

# Using structs

If you want to use structs for the inputs/outputs of procedures, you should always add `#[taurpc::ipc_type]` to make sure the coresponding ts types are generated. This make will derive serde `Serialize` and `Deserialize`, `Clone` and `specta::Type`.

```rust
#[taurpc::ipc_type]
// #[derive(serde::Serialize, serde::Deserialize, specta::Type, Clone)]
struct User {
    user_id: u32,
    first_name: String,
    last_name: String,
}

#[taurpc::procedures]
trait Api {
    async fn get_user() -> User;
}
```

# Accessing managed state

To share some state between procedures, you can add fields on the API implementation struct. If the state requires to be mutable, you need to use a container that enables interior mutability, like a [Mutex](https://doc.rust-lang.org/std/sync/struct.Mutex.html).

You can use the `window`, `app_handle` and `webview_window` arguments just like with Tauri's commands. [Tauri docs](https://v2.tauri.app/develop/calling-rust/#accessing-the-webviewwindow-in-commands)

```rust
// src-tauri/src/main.rs

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Runtime, State, Window};

type MyState = Arc<Mutex<String>>;

#[taurpc::procedures]
trait Api {
    async fn with_state();

    async fn with_window<R: Runtime>(window: Window<R>);
}

#[derive(Clone)]
struct ApiImpl {
    state: MyState
};

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn with_state(self) {
        // ... 
        // let state = self.state.lock().await;
        // ... 
    }

    async fn with_window<R: Runtime>(self, window: Window<R>) {
        // ...
    }
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_ipc_handler(
            ApiImpl {
                state: Arc::new(Mutex::new("state".to_string())),
            }
            .into_handler(),
        ))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

# Custom error handling

You can return a `Result<T, E>` to return an error if the procedure fails. This is will reject the promise on the frontend and throw an error.
If you're working with error types from Rust's std library, they will probably not implement `serde::Serialize` which is required for anything that is returned in the procedure.
In simple scenarios you can use `map_err` to convert these errors to `String`s. For more complex scenarios, you can create your own error type that implements `serde::Serialize`.
You can find an example using [thiserror](https://github.com/dtolnay/thiserror) [here](https://github.com/MatsDK/TauRPC/blob/main/example/src-tauri/src/main.rs).
You can also find more information about this in the [Tauri guides](https://v2.tauri.app/develop/calling-rust/#error-handling).

# Extra options for procedures

Inside your procedures trait you can add attributes to the defined methods. This can be used to ignore or rename a method. Renaming will change the name of the procedure on the frontend.

```rust
#[taurpc::procedures]
trait Api {
    // #[taurpc(skip)]
    #[taurpc(alias = "_hello_world_")]
    async fn hello_world();
}
```

# Routing

It is possible to define all your commands and events inside a single procedures trait, but this can quickly get cluttered. By using the `Router` struct you can create nested commands and events,
that you can call using a proxy TypeScript client.

The path of the procedures trait is set by using the `path` attribute on `#[taurpc::procedures(path = "")]`, then you can create an empty router and use the `merge` method to add handlers to the router.
You can only have 1 trait without a path specified, this will be the root. Finally instead of using `taurpc::create_ipc_handler()`, you should just call `into_handler()` on the router.

```rust
// Root procedures
#[taurpc::procedures]
trait Api {
    async fn hello_world();
}

#[derive(Clone)]
struct ApiImpl;

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn hello_world(self) {
        println!("Hello world");
    }
}

// Nested procedures, you can also do this (path = "api.events.users")
#[taurpc::procedures(path = "events")]
trait Events {
    #[taurpc(event)]
    async fn event();
}

#[derive(Clone)]
struct EventsImpl;

#[taurpc::resolvers]
impl Events for EventsImpl {}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .merge(ApiImpl.into_handler())
        .merge(EventsImpl.into_handler());

    tauri::Builder::default()
        .invoke_handler(router.into_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Now on the frontend you can use the proxy client.

```typescript
// Call `hello_world` on the root layer
await taurpc.hello_world()

// Listen for `event` on the `events` layer
const unlisten = await taurpc.events.event.on(() => {
  console.log('Hello World!')
})
```

# Typescript export configuration

You can specify a `Specta` typescript export configuration on the `Router`. These options will overwrite `Specta`'s defaults. Make sure to install the latest version of `specta_typescript`.
All available options can be found in [specta_typescript's docs](https://docs.rs/specta-typescript/latest/specta_typescript/struct.Typescript.html).

```rust
let router = Router::new()
    .export_config(
        specta_typescript::Typescript::default()
            .header("// My header\n")
            .bigint(specta_typescript::BigIntExportBehavior::String),
            // Make sure you have the specified formatter installed on your system.
            .formatter(specta_typescript::formatter::prettier)
    )
    .merge(ApiImpl.into_handler())
    .merge(EventsImpl.into_handler());
```

# Calling the frontend

Trigger [events](https://v2.tauri.app/develop/calling-rust/#event-system) on your TypeScript frontend from your Rust backend with a fully-typed experience.
The `#[taurpc::procedures]` macro also generates a struct that you can use to trigger the events, this means you can define the event types the same way you define the procedures.

First start by declaring the API structure, by default the event trigger struct will be identified by `TauRpc{trait_ident}EventTrigger`. If you want to change this, you can add an attribute to do this, `#[taurpc::procedures(event_trigger = ApiEventTrigger)]`.
For more details you can look at the [example](https://github.com/MatsDK/TauRPC/blob/main/example/src-tauri/src/main.rs).

You should add the `#[taurpc(event)]` attribute to your events. If you do this, you will not have to implement the corresponding resolver.

```rust
// src-tauri/src/main.rs

#[taurpc::procedures(event_trigger = ApiEventTrigger)]
trait Api {
    #[taurpc(event)]
    async fn hello_world();
}

#[derive(Clone)]
struct ApiImpl;

#[taurpc::resolvers]
impl Api for ApiImpl {}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_ipc_handler(ApiImpl.into_handler()))
        .setup(|app| {
            let trigger = ApiEventTrigger::new(app.handle());
            trigger.hello_world()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Then, on the frontend you can listen for the events with types:

```typescript
const unlisten = await taurpc.hello_world.on(() => {
  console.log('Hello World!')
})

// Run this inside a cleanup function, for example within useEffect in React and onDestroy in Svelte
unlisten()
```

## Sending an event to a specific window

By default, events are emitted to all windows. If you want to send an event to a specific window by label, you can do the following:

```rust
use taurpc::Windows;

trigger.send_to(Windows::One("main".to_string())).hello_world()?;
// Options:
//   - Windows::All (default)
//   - Windows::One(String)
//   - Windows::N(Vec<String>)
```

# Using channels

TauRPC will also generate types if you are using [Tauri Channels](https://v2.tauri.app/develop/calling-frontend/#channels).
On the frontend you will be able to pass a typed callback function to your command.

```rust
#[taurpc::ipc_type]
struct Update {
    progress: u8,
}

#[taurpc::procedures]
trait Api {
    async fn update(on_event: Channel<Update>);
}

#[derive(Clone)]
struct ApiImpl;

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn update(self, on_event: Channel<Update>) {
        for progress in [15, 20, 35, 50, 90] {
            on_event.send(Update { progress }).unwrap();
        }
    }
}
```

Calling the command:

```typescript
let taurpc = createTauRPCProxy()
await taurpc.update((update) => {
  console.log(update.progress)
})
```

# Features

- [x] Basic inputs
- [x] Struct inputs
- [x] Sharing state
  - [ ] Use Tauri's managed state?
- [x] Renaming methods
- [x] Nested routes
- [x] Merging routers
- [x] Custom error handling
- [x] Typed outputs
- [x] Async methods - [async traits👀](https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html)
  - [ ] Allow sync methods
- [x] Calling the frontend
- [x] Renaming event trigger struct
- [x] Send event to specific window
- [ ] React/Svelte handlers
