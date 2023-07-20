import { invoke } from '@tauri-apps/api'

type TauRpcInputs = { proc_name: string; input_type: unknown }
type TauRpcOutputs = { proc_name: string; output_type: unknown }

type RoutesLayer = [TauRpcInputs, TauRpcOutputs]
type NestedRoutes = {
  [route: string]: RoutesLayer | NestedRoutes
}
type Router = {
  root: RoutesLayer
} & NestedRoutes

type FnInput<TInputs extends TauRpcInputs, TProc extends string> = Extract<
  TInputs,
  { proc_name: TProc }
>['input_type']
type FnOutput<TOutputs extends TauRpcOutputs, TProc extends string> = Extract<
  TOutputs,
  { proc_name: TProc }
>['output_type']

type InvokeFn<
  TRoutes extends RoutesLayer,
  TProc extends string,
  TInput = FnInput<TRoutes[0], TProc>,
  TOutput = Promise<FnOutput<TRoutes[1], TProc>>,
> = TInput extends null ? (() => TOutput)
  : TInput extends Array<unknown> ? ((...p: TInput) => TOutput)
  : ((p: TInput) => TOutput)

type InvokeLayer<
  TRoutes extends RoutesLayer,
  TProcedures extends string = TRoutes[0]['proc_name'],
> = {
  [TProc in TProcedures]: InvokeFn<TRoutes, TProc>
}

type NestedProxy<TRoutes extends NestedRoutes> = {
  [K in keyof TRoutes]: TRoutes[K] extends RoutesLayer ? InvokeLayer<TRoutes[K]>
    : TRoutes[K] extends NestedRoutes ? NestedProxy<TRoutes[K]>
    : null
}

type TauRpcProxy<TRouter extends Router> =
  & InvokeLayer<TRouter['root']>
  & NestedProxy<Omit<TRouter, 'root'>>

const createTauRPCProxy = async <TRouter extends Router>() => {
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
  }) as TauRpcProxy<TRouter>
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

// type ResolverOptions = {
//   subsribe?: boolean
// }

// const defaultOptions = {
//   subsribe: true,
// } satisfies ResolverOptions

// type ListenerFn<T extends Procedures> = FnInput<T> extends null ? (() => void)
//   : FnInput<T> extends Array<unknown> ? ((...p: FnInput<T>) => void)
//   : ((p: FnInput<T>) => void)
// import { UnlistenFn } from "@tauri-apps/api/event"

// const defineResolvers = async <TRouter extends Router>(options: ResolverOptions = defaultOptions) => {
//   let unlistenFn: null | UnlistenFn

//   const listeners: Map<string, ListenerFn<Procedures>> = new Map()

//   const handler: EventCallback<TauRpcInputs> = (event) => {
//     const listener = listeners.get(event.payload.proc_name)
//     if (!listener) return

//     if (Array.isArray(event.payload.input_type)) {
//       const _ = (listener as ((...args: unknown[]) => void))(
//         ...event.payload.input_type as unknown[],
//       )
//     } else {
//       listener(event.payload.input_type)
//     }
//   }

//   if (options.subsribe) {
//     unlistenFn = await listen(TAURPC_EVENT_NAME, handler)
//   }

//   return {
//     on: <T extends Procedures>(event: T, listener: ListenerFn<T>) => {
//       listeners.set(event, listener)

//       return () => listeners.delete(event)
//     },
//     subsribe: async () => {
//       unlistenFn = await listen(TAURPC_EVENT_NAME, handler)
//     },
//     unsubscribe: (event?: Procedures) => {
//       if (event) {
//         listeners.delete(event)
//       } else {
//         unlistenFn?.()
//       }
//     },
//   }
// }

// export * from '../node_modules/.taurpc'
// // export * from '.taurpc'
// export { createTauRPCProxy, defineResolvers }

export { createTauRPCProxy }
