import { type TauRpcApiInputs } from '.taurpc'
import { invoke } from '@tauri-apps/api'

type Procedures = TauRpcApiInputs['proc_name']

type FnInput<T extends Procedures> = Extract<
  TauRpcApiInputs,
  { proc_name: T }
>['input_type']

type FnOutput<T extends Procedures> = void

type Fn<T extends Procedures> = FnInput<T> extends Array<unknown>
  ? ((...p: FnInput<T>) => Promise<FnOutput<T>>)
  : ((p: FnInput<T>) => Promise<FnOutput<T>>)

type TauRPCProxy = {
  [K in Procedures]: Fn<K>
}

const createTauRPCProxy = () => {
  return new window.Proxy({}, {
    get: (_target, p, _receiver) => {
      return (...args: unknown[]) => {
        return handleProxyCall(p.toString(), args)
      }
    },
  }) as TauRPCProxy
}

const handleProxyCall = async (path: string, args: unknown[]) => {
  try {
    console.log(path, args)
    const response = await invoke(`TauRPC__${path}`, { args })
    return response
  } catch (error) {
    console.error(error)
  }
}

export * from 'H:/p/2022-2023/TauRPC/node_modules/.taurpc'
// export * from '.taurpc'
export { createTauRPCProxy }
