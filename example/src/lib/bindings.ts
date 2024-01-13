// This file has been generated by Specta. DO NOT EDIT.

export type TauRpcEventsInputTypes =
  | { proc_name: 'test_ev'; input_type: null }
  | { proc_name: 'state_changed'; input_type: { __taurpc_type: string } }
  | { proc_name: 'vec_test'; input_type: { __taurpc_type: string[] } }
  | { proc_name: 'multiple_args'; input_type: [number, string[]] }

export type TauRpcEventsOutputTypes =
  | { proc_name: 'test_ev'; output_type: null }
  | { proc_name: 'state_changed'; output_type: null }
  | { proc_name: 'vec_test'; output_type: null }
  | { proc_name: 'multiple_args'; output_type: null }

export type User = { uid: number; first_name: string; last_name: string }

export type TauRpcApiOutputTypes =
  | { proc_name: 'update_state'; output_type: null }
  | { proc_name: 'get_window'; output_type: null }
  | { proc_name: 'get_app_handle'; output_type: null }
  | { proc_name: 'test_io'; output_type: User }
  | { proc_name: 'test_option'; output_type: null | null }
  | { proc_name: 'test_result'; output_type: User }
  | { proc_name: 'with_sleep'; output_type: null }
  | { proc_name: 'method_with_alias'; output_type: null }
  | { proc_name: 'ev'; output_type: null }
  | { proc_name: 'vec_test'; output_type: null }
  | { proc_name: 'multiple_args'; output_type: null }

export type TauRpcApiInputTypes =
  | { proc_name: 'update_state'; input_type: { __taurpc_type: string } }
  | { proc_name: 'get_window'; input_type: null }
  | { proc_name: 'get_app_handle'; input_type: null }
  | { proc_name: 'test_io'; input_type: { __taurpc_type: User } }
  | { proc_name: 'test_option'; input_type: null }
  | { proc_name: 'test_result'; input_type: { __taurpc_type: User } }
  | { proc_name: 'with_sleep'; input_type: null }
  | { proc_name: 'method_with_alias'; input_type: null }
  | { proc_name: 'ev'; input_type: { __taurpc_type: string } }
  | { proc_name: 'vec_test'; input_type: { __taurpc_type: string[] } }
  | { proc_name: 'multiple_args'; input_type: [string[], string] }

const ARGS_MAP = {
  '':
    '{"get_app_handle":[],"test_option":[],"test_io":["user"],"update_state":["new_value"],"method_with_alias":[],"vec_test":["arg"],"multiple_args":["arg","arg2"],"get_window":[],"test_result":["user"],"ev":["updated_value"],"with_sleep":[]}',
  'events':
    '{"state_changed":["new_state"],"test_ev":[],"vec_test":["args"],"multiple_args":["arg1","arg2"]}',
}
import { createTauRPCProxy as createProxy } from 'taurpc'

export const createTauRPCProxy = () => createProxy<Router>(ARGS_MAP)

type Router = {
  '': [TauRpcApiInputTypes, TauRpcApiOutputTypes]
  'events': [TauRpcEventsInputTypes, TauRpcEventsOutputTypes]
}
