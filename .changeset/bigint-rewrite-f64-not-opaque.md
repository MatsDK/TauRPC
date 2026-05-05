---
"@fltsci/taurpc": patch
---

Rewrite bigint primitives (`i64`, `u64`, `usize`, `isize`, `i128`, `u128`, `f128`) to `Primitive::f64` instead of an opaque `define("number")` reference.

Both forms render as `number` in the generated TS, but only `Primitive::f64` is accepted as a `serde_json` map key by `specta-typescript`'s `validate_map_key`. The opaque form would surface as `Invalid map key at 'HashMap.<map_key>': opaque references cannot be validated as serde_json map keys` whenever a taurpc command's arg or return type contained a HashMap/IndexMap with a bigint key (e.g. `HashMap<i64, V>` or any newtype wrapping such a map).
