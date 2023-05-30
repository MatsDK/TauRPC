use std::{env, path::PathBuf};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{self, Parse, ParseStream},
    parse_macro_input, FnArg, Ident, Pat, PatType, ReturnType, Token, Visibility,
};

pub struct Procedures {
    pub ident: Ident,
    pub methods: Vec<RpcMethod>,
    pub vis: Visibility,
}

impl Parse for Procedures {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let vis = input.parse()?;
        <Token![trait]>::parse(input)?;
        let ident: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let mut methods = Vec::new();
        while !content.is_empty() {
            methods.push(<RpcMethod>::parse(&content)?);
        }

        for procedure in &methods {
            if procedure.ident == "into_handler" {
                Err(syn::Error::new(
                    procedure.ident.span(),
                    format!("method name conflicts with generated fn `{ident}::into_handler`"),
                ))?;
            }
        }

        Ok(Procedures {
            ident,
            methods,
            vis,
        })
    }
}

pub struct RpcMethod {
    pub ident: Ident,
    pub output: ReturnType,
    pub args: Vec<PatType>,
}

impl Parse for RpcMethod {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        // <Token![async]>::parse(input)?;
        <Token![fn]>::parse(input)?;

        let ident: Ident = input.parse()?;

        let content;
        parenthesized!(content in input);

        let mut args = Vec::new();
        for arg in content.parse_terminated(FnArg::parse, Token![,])? {
            match arg {
                FnArg::Typed(p) if matches!(*p.pat, Pat::Ident(_)) => args.push(p),
                _ => {
                    eprintln!("Not supported")
                }
            }
        }

        let output = input.parse()?;
        <Token![;]>::parse(input)?;

        Ok(RpcMethod {
            ident,
            output,
            args,
        })
    }
}

pub struct ProceduresGenerator<'a> {
    pub trait_ident: &'a Ident,
    pub handler_ident: &'a Ident,
    pub inputs_ident: &'a Ident,
    pub vis: Visibility,
    pub methods: &'a [RpcMethod],
    pub struct_idents: &'a [Ident],
}

impl<'a> ProceduresGenerator<'a> {
    fn procedures_trait(&self) -> TokenStream2 {
        let ProceduresGenerator {
            trait_ident,
            handler_ident,
            methods,
            vis,
            ..
        } = self;

        let types_and_fns = methods.iter().map(
            |RpcMethod {
                 ident,
                 output,
                 args,
             }| {
                quote! {
                    fn #ident(self, #( #args ),*) #output;
                }
            },
        );

        quote! {
            #vis trait #trait_ident: Sized {
                #( #types_and_fns )*

                fn into_handler(self) -> #handler_ident<Self> {
                    #handler_ident { methods: self }
                }
            }
        }
    }

    fn input_enum(&self) -> TokenStream2 {
        let ProceduresGenerator {
            trait_ident,
            handler_ident,
            methods,
            vis,
            inputs_ident,
            ..
        } = self;

        let inputs = methods.iter().map(|RpcMethod { ident, args, .. }| {
            let types = args.iter().map(|PatType { ty, .. }| ty);

            quote! {
                #ident(( #( #types ),* ))
            }
        });

        quote! {
            #[derive(taurpc::TS, taurpc::Serialize)]
            #[serde(tag = "proc_name", content = "output_ty")]
            #vis enum #inputs_ident {
                #( #inputs ),*
            }
        }
    }

    fn rest(&self) -> TokenStream2 {
        let ProceduresGenerator {
            handler_ident,
            vis,
            inputs_ident,
            struct_idents,
            ..
        } = self;

        let path = generate_export_path();

        quote! {
            #[derive(Clone)]
            #vis struct #handler_ident<P> {
                methods: P,
            }

            impl<P> taurpc::TauRpcHandler for #handler_ident<P> {
                fn handle_incoming_request(self) {
                    println!("handle incoming event");
                }

                fn generate_ts_types() {
                    let mut ts_types = String::new();

                    #(
                        let decl = <#struct_idents as taurpc::TS>::decl();
                        ts_types.push_str(&format!("{}\n", decl));
                    )*

                    let input_enum_decl = <#inputs_ident as taurpc::TS>::decl();
                    ts_types.push_str(&input_enum_decl);

                    // Export to .ts file in `node_modules/.taurpc`
                    let path = std::path::Path::new(#path);
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                    std::fs::write(path, &ts_types).unwrap();
                }
            }
        }
    }
}

impl<'a> ToTokens for ProceduresGenerator<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(vec![
            self.procedures_trait(),
            self.rest(),
            self.input_enum(),
        ])
    }
}

fn generate_export_path() -> String {
    let path = env::current_dir()
        .unwrap()
        .parent()
        .map(|p| p.join("node_modules\\.taurpc"));

    match path {
        Some(path) => path
            .join("index.ts")
            .into_os_string()
            .into_string()
            .unwrap(),
        None => panic!("Export path not found"),
    }
}
