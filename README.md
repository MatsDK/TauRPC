# TauRPC

This package is a Tauri extension to give you a fully-typed RPC layer for [Tauri commands](https://tauri.app/v1/guides/features/command/).
The TS types corresponding to your pre-defined Rust backend API are generated on runtime, after which they can be used to call the backend from your Typescript frontend framework of choice.

# Usage🔧

First, add the crate to your dependencies:

```toml
# src-tauri/Cargo.toml

[dependencies]
taurpc = "0.1.0"
ts-rs = "6.2"
```

Then, declare and implement your RPC methods.

```rust
// src-tauri/src/main.rs

#[taurpc::procedures]
trait Api {
    fn hello_world();
}

#[derive(Clone)]
struct ApiImpl;
impl Api for ApiImpl {
    fn hello_world(self) {
        println!("Hello world");
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(ApiImpl.into_handler()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Then on the frontend install the taurpc package.

```
pnpm install taurpc
```

Now you can call your backend with types from inside typescript frontend files.

```typescript
import { createTauRPCProxy } from 'taurpc'

const taurpc = await createTauRPCProxy()
taurpc.hello_world()
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
    fn get_user() -> User;
}
```

# Accessing managed state

You can use Tauri's managed state within your commands, along the `state` argument, you can also use the `window` and `app_handle` arguments. [Tauri docs](https://tauri.app/v1/guides/features/command/#accessing-the-window-in-commands)

If you want your state to be mutable, you need to use a container that enables interior mutability, like a [Mutex](https://doc.rust-lang.org/std/sync/struct.Mutex.html).

```rust
// src-tauri/src/main.rs

use std::sync::Mutex;
use tauri::{Manager, Runtime, State, Window};

type MyState = Mutex<String>;

#[taurpc::procedures]
trait Api {
    fn method_with_state(state: State<MyState>);

    fn method_with_window<R: Runtime>(window: Window<R>);
}

#[derive(Clone)]
struct ApiImpl;
impl Api for ApiImpl {
    fn with_state(self, state: State<MyState>) {
        // ...
    }

    fn with_window<R: Runtime>(self, window: Window<R>) {
        // ...
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(taurpc::create_rpc_handler(ApiImpl.into_handler()))
        .manage(Mutex::new("some state value".to_string()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

# Features

- [x] Basic inputs
- [x] Struct inputs
- [x] Sharing state
- [ ] Renaming methods
- [ ] Merging routers
- [ ] Custom error handling
- [x] Typed outputs
- [ ] Async methods - [async traits👀](https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html)
- [ ] Calling the frontend
