# TauRPC

[![](https://img.shields.io/npm/v/taurpc)](https://www.npmjs.com/package/taurpc) [![](https://img.shields.io/crates/v/taurpc)](https://crates.io/crates/taurpc) [![](https://img.shields.io/docsrs/taurpc)](https://docs.rs/taurpc/) ![](https://img.shields.io/crates/l/taurpc)

This package is a Tauri extension to give you a fully-typed RPC layer for [Tauri commands](https://tauri.app/v1/guides/features/command/).
The TS types corresponding to your pre-defined Rust backend API are generated on runtime, after which they can be used to call the backend from your Typescript frontend framework of choice.

# Usage🔧

First, add the following crates to your `Cargo.toml`:

```toml
# src-tauri/Cargo.toml

[dependencies]
taurpc = "0.1.0"

ts-rs = "6.2"
tokio = { version = "1", features = ["full"] }
```

Then, declare and implement your RPC methods.

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
fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(ApiImpl.into_handler()))
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

If you want to you structs for the inputs/outputs of procedures, you should always add `#[taurpc::rpc_struct]` to make sure the coresponding ts types are generated.

```rust
#[taurpc::rpc_struct]
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
    async fn method_with_state();

    async fn method_with_window<R: Runtime>(window: Window<R>);
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
fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(
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

# Features

- [x] Basic inputs
- [x] Struct inputs
- [x] Sharing state
  - [ ] Use Tauri's managed state?
- [ ] Renaming methods
- [ ] Merging routers
- [x] Custom error handling
- [x] Typed outputs
- [x] Async methods - [async traits👀](https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html)
  - [ ] Allow sync methods
- [ ] Calling the frontend
