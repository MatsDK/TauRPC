use std::env;

use proc::{Procedures, ProceduresGenerator, RpcMethod};
use proc_macro::{self, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{self, Parse, ParseStream},
    parse_macro_input,
    token::Comma,
    FnArg, Ident, ImplItem, ItemImpl, ItemStruct, Pat, PatType, ReturnType, Token, Visibility,
};

mod proc;

use once_cell::sync::Lazy;
use std::sync::Mutex;

static STRUCT_NAMES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));

#[proc_macro_attribute]
pub fn rpc_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    STRUCT_NAMES.lock().unwrap().push(input.ident.to_string());

    quote! {
        #[derive(taurpc::Serialize, taurpc::TS)]
        #input
    }
    .into()
}

#[proc_macro_attribute]
pub fn procedures(attr: TokenStream, item: TokenStream) -> TokenStream {
    let Procedures {
        ref ident,
        ref methods,
        vis,
    } = parse_macro_input!(item as Procedures);

    let struct_idents = STRUCT_NAMES.lock().unwrap();

    ProceduresGenerator {
        trait_ident: ident,
        handler_ident: &format_ident!("TauRpc{}Handler", ident),
        inputs_ident: &format_ident!("TauRpc{}Inputs", ident),
        methods,
        struct_idents: &struct_idents
            .iter()
            .map(|name| format_ident!("{}", name))
            .collect::<Vec<_>>(),
        vis,
    }
    .into_token_stream()
    .into()
}
