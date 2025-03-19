import { Channel, invoke } from '@tauri-apps/api/core'
import { type EventCallback, listen, UnlistenFn } from '@tauri-apps/api/event'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type RoutesLayer = { [key: string]: (...args: any) => unknown }
type NestedRoutes = {
  [route: string]: RoutesLayer | NestedRoutes
}
type Router = NestedRoutes & { ''?: RoutesLayer }

type InvokeFn<
  TRoutes extends RoutesLayer,
  TProc extends string,
> = TRoutes[TProc]

// Helper type to swap the return type of functions returning Promise<T> to void
type SwapReturnTypeToVoid<T> = T extends (...args: infer A) => Promise<unknown>
  ? (...args: A) => void
  : never

type ListenerFn<
  TRoutes extends RoutesLayer,
  TProc extends string,
> = SwapReturnTypeToVoid<TRoutes[TProc]>

type InvokeLayer<
  TRoutes extends RoutesLayer,
  TProcedures extends Extract<keyof TRoutes, string> = Extract<
    keyof TRoutes,
    string
  >,
> = {
  [TProc in TProcedures]: InvokeFn<TRoutes, TProc> & {
    on: (listener: ListenerFn<TRoutes, TProc>) => Promise<UnlistenFn>
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

type RouterPathsToNestedObject<
  TRouter extends NestedRoutes,
  TPath extends keyof TRouter,
> = TPath extends `${infer A}.${infer B}`
  ? { [K in A]: SplitKeyNested<TRouter, TPath, B> }
  : {
    [K in TPath]: TRouter[TPath] extends RoutesLayer
      ? InvokeLayer<TRouter[TPath]>
      : never
  }

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type UnionToIntersection<U> = (U extends any ? (k: U) => void : never) extends
  ((k: infer I) => void) ? I : never

type ConvertToNestedObject<TRouter extends NestedRoutes> = UnionToIntersection<
  RouterPathsToNestedObject<TRouter, keyof TRouter>
>

type TauRpcProxy<TRouter extends Router> =
  & (TRouter[''] extends RoutesLayer ? InvokeLayer<TRouter['']>
    : object)
  & ConvertToNestedObject<Omit<TRouter, ''>>

type Payload = {
  event_name: string
  event: { proc_name: string; input_type: unknown }
}
type ListenFn = (args: unknown) => void
type ArgsMap = Record<string, Record<string, string[]>>

const TAURPC_EVENT_NAME = 'TauRpc_event'

const createTauRPCProxy = <TRouter extends Router>(
  args: Record<string, string>,
) => {
  const args_map = parseArgsMap(args)
  return nestedProxy(args_map) as TauRpcProxy<TRouter>
}

const nestedProxy = (
  args_maps: ArgsMap,
  path: string[] = [],
) => {
  return new window.Proxy({}, {
    get(_target, p, _receiver): object {
      const method_name = p.toString()
      const nested_path = [...path, method_name]
      const args_map = args_maps[path.join('.')]
      if (method_name === 'then') return {}

      if (args_map && method_name in args_map) {
        return new window.Proxy(() => {
          // Empty fn
        }, {
          get: (_target, prop, _receiver) => {
            if (prop !== 'on') return

            const event_name = nested_path.join('.')
            return async (listener: (args: unknown) => void) => {
              return await listen(
                TAURPC_EVENT_NAME,
                createEventHandlder(event_name, listener, args_map),
              )
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
      } else if (
        nested_path.join('.') in args_maps
        || Object.keys(args_maps).some(path =>
          path.startsWith(`${nested_path.join('.')}.`)
        )
      ) {
        return nestedProxy(args_maps, nested_path)
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

    const arg = args[idx]
    if (typeof arg == 'function') {
      const channel = new Channel()
      channel.onmessage = arg as typeof channel.onmessage
      args_object[arg_name] = channel
    } else {
      args_object[arg_name] = arg
    }
  }

  const response = await invoke(
    `TauRPC__${path}`,
    args_object,
  )
  return response
}

const createEventHandlder = (
  event_name: string,
  listener: ListenFn,
  args_map: ArgsMap[string],
): EventCallback<Payload> => {
  return (event) => {
    if (event_name !== event.payload.event_name) return

    const path_segments = event.payload.event_name.split('.')
    const ev = path_segments.pop()
    if (!ev) return

    const args = args_map[ev]
    if (!args) return

    if (args.length === 1) {
      listener(event.payload.event.input_type)
    } else if (Array.isArray(event.payload.event.input_type)) {
      const _ = (listener as ((...args: unknown[]) => void))(
        ...event.payload.event.input_type as unknown[],
      )
    } else {
      listener(event.payload.event.input_type)
    }
  }
}

const parseArgsMap = (args: Record<string, string>) => {
  const args_map: Record<string, Record<string, string[]>> = {}
  Object.entries(args).map(
    ([path, args]) => {
      args_map[path] = JSON.parse(args) as Record<string, string[]>
    },
  )

  return args_map
}

export type InferCommandOutput<
  TRouter extends Router,
  TPath extends keyof TRouter,
  TCommand extends keyof TRouter[TPath],
> = TRouter[TPath] extends RoutesLayer
  ? Awaited<ReturnType<TRouter[TPath][TCommand]>>
  : unknown

export { createTauRPCProxy }
