use super::extend_errors;
use proc_macro2::Ident;
use quote::format_ident;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Expr, LitStr, MetaNameValue, Token,
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

#[derive(Default, Debug)]
pub struct MethodAttrs {
    pub(crate) skip: bool,
    pub(crate) alias: Option<String>,
}

impl Parse for MethodAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut res = MethodAttrs::default();
        let attrs = input.call(Attribute::parse_outer)?;

        let mut errors = Ok(());

        for attr in attrs {
            if !attr.path().is_ident("taurpc") {
                extend_errors!(
                    errors,
                    syn::Error::new(
                        attr.meta.span(),
                        "these attributes are not supported, use `#[taurpc(...)]` instead"
                    )
                );
                // continue;
            }

            if let Err(e) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    res.skip = true;
                    Ok(())
                } else if meta.path.is_ident("alias") {
                    let value = meta.value()?;
                    let alias: LitStr = value.parse()?;

                    res.alias = Some(alias.value());
                    Ok(())
                } else {
                    Err(meta.error("unsupported attribute"))
                }
            }) {
                extend_errors!(errors, e);
            };
        }

        errors?;

        Ok(res)
    }
}
