import { listen } from '@tauri-apps/api/event'
import { createTauRPCProxy } from '../../../src'

// import { createTauRPCProxy, } from 'taurpc'

const taurpc = await createTauRPCProxy()
listen('TauRpc_event', (event) => {
  console.log('even listen', event)
})

export { taurpc }
