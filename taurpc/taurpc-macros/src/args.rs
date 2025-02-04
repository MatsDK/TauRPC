use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{ext::IdentExt, spanned::Spanned, Ident, Pat, PatType, Type};

// TODO: Add raw request??
const RESERVED_ARGS: &[&str] = &["window", "state", "app_handle", "webview_window"];

pub(crate) struct Arg {
    pat: PatType,
    /// Should this argument be skipped in the generated types.
    pub skip_type: bool,
    // alias: String
}

impl Arg {
    pub fn ty(&self) -> &Type {
        &self.pat.ty
    }

    pub fn pat(&self) -> &Pat {
        &self.pat.pat
    }
}

impl From<PatType> for Arg {
    fn from(mut pat: PatType) -> Self {
        // Skip this argument in type generation based on our defined reserved argument names.
        let mut skip_type = matches!(
            pat.pat.as_ref(),
            Pat::Ident(pat_ident) if RESERVED_ARGS.iter().any(|&s| pat_ident.ident == s)
        );

        // These reserved args can also be used when they are tagged with an attribute, for
        // example `fn my_command(#[app_handle] h: AppHandle<impl Runtime>)`.
        pat.attrs = pat
            .attrs
            .into_iter()
            .filter(|attr| {
                if RESERVED_ARGS.iter().any(|s| attr.path().is_ident(s)) {
                    skip_type = true;
                    return false;
                }

                true
            })
            .collect::<Vec<_>>();

        Self { pat, skip_type }
    }
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.pat.to_tokens(tokens);
    }
}

/// Generate the code that extracts and deserializes the args from the tauri message.
pub(crate) fn parse_args(
    args: &[Arg],
    message: &Ident,
    proc_ident: &Ident,
) -> syn::Result<Vec<TokenStream2>> {
    args.iter()
        .map(|arg| parse_arg(arg, message, proc_ident))
        .collect()
}

fn parse_arg(arg: &Arg, message: &Ident, proc_ident: &Ident) -> syn::Result<TokenStream2> {
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

pub(crate) fn parse_arg_key(arg: &Arg) -> Result<String, syn::Error> {
    // we only support patterns that allow us to extract some sort of keyed identifier
    match arg.pat() {
        Pat::Ident(arg) => Ok(arg.ident.unraw().to_string()),
        Pat::Wild(_) => Ok("".into()), // we always convert to camelCase, so "_" will end up empty anyways
        Pat::Struct(s) => Ok(s.path.segments.last().unwrap().ident.to_string()),
        Pat::TupleStruct(s) => Ok(s.path.segments.last().unwrap().ident.to_string()),
        err => Err(syn::Error::new(
            err.span(),
            "only named, wildcard, struct, and tuple struct arguments allowed",
        )),
    }
}
