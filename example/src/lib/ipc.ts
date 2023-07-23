import { createTauRPCProxy } from '../../node_modules/.taurpc'
// import { createTauRPCProxy } from '.taurpc'

const taurpc = await createTauRPCProxy()

export { taurpc }
