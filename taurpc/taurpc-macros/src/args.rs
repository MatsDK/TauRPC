use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ext::IdentExt, spanned::Spanned, Ident, Pat, PatType};

/// Generate the code that extracts and deserializes the args from the tauri message.
pub(crate) fn parse_args(
    args: &[PatType],
    message: &Ident,
    proc_ident: &Ident,
) -> syn::Result<Vec<TokenStream2>> {
    args.iter()
        .map(|arg| parse_arg(arg, message, proc_ident))
        .collect()
}

fn parse_arg(arg: &PatType, message: &Ident, proc_ident: &Ident) -> syn::Result<TokenStream2> {
    let key = parse_arg_key(arg)?;

    // catch self arguments that use FnArg::Typed syntax
    if key == "self" {
        return Err(syn::Error::new(
            key.span(),
            "unable to use self as a command function parameter",
        ));
    }

    // this way tauri knows how to deserialize the different types of the args
    Ok(quote!(::tauri::ipc::CommandArg::from_command(
      ::tauri::ipc::CommandItem {
        name: stringify!(#proc_ident),
        key: #key,
        message: &#message,
        acl: &None,
        plugin: None,
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
        err => Err(syn::Error::new(
            err.span(),
            "only named, wildcard, struct, and tuple struct arguments allowed",
        )),
    }
}
