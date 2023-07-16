import type { TauRpcInputs, TauRpcOutputs } from '.taurpc'
import { invoke } from '@tauri-apps/api'

type Procedures = TauRpcInputs['proc_name']

type FnInput<T extends Procedures> = Extract<
  TauRpcInputs,
  { proc_name: T }
>['input_type']

type FnOutput<T extends Procedures> = Extract<
  TauRpcOutputs,
  { proc_name: T }
>['output_type']

type Fn<T extends Procedures> = FnInput<T> extends null
  ? (() => Promise<FnOutput<T>>)
  : FnInput<T> extends Array<unknown>
  ? ((...p: FnInput<T>) => Promise<FnOutput<T>>)
  : ((p: FnInput<T>) => Promise<FnOutput<T>>)

type TauRPCProxy = {
  [K in Procedures]: Fn<K>
}

const createTauRPCProxy = async () => {
  const setup_response: string = await invoke('TauRPC__setup')
  const args_map = JSON.parse(setup_response) as Record<string, string[]>

  return new window.Proxy({}, {
    get: (_target, p, _receiver) => {
      const path = p.toString()

      if (path === 'then') return
      if (!(path in args_map)) throw new Error(`Procedure '${path}' not found`)

      return (...args: unknown[]) =>
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        handleProxyCall(path, args, args_map[path]!)
    },
  }) as TauRPCProxy
}

const handleProxyCall = async (
  path: string,
  args: unknown[],
  procedure_args: string[],
) => {
  const args_object: Record<string, unknown> = {}

  for (let idx = 0; idx < procedure_args.length; idx++) {
    const arg_name = procedure_args[idx]
    if (!arg_name) throw new Error('Received invalid arguments')

    args_object[arg_name] = args[idx]
  }

  const response = await invoke(
    `TauRPC__${path}`,
    args_object,
  )
  return response
}

export * from '../node_modules/.taurpc'
// export * from '.taurpc'
export { createTauRPCProxy }
