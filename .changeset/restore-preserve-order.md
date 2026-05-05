---
"@fltsci/taurpc": patch
---

Restore `preserve_order` on the runtime `taurpc` crate's `serde_json` dependency.

The rc.24 rebuild against upstream main collapsed `serde_json = { version = "1", features = ["preserve_order"] }` to bare `serde_json = "1"`. The `taurpc-macros` sub-crate still pins `["preserve_order"]`, so codegen field order is deterministic at build time, but the runtime crate's `serde_json::Map` falls back to `BTreeMap`'s sorted iteration -- a mismatch that future runtime changes touching `serde_json::Map` would silently inherit. Re-pin to match the macros crate baseline.
