import { invoke } from '@tauri-apps/api'
import { type EventCallback, listen } from '@tauri-apps/api/event'

type TauRpcInputs = { proc_name: string; input_type: unknown }
type TauRpcOutputs = { proc_name: string; output_type: unknown }

type RoutesLayer = [TauRpcInputs, TauRpcOutputs]
type NestedRoutes = {
  [route: string]: RoutesLayer | NestedRoutes
}
type Router = NestedRoutes & { '': RoutesLayer }

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

type SplitKeyNested<
  TRouter extends NestedRoutes,
  TPath extends keyof TRouter,
  T extends string,
> = T extends `${infer A}.${infer B}`
  ? { [K in A]: SplitKeyNested<TRouter, TPath, B> }
  : {
    [K in T]: TRouter[TPath] extends RoutesLayer ? InvokeLayer<TRouter[TPath]>
      : never
  }

type SplitKey<TRouter extends NestedRoutes, T extends keyof TRouter> = T extends
  `${infer A}.${infer B}` ? { [K in A]: SplitKeyNested<TRouter, T, B> }
  : {
    [K in T]: TRouter[T] extends RoutesLayer ? InvokeLayer<TRouter[T]> : never
  }

type UnionToIntersection<U> = (U extends any ? (k: U) => void : never) extends
  ((k: infer I) => void) ? I : never

type Convert<TRouter extends NestedRoutes> = UnionToIntersection<
  SplitKey<TRouter, keyof TRouter>
>

type TauRpcProxy<TRouter extends Router> =
  & InvokeLayer<TRouter['']>
  & Convert<Omit<TRouter, ''>>

type Payload = {
  event_name: string
  event: { proc_name: string; input_type: unknown }
}

type Listeners = Map<string, (args: unknown) => void>
const TAURPC_EVENT_NAME = 'TauRpc_event'

const createTauRPCProxy = async <TRouter extends Router>() => {
  const args_map = await getArgsMap()
  const listeners: Listeners = new Map()

  const event_handler: EventCallback<Payload> = (event) => {
    const listener = listeners.get(event.payload.event_name)
    if (!listener) return

    if (Array.isArray(event.payload.event.input_type)) {
      const _ = (listener as ((...args: unknown[]) => void))(
        ...event.payload.event.input_type as unknown[],
      )
    } else {
      listener(event.payload.event.input_type)
    }
  }

  await listen(TAURPC_EVENT_NAME, event_handler)
  return nestedProxy(args_map, listeners) as TauRpcProxy<TRouter>
}

const nestedProxy = (
  args_maps: Record<string, Record<string, string[]>>,
  listeners: Listeners,
  path: string[] = [],
) => {
  return new window.Proxy({}, {
    get(_target, p, _receiver): object {
      const method_name = p.toString()
      const nested_path = [...path, method_name]
      const args_map = args_maps[path.join('.')]
      if (method_name === 'then' || !args_map) return {}

      if (method_name in args_map) {
        return new window.Proxy(() => {
          // Empty fn
        }, {
          get: (_target, prop, _receiver) => {
            if (prop !== 'on') return

            return (listener: (args: unknown) => void) => {
              listeners.set(nested_path.join('.'), listener)

              return () => listeners.delete(nested_path.join('.'))
            }
          },
          apply(_target, _thisArg, args) {
            return handleProxyCall(
              nested_path.join('.'),
              args,
              // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
              args_map[method_name]!,
            )
          },
        })
      } else if (nested_path.join('.') in args_maps) {
        return nestedProxy(args_maps, listeners, nested_path)
      } else {
        throw new Error(`'${nested_path.join('.')}' not found`)
      }
    },
  })
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

const getArgsMap = async () => {
  const setup: string = await invoke('TauRPC__setup')
  const args_map: Record<string, Record<string, string[]>> = {}
  Object.entries(JSON.parse(setup) as Record<string, string>).map(
    ([path, args]) => {
      args_map[path] = JSON.parse(args) as Record<string, string[]>
    },
  )

  return args_map
}

export { createTauRPCProxy }
