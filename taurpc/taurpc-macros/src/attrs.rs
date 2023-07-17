use super::extend_errors;
use proc_macro2::Ident;
use quote::format_ident;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Expr, MetaNameValue, Token,
};

#[derive(Debug, Default)]
pub struct ProceduresAttrs {
    pub event_trigger_ident: Option<Ident>,
}

impl Parse for ProceduresAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let meta_items = input.parse_terminated(MetaNameValue::parse, Token![,])?;

        let mut errors = Ok(());
        let mut result = Self::default();

        for meta in meta_items {
            if meta.path.segments.len() != 1 {
                extend_errors!(
                    errors,
                    syn::Error::new(
                        meta.span(),
                        "taurpc::procedures does not support this meta item"
                    )
                );
            }

            let segment = meta.path.segments.first().unwrap();

            if segment.ident == "event_trigger" {
                if let Expr::Path(p) = meta.value {
                    if p.path.segments.len() != 1 {
                        extend_errors!(
                            errors,
                            syn::Error::new(
                                p.span(),
                                "taurpc::procedures does not support this meta value"
                            )
                        );
                    }

                    let event_trigger_ident = p.path.segments.last().unwrap();
                    result.event_trigger_ident =
                        Some(format_ident!("{}", event_trigger_ident.ident));
                }
            }
        }

        errors?;

        Ok(result)
    }
}
