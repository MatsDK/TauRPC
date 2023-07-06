import { emit, listen } from '@tauri-apps/api/event'
import { createTauRPCProxy } from '../../../src'

// import { createTauRPCProxy } from 'taurpc'

const taurpc = await createTauRPCProxy()
listen('test_event', (event) => {
  console.log('even listen', event)
})

export { taurpc }
