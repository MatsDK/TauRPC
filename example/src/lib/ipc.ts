import { createTauRPCProxy } from '../../node_modules/.taurpc'

const taurpc = await createTauRPCProxy()

export { taurpc }
