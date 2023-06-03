import { createTauRPCProxy } from '../../../src'
// import { createTauRPCProxy } from 'taurpc'

const taurpc = await createTauRPCProxy()

export { taurpc }
