use proc::{Procedures, ProceduresGenerator, RpcMethod};
use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{ext::IdentExt, parse_macro_input, spanned::Spanned, Ident, ItemStruct, Pat, PatType};

mod proc;

use once_cell::sync::Lazy;
use std::sync::Mutex;

static STRUCT_NAMES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));

#[proc_macro_attribute]
pub fn rpc_struct(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    STRUCT_NAMES.lock().unwrap().push(input.ident.to_string());

    quote! {
        #[derive(taurpc::Serialize, taurpc::TS)]
        #input
    }
    .into()
}

#[proc_macro_attribute]
pub fn procedures(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let Procedures {
        ref ident,
        ref methods,
        vis,
        generics,
    } = parse_macro_input!(item as Procedures);

    let struct_idents = STRUCT_NAMES.lock().unwrap();

    ProceduresGenerator {
        trait_ident: ident,
        handler_ident: &format_ident!("TauRpc{}Handler", ident),
        inputs_ident: &format_ident!("TauRpcApiInputs"),
        outputs_ident: &format_ident!("TauRpcApiOutputs"),
        methods,
        method_names: &methods
            .iter()
            .map(|RpcMethod { ident, .. }| format_method_name(ident))
            .collect::<Vec<_>>(),
        struct_idents: &struct_idents
            .iter()
            .map(|name| format_ident!("{}", name))
            .collect::<Vec<_>>(),
        vis,
        generics,
    }
    .into_token_stream()
    .into()
}

fn format_method_name(method: &Ident) -> Ident {
    format_ident!("TauRPC__{}", method)
}

pub(crate) fn parse_args(args: &Vec<PatType>, message: &Ident) -> syn::Result<Vec<TokenStream2>> {
    args.iter()
        .map(|arg| parse_arg(&format_ident!("placeholder_test_command"), arg, message))
        .collect()
}

fn parse_arg(command: &Ident, arg: &PatType, message: &Ident) -> syn::Result<TokenStream2> {
    let key = parse_arg_key(arg)?;

    // catch self arguments that use FnArg::Typed syntax
    if key == "self" {
        return Err(syn::Error::new(
            key.span(),
            "unable to use self as a command function parameter",
        ));
    }

    Ok(quote!(::tauri::command::CommandArg::from_command(
      ::tauri::command::CommandItem {
        name: stringify!(#command),
        key: #key,
        message: &#message
      }
    )))
}

pub(crate) fn parse_arg_key(arg: &PatType) -> Result<String, syn::Error> {
    // we only support patterns that allow us to extract some sort of keyed identifier
    match &mut arg.pat.as_ref().clone() {
        Pat::Ident(arg) => Ok(arg.ident.unraw().to_string()),
        Pat::Wild(_) => Ok("".into()), // we always convert to camelCase, so "_" will end up empty anyways
        Pat::Struct(s) => Ok(s.path.segments.last_mut().unwrap().ident.to_string()),
        Pat::TupleStruct(s) => Ok(s.path.segments.last_mut().unwrap().ident.to_string()),
        err => {
            return Err(syn::Error::new(
                err.span(),
                "only named, wildcard, struct, and tuple struct arguments allowed",
            ))
        }
    }
}
