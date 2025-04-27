# taurpc

## 1.8.1

### Patch Changes

- [`92b170e`](https://github.com/MatsDK/TauRPC/commit/92b170ed44f062b1da3989f1ce40cb315dcc0446) Thanks [@MatsDK](https://github.com/MatsDK)! - Actually return unlisten function for events

## 1.8.0

### Minor Changes

- [`04af2f3`](https://github.com/MatsDK/TauRPC/commit/04af2f3565571777f6d76f9fb3d71538ec574313) Thanks [@MatsDK](https://github.com/MatsDK)!
  - Support Tauri channels [#34](https://github.com/MatsDK/TauRPC/issues/34)
  - Better error handling in exporter [#43](https://github.com/MatsDK/TauRPC/issues/43)
  - Show correct names for parameters on the frontend types [#37](https://github.com/MatsDK/TauRPC/issues/37)

## 1.7.0

### Minor Changes

- [`3df869f`](https://github.com/MatsDK/TauRPC/commit/3df869fc85f7f1fcc41525207e504558b81bedee) Thanks [@MatsDK](https://github.com/MatsDK)! - Fix unnecessary await for event handlers [#38](https://github.com/MatsDK/TauRPC/issues/38).

## 1.6.0

### Minor Changes

- [`a2b457a`](https://github.com/MatsDK/TauRPC/commit/a2b457a0e4531fbd31ea5d5d6bb834e247375fec) Thanks [@MatsDK](https://github.com/MatsDK)! - support tauri@2.0.0

## 1.4.3

### Patch Changes

- [`2ffad75`](https://github.com/MatsDK/TauRPC/commit/2ffad7527a55b51fc926d90515331053777aa37a) Thanks [@MatsDK](https://github.com/MatsDK)!

  - Allow doc comments on IPC types - [#21](https://github.com/MatsDK/TauRPC/issues/21)
  - Allow users to declare a router without root procedures - [#22](https://github.com/MatsDK/TauRPC/issues/22)

## 1.4.2

### Patch Changes

- [`0a87d07`](https://github.com/MatsDK/TauRPC/commit/0a87d0778c9b64af1e21e0d9ca5bcb8a9f746ff5) Thanks [@MatsDK](https://github.com/MatsDK)! - Fix issue when the only argument is of type Vec<T> or a tuple for events.

## 1.4.1

### Patch Changes

- [`4c0b1b4`](https://github.com/MatsDK/TauRPC/commit/4c0b1b44ae83fdbbcb154d1f32904181a28a6419) Thanks [@MatsDK](https://github.com/MatsDK)! -
  - Fix [issue](https://github.com/MatsDK/TauRPC/issues/14) when the only argument is of type `Vec<T>` or a tuple.
  - Set default export to `../bindings.ts`.
  - Directly generate args_map with types instead of using `TauRpc__setup`.

## 1.4.0

### Minor Changes

- [`8df57cf`](https://github.com/MatsDK/TauRPC/commit/8df57cf221f8cab0a7de6c39f54eee9b095ad2d3) Thanks [@MatsDK](https://github.com/MatsDK)! - Allow users to create nested commands that can be called with a proxy-like ts client

## 1.3.1

### Patch Changes

- [`31690ca`](https://github.com/MatsDK/TauRPC/commit/31690cadacbee837b73fcf471955936296f67431) Thanks [@MatsDK](https://github.com/MatsDK)! - event attribute so you are not forced to implement a resolver for them

## 1.3.0

### Minor Changes

- [`8a7b495`](https://github.com/MatsDK/TauRPC/commit/8a7b495f6c96b8ef4f8fc706e4b51c1f2793ebc5) Thanks [@MatsDK](https://github.com/MatsDK)!
  - Switch from `ts_rs` to `specta` for the type-generation.
  - Allow to specify `export_to` attribute on procedures for exporting the generated types.
  - Windows enum for sending scoped events.
  - Common client for both invoking commands and listening to events.

## 1.2.4

### Patch Changes

- [`2bae0ca`](https://github.com/MatsDK/TauRPC/commit/2bae0ca9c1eee7f36d2ab2bcbd6773792babd475) Thanks [@MatsDK](https://github.com/MatsDK)! - alias/skip method attributes

## 1.2.3

### Patch Changes

- [`209358c`](https://github.com/MatsDK/TauRPC/commit/209358c2084e6a77a3e34e5a20b9a8614361720c) Thanks [@MatsDK](https://github.com/MatsDK)! - rename event trigger, event scope

- [`3c8fee9`](https://github.com/MatsDK/TauRPC/commit/3c8fee9af6571f420ec121c33adfc91382592681) Thanks [@MatsDK](https://github.com/MatsDK)! - trigger events on client side, with types

## 1.2.2

### Patch Changes

- [`0424f61`](https://github.com/MatsDK/TauRPC/commit/0424f611f812d8ccfc9055cbddbceee7a5fef023) Thanks [@MatsDK](https://github.com/MatsDK)! - Custom error handling using Result types

## 1.2.1

### Patch Changes

- [`3c98a2c`](https://github.com/MatsDK/TauRPC/commit/3c98a2cb0bf07fb3100a927d0aa2f84d76f8aea2) Thanks [@MatsDK](https://github.com/MatsDK)! - make procedures async

- [`054ed4b`](https://github.com/MatsDK/TauRPC/commit/054ed4b22afb25bc3d5b178f82485af4ec313c32) Thanks [@MatsDK](https://github.com/MatsDK)! - support for async methods

## 1.2.0

### Minor Changes

- [`0fc1bf0`](https://github.com/MatsDK/TauRPC/commit/0fc1bf07d1feb0e6520dafc0af23199bcb1dccc6) Thanks [@MatsDK](https://github.com/MatsDK)! - state/window/app_handle

- [`60deedf`](https://github.com/MatsDK/TauRPC/commit/60deedfa91a7d04f654e1d52677d5e543b365788) Thanks [@MatsDK](https://github.com/MatsDK)! - use state/window/app_handle in commands

## 1.1.0

### Minor Changes

- [`0896862`](https://github.com/MatsDK/TauRPC/commit/089686280c2192a104467a0976b107b520fb8a8b) Thanks [@MatsDK](https://github.com/MatsDK)! - add types for outputs
