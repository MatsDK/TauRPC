# taurpc

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
