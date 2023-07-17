import { createTauRPCProxy, defineResolvers } from '../../../src'

// import { createTauRPCProxy, } from 'taurpc'

const taurpc = await createTauRPCProxy()

const { unsubscribe, on } = await defineResolvers()

on('update_state', (value) => {
  console.log(value)
  // unsubscribe("update_state")
})

// subsribe()

export { taurpc }
