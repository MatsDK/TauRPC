# TauRPC

[![](https://img.shields.io/npm/v/taurpc)](https://www.npmjs.com/package/taurpc) [![](https://img.shields.io/crates/v/taurpc)](https://crates.io/crates/taurpc) [![](https://img.shields.io/docsrs/taurpc)](https://docs.rs/taurpc/) ![](https://img.shields.io/crates/l/taurpc)

This package is a Tauri extension to give you a fully-typed IPC layer for [Tauri commands](https://tauri.app/v1/guides/features/command/).
The TS types corresponding to your pre-defined Rust backend API are generated on runtime, after which they can be used to call the backend from your Typescript frontend framework of choice. You can also easily send events to the frontend with typed arguments from the Rust backend.

# Usage🔧

First, add the following crates to your `Cargo.toml`:

```toml
# src-tauri/Cargo.toml

[dependencies]
taurpc = "0.1.3"

ts-rs = "6.2"
tokio = { version = "1", features = ["full"] }
```

Then, declare and implement your IPC methods.

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
        .invoke_handler(taurpc::create_ipc_handler(ApiImpl.into_handler()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

The `#[taurpc::procedures]` trait will generate everything necessary for handling calls and the type-generation. Now, you should run `pnpm tauri dev` to generate and export the TS types (the types will be exported to `node_moduldes/.taurpc`).

Then on the frontend install the taurpc package.

```
pnpm install taurpc
```

Now you can call your backend with types from inside typescript frontend files.

```typescript
import { createTauRPCProxy } from 'taurpc'

const taurpc = await createTauRPCProxy()
await taurpc.hello_world()
```

The types for taurpc are generated once you start your application, run `pnpm tauri dev`. If the types are not picked up by the LSP, you may have to restart typescript to reload the types.

You can find a complete example (using Svelte) [here](https://github.com/MatsDK/TauRPC/tree/main/example).

# Using structs

If you want to you structs for the inputs/outputs of procedures, you should always add `#[taurpc::ipc_struct]` to make sure the coresponding ts types are generated.

```rust
#[taurpc::ipc_struct]
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

<!-- You can use Tauri's managed state within your commands, along the `state` argument, you can also use the `window` and `app_handle` arguments. [Tauri docs](https://tauri.app/v1/guides/features/command/#accessing-the-window-in-commands) -->

To share some state between procedures, you can add fields on the API implementation struct. If the state requires to be mutable, you need to use a container that enables interior mutability, like a [Mutex](https://doc.rust-lang.org/std/sync/struct.Mutex.html).

You can use the `window` and `app_handle` arguments just like with Tauri's commands. [Tauri docs](https://tauri.app/v1/guides/features/command/#accessing-the-window-in-commands)

```rust
// src-tauri/src/main.rs

use std::sync::{Arc, Mutex};
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
        // self.state.lock()
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
You can also find more information about this in the [Tauri guides](https://tauri.app/v1/guides/features/command/#error-handling).

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

# Calling the frontend

Trigger [events](https://tauri.app/v1/guides/features/events/) on your TypeScript frontend from your Rust backend with a fully-typed experience.
The `#[taurpc::procedures]` macro also generates a struct that you can use to trigger the events, this means you can define the event types the same way you define the procedures.

First start by declaring the API structure, by default the event trigger struct will be identified by `TauRpc{trait_ident}EventTrigger`. If you want to change this, you can add an attribute to do this, `#[taurpc::procedures(event_trigger = ApiEventTrigger)]`.
For more details you can look at the [example](https://github.com/MatsDK/TauRPC/blob/main/example/src-tauri/src/main.rs).

```rust
// src-tauri/src/main.rs

#[taurpc::procedures(event_trigger = ApiEventTrigger)]
trait Api {
    async fn hello_world();
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
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
import { defineResolvers } from 'taurpc'

const { on, subsribe, unsubscribe } = await defineResolvers()

on('hello_world', () => {
  console.log('Hello World!')
})

// Run this inside a cleanup function, for example in React and onDestroy in Svelte
unsubscribe()

// You can also unlisten from a single method like this
unsubsribe('hello_world')
```

## Sending an event to a specific window

By default, events are emitted to all windows. If you want to send an event to a specific window by label, you can do the following:

```rust
trigger.send_to("main").hello_world()?;
```

# Features

- [x] Basic inputs
- [x] Struct inputs
- [x] Sharing state
  - [ ] Use Tauri's managed state?
- [x] Renaming methods
- [ ] Nested routes
- [ ] Merging routers
- [x] Custom error handling
- [x] Typed outputs
- [x] Async methods - [async traits👀](https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html)
  - [ ] Allow sync methods
- [x] Calling the frontend
- [x] Renaming event trigger struct
- [x] Send event to specific window
- [ ] React/Svelte handlers
