import { createTauRPCProxy } from './bindings'

const taurpc = await createTauRPCProxy()

export { taurpc }
