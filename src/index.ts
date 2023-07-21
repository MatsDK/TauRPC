import { invoke } from '@tauri-apps/api'
import { type EventCallback, listen } from '@tauri-apps/api/event'

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

type ListenerFn<
  TRoutes extends RoutesLayer,
  TProc extends string,
  TInput = FnInput<TRoutes[0], TProc>,
> = TInput extends null ? (() => void)
  : TInput extends Array<unknown> ? ((...p: TInput) => void)
  : ((p: TInput) => void)

type UnlistenFn = () => void

type InvokeLayer<
  TRoutes extends RoutesLayer,
  TProcedures extends string = TRoutes[0]['proc_name'],
> = {
  [TProc in TProcedures]: InvokeFn<TRoutes, TProc> & {
    on: (listener: ListenerFn<TRoutes, TProc>) => UnlistenFn
  }
}

type NestedProxy<TRoutes extends NestedRoutes> = {
  [K in keyof TRoutes]: TRoutes[K] extends RoutesLayer ? InvokeLayer<TRoutes[K]>
    : TRoutes[K] extends NestedRoutes ? NestedProxy<TRoutes[K]>
    : null
}

type TauRpcProxy<TRouter extends Router> =
  & InvokeLayer<TRouter['root']>
  & NestedProxy<Omit<TRouter, 'root'>>

type Payload = { proc_name: string; input_type: unknown }

const TAURPC_EVENT_NAME = 'TauRpc_event'

const createTauRPCProxy = async <TRouter extends Router>() => {
  const setup_response: string = await invoke('TauRPC__setup')
  const args_map = JSON.parse(setup_response) as Record<string, string[]>

  const listeners: Map<string, (args: unknown) => void> = new Map()

  const event_handler: EventCallback<Payload> = (event) => {
    const listener = listeners.get(event.payload.proc_name)
    if (!listener) return

    if (Array.isArray(event.payload.input_type)) {
      const _ = (listener as ((...args: unknown[]) => void))(
        ...event.payload.input_type as unknown[],
      )
    } else {
      listener(event.payload.input_type)
    }
  }

  await listen(TAURPC_EVENT_NAME, event_handler)

  return new window.Proxy({}, {
    get: (_target, p, _receiver) => {
      const path = p.toString()

      if (path === 'then') return
      if (!(path in args_map)) throw new Error(`Procedure '${path}' not found`)

      return new window.Proxy(() => {
        // Empty fn
      }, {
        get: (_target, prop, _receiver) => {
          if (prop !== 'on') return

          return (listener: (args: unknown) => void) => {
            listeners.set(path, listener)

            return () => listeners.delete(path)
          }
        },
        apply(_target, _thisArg, args) {
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          return handleProxyCall(path, args, args_map[path]!)
        },
      })
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

// type SplitKeyNested<TRouter extends Router, TPath extends keyof TRouter, T extends string> = T extends `${infer A}.${infer B}`
//   ? { [K in A]: SplitKeyNested<TRouter, TPath, B> }
//   : { [K in T]: TRouter[TPath] extends RoutesLayer ? InvokeLayer<TRouter[TPath]> : never };

// type SplitKey<TRouter extends Router, T extends keyof TRouter> = T extends `${infer A}.${infer B}`
//   ? { [K in A]: SplitKeyNested<TRouter, T, B> }
//   : { [K in T]: TRouter[T] extends RoutesLayer ? InvokeLayer<TRouter[T]> : never };

// type UnionToIntersection<U> =
//   (U extends any ? (k: U) => void : never) extends ((k: infer I) => void) ? I : never

// export type Convert<TRouter extends Router> = UnionToIntersection<SplitKey<TRouter, keyof TRouter>>

export { createTauRPCProxy }
