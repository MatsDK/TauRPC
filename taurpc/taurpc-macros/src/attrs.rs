use super::extend_errors;
use proc_macro2::Ident;
use syn::{
    Attribute, Expr, Lit, LitStr, MetaNameValue, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// Attributes added on the procedures trait itself, `#[taurpc::procedures( ... )]`.
#[derive(Debug, Default)]
pub struct ProceduresAttrs {
    pub event_trigger_ident: Option<Ident>,
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
#[derive(Default)]
pub struct MethodAttrs {
    pub(crate) skip: bool,
    pub(crate) alias: Option<String>,
    pub(crate) is_event: bool,
    pub(crate) comments: Vec<String>,
    /// Attributes to forward to the generated code (e.g., #[allow(...)])
    pub(crate) passthrough_attrs: Vec<Attribute>,
}

impl Parse for MethodAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut res = MethodAttrs::default();
        let attrs = input.call(Attribute::parse_outer)?;

        let mut errors = Ok(());

        for attr in attrs {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &attr.meta {
                    if let Expr::Lit(expr_lit) = &meta.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            res.comments.push(lit_str.value().trim().to_string());
                        }
                    }
                }
            }
            if !attr.path().is_ident("taurpc") {
                // Forward non-taurpc attributes (like #[allow(...)], #[cfg(...)], etc.) to generated code
                res.passthrough_attrs.push(attr);
                continue;
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
