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
export type ResultMap = Record<string, Record<string, boolean>>
export type ErrorHandlingMode = 'throw' | 'result'

export type TauRpcResult<T, E> =
  | { status: 'ok'; data: T }
  | { status: 'error'; error: E }

export type TypedErrorFn =
  <T, E>(result: Promise<T>) => Promise<T | TauRpcResult<T, E>>

type CreateTauRPCProxyOptions = {
  resultMap?: ResultMap
  errorHandling?: ErrorHandlingMode
  typedError?: TypedErrorFn
}

const TAURPC_EVENT_NAME = 'TauRpc_event'

const passthroughTypedError: TypedErrorFn = async (result) => await result

const resultTypedError: TypedErrorFn = async (result) => {
  try {
    return { status: 'ok', data: await result }
  } catch (error) {
    if (error instanceof Error) throw error
    return { status: 'error', error: error as never }
  }
}

const createTauRPCProxy = <TRouter extends Router>(
  args: Record<string, string>,
  options: CreateTauRPCProxyOptions = {},
) => {
  const argsMap = parseArgsMap(args)
  const resultMap = options.resultMap ?? {}
  const errorHandling = options.errorHandling ?? 'throw'
  const typedError = options.typedError
    ?? (errorHandling === 'result' ? resultTypedError : passthroughTypedError)

  return nestedProxy(argsMap, resultMap, errorHandling, typedError) as TauRpcProxy<TRouter>
}

const nestedProxy = (
  argsMaps: ArgsMap,
  resultMap: ResultMap,
  errorHandling: ErrorHandlingMode,
  typedError: TypedErrorFn,
  path: string[] = [],
) => {
  return new window.Proxy({}, {
    get(_target, p, _receiver): object {
      const methodName = p.toString()
      const nestedPath = [...path, methodName]
      const routePath = path.join('.')
      const argsMap = argsMaps[routePath]
      if (methodName === 'then') return {}

      if (argsMap && methodName in argsMap) {
        return new window.Proxy(() => {
          // Empty fn
        }, {
          get: (_target, prop, _receiver) => {
            if (prop !== 'on') return

            const eventName = nestedPath.join('.')
            return async (listener: (args: unknown) => void) => {
              return await listen(
                TAURPC_EVENT_NAME,
                createEventHandlder(eventName, listener, argsMap),
              )
            }
          },
          apply(_target, _thisArg, args) {
            const shouldWrapResult =
              errorHandling === 'result' && resultMap[routePath]?.[methodName] === true

            return handleProxyCall(
              nestedPath.join('.'),
              args,
              // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
              argsMap[methodName]!,
              shouldWrapResult,
              typedError,
            )
          },
        })
      } else if (
        nestedPath.join('.') in argsMaps
        || Object.keys(argsMaps).some(path =>
          path.startsWith(`${nestedPath.join('.')}.`)
        )
      ) {
        return nestedProxy(argsMaps, resultMap, errorHandling, typedError, nestedPath)
      } else {
        throw new Error(`'${nestedPath.join('.')}' not found`)
      }
    },
  })
}

const handleProxyCall = async (
  path: string,
  args: unknown[],
  procedureArgs: string[],
  wrapResult: boolean,
  typedError: TypedErrorFn,
) => {
  const argsObject: Record<string, unknown> = {}

  for (let idx = 0; idx < procedureArgs.length; idx++) {
    const argName = procedureArgs[idx]
    if (!argName) throw new Error('Received invalid arguments')

    const arg = args[idx]
    if (typeof arg == 'function') {
      const channel = new Channel()
      channel.onmessage = arg as typeof channel.onmessage
      argsObject[argName] = channel
    } else {
      argsObject[argName] = arg
    }
  }

  const response = invoke(
    `TauRPC__${path}`,
    argsObject,
  )

  return wrapResult ? typedError(response) : await response
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
