use super::extend_errors;
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{self, Parse, ParseStream},
    spanned::Spanned,
    Attribute, FnArg, Generics, Ident, Pat, PatType, ReturnType, Token, Visibility,
};

use crate::attrs::MethodAttrs;

pub struct Procedures {
    pub ident: Ident,
    pub methods: Vec<RpcMethod>,
    pub vis: Visibility,
    pub generics: Generics,
    pub attrs: Vec<Attribute>,
}

pub struct RpcMethod {
    pub ident: Ident,
    pub output: ReturnType,
    pub args: Vec<PatType>,
    pub generics: Generics,
    pub attrs: MethodAttrs,
}

impl Parse for Procedures {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        <Token![trait]>::parse(input)?;
        let ident: Ident = input.parse()?;

        let generics: Generics = input.parse()?;

        let content;
        braced!(content in input);

        let mut methods = Vec::new();
        while !content.is_empty() {
            let method = <RpcMethod>::parse(&content)?;
            if method.attrs.skip {
                continue;
            }
            methods.push(method);
        }

        let mut ident_errors = Ok(());
        for procedure in &methods {
            if procedure.ident == "into_handler" {
                extend_errors!(
                    ident_errors,
                    syn::Error::new(
                        procedure.ident.span(),
                        format!(
                            "method name conflicts with generated fn `{}::into_handler`",
                            ident.unraw()
                        ),
                    )
                );
            }

            if procedure.ident == "setup" {
                extend_errors!(
                    ident_errors,
                    syn::Error::new(
                        procedure.ident.span(),
                        format!(
                            "method name conflicts with generated fn `{}::setup`",
                            ident.unraw()
                        ),
                    )
                );
            }

            if procedure.ident == "send_to" {
                extend_errors!(
                    ident_errors,
                    syn::Error::new(
                        procedure.ident.span(),
                        format!(
                            "method name conflicts with generated fn `{}::send_to, this method is used to send scoped events`",
                            ident.unraw()
                        ),
                    )
                );
            }
        }
        ident_errors?;

        Ok(Procedures {
            ident,
            methods,
            vis,
            generics,
            attrs,
        })
    }
}

impl Parse for RpcMethod {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let attrs = MethodAttrs::parse(input)?;
        println!("{:?}", attrs);

        <Token![async]>::parse(input)?;
        <Token![fn]>::parse(input)?;

        let ident: Ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        parenthesized!(content in input);

        let mut args = Vec::new();
        for arg in content.parse_terminated(FnArg::parse, Token![,])? {
            match arg {
                FnArg::Typed(pat_ty) if matches!(*pat_ty.pat, Pat::Ident(_)) => {
                    args.push(pat_ty);
                }
                err => {
                    return Err(syn::Error::new(
                        err.span(),
                        "only named arguments are allowed",
                    ))
                }
            }
        }

        let output = input.parse()?;
        <Token![;]>::parse(input)?;

        Ok(RpcMethod {
            ident,
            output,
            args,
            generics,
            attrs,
        })
    }
}
