---
"@fltsci/taurpc": major
---

**BREAKING:** Split generated TypeScript output into two sibling files.

`#[taurpc::procedures(export_to = "...")]` now emits both a `proxy.ts` (the
runtime `createTauRPCProxy` factory plus a re-export of `InferCommandOutput`)
and a sibling `bindings.ts` (pure types — `Router`, IPC payload types, and
`export const ARGS_MAP`).

Use `./proxy` by default — it's the app-facing API and surfaces the full
public interface of `@fltsci/taurpc`, so consumers don't reference the
package by name.

Use `./bindings` **only** when you need types without the runtime — e.g. a
decoupled package that references `Router` but must not pull `@fltsci/taurpc`
into its bundle graph. `bindings.ts` contains zero npm imports, so Vite's
optimizeDeps scanner has nothing to pre-bundle when only `import type` walks
through it — fixes the dev-server race where newly discovered deps trigger
mid-test optimizer reloads.

**Migration.** Update consumer imports:

```ts
// before
import {
  createTauRPCProxy,
  type InferCommandOutput,
  type Router,
} from './taurpc/bindings'

// after
import type { Router } from './taurpc/bindings'
import { createTauRPCProxy, type InferCommandOutput } from './taurpc/proxy'
```
