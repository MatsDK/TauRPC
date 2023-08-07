use super::extend_errors;
use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Expr, Lit, LitStr, MetaNameValue, Token,
};

/// Attributes added on the procedures trait itself, `#[taurpc::procedures( ... )]`.
#[derive(Debug, Default)]
pub struct ProceduresAttrs {
    pub event_trigger_ident: Option<Ident>,
    pub export_to: Option<String>,
    pub path: String,
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

            if meta.path.is_ident("event_trigger") {
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

                    let ident = p.path.get_ident().unwrap();
                    result.event_trigger_ident = Some(ident.clone());
                }
            } else if meta.path.is_ident("export_to") {
                if let Expr::Lit(p) = meta.value {
                    match p.lit {
                        Lit::Str(str) => result.export_to = Some(str.value()),
                        _ => {
                            extend_errors!(
                                errors,
                                syn::Error::new(p.span(), "export_to should be a str")
                            );
                        }
                    }
                } else {
                    extend_errors!(
                        errors,
                        syn::Error::new(meta.path.span(), "export_to should be a str")
                    );
                }
            } else if meta.path.is_ident("path") {
                if let Expr::Lit(p) = meta.value {
                    match p.lit {
                        Lit::Str(str) => {
                            // TODO: validate path
                            result.path = str.value()
                        }
                        _ => {
                            extend_errors!(
                                errors,
                                syn::Error::new(p.span(), "path should be a str")
                            );
                        }
                    }
                } else {
                    extend_errors!(
                        errors,
                        syn::Error::new(meta.path.span(), "path should be a str")
                    );
                }
            } else {
                extend_errors!(
                    errors,
                    syn::Error::new(meta.path.span(), "Unsupported attribute")
                );
            }
        }

        errors?;

        Ok(result)
    }
}

/// Attributes defined on methods inside a procedures trait.
/// Parse the attributes to make sure they are defined in the correct way, like `#[taurpc( ... )]`, accumulate
/// all errors and then display them together with `extend_errors!()`.  
#[derive(Default, Debug)]
pub struct MethodAttrs {
    pub(crate) skip: bool,
    pub(crate) alias: Option<String>,
    pub(crate) is_event: bool,
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
                } else if meta.path.is_ident("event") {
                    res.is_event = true;
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
