use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ext::IdentExt, spanned::Spanned, Ident, Pat, PatType};

pub(crate) fn parse_args(args: &Vec<PatType>, message: &Ident) -> syn::Result<Vec<TokenStream2>> {
    args.iter().map(|arg| parse_arg(arg, message)).collect()
}

fn parse_arg(arg: &PatType, message: &Ident) -> syn::Result<TokenStream2> {
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
        name: "placeholder",
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
