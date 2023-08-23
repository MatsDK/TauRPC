---
"taurpc": patch
---

- Fix [issue](https://github.com/MatsDK/TauRPC/issues/14) when the only argument is of type `Vec<T>` or a tuple.
- Set default export to `../bindings.ts`.
- Directly generate args_map with types instead of using `TauRpc__setup`.
