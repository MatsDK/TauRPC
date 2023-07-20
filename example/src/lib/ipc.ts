// import { createTauRPCProxy } from '../../../src'

// import { createTauRPCProxy, } from 'taurpc'

// const taurpc = await createTauRPCProxy()

// export { taurpc }

import { createTauRPCProxy } from '../../node_modules/.taurpc'

const taurpc = await createTauRPCProxy()

export { taurpc }
