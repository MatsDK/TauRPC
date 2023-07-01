use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use std::{collections::HashMap, env};
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{self, Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    FnArg, Generics, Ident, Pat, PatType, ReturnType, Token, Type, Visibility,
};

use crate::{method_fut_ident, parse_arg_key, parse_args};

const RESERVED_ARGS: &'static [&'static str] = &["window", "state", "app_handle"];

pub struct Procedures {
    pub ident: Ident,
    pub methods: Vec<RpcMethod>,
    pub vis: Visibility,
    pub generics: Generics,
}

impl Parse for Procedures {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let vis = input.parse()?;
        <Token![trait]>::parse(input)?;
        let ident: Ident = input.parse()?;

        let generics: Generics = input.parse()?;

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

            if procedure.ident == "setup" {
                Err(syn::Error::new(
                    procedure.ident.span(),
                    format!("method name `setup` in `{ident}` conflicts with internal method"),
                ))?;
            }
        }

        Ok(Procedures {
            ident,
            methods,
            vis,
            generics,
        })
    }
}

pub struct RpcMethod {
    pub ident: Ident,
    pub output: ReturnType,
    pub args: Vec<PatType>,
    pub generics: Generics,
}

impl Parse for RpcMethod {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        <Token![async]>::parse(input)?;
        <Token![fn]>::parse(input)?;

        let ident: Ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        parenthesized!(content in input);

        let mut args = Vec::new();
        for arg in content.parse_terminated(FnArg::parse, Token![,])? {
            match arg {
                // TODO: allow other Pat variants
                FnArg::Typed(pat_ty) if matches!(*pat_ty.pat, Pat::Ident(_)) => {
                    args.push(pat_ty);
                }
                err => Err(syn::Error::new(
                    err.span(),
                    "only named arguments are allowed",
                ))?,
            }
        }

        let output = input.parse()?;
        <Token![;]>::parse(input)?;

        Ok(RpcMethod {
            ident,
            output,
            args,
            generics,
        })
    }
}

pub struct ProceduresGenerator<'a> {
    pub trait_ident: &'a Ident,
    pub handler_ident: &'a Ident,
    pub inputs_ident: &'a Ident,
    pub outputs_ident: &'a Ident,
    pub outputs_futures_ident: &'a Ident,
    pub vis: Visibility,
    pub methods: &'a [RpcMethod],
    pub method_names: &'a [Ident],
    pub struct_idents: &'a [Ident],
    pub generics: Generics,
}

impl<'a> ProceduresGenerator<'a> {
    fn procedures_trait(&self) -> TokenStream2 {
        let ProceduresGenerator {
            trait_ident,
            handler_ident,
            methods,
            vis,
            generics,
            ..
        } = self;
        let unit_type: &Type = &parse_quote!(());

        let types_and_fns = methods.iter().map(
            |RpcMethod {
                 ident,
                 output,
                 args,
                 generics,
             }| {
                let ty_doc = format!("The response future returned by [`{trait_ident}::{ident}`].");
                let future_type_ident = method_fut_ident(ident);

                let output_ty: &Type = match output {
                    ReturnType::Type(_, ref ty) => ty,
                    ReturnType::Default => unit_type,
                };

                quote! {
                    #[allow(non_camel_case_types)]
                    #[doc = #ty_doc]
                    type #future_type_ident: std::future::Future<Output = #output_ty> + Send;
                    // type #future_type_ident: std::future::Future<Output = #output_ty>;

                    #[allow(non_camel_case_types)]
                    fn #ident #generics(self, #( #args ),*) -> Self::#future_type_ident;
                }
            },
        );

        quote! {
            #vis trait #trait_ident #generics: Sized {
                #( #types_and_fns )*

                fn into_handler(self) -> #handler_ident<Self> {
                    #handler_ident { methods: self }
                }
            }
        }
    }

    fn input_enum(&self) -> TokenStream2 {
        let ProceduresGenerator {
            methods,
            vis,
            inputs_ident,
            ..
        } = self;

        let inputs = methods.iter().map(|RpcMethod { ident, args, .. }| {
            let types = args
                .iter()
                .filter_map(|PatType { ty, pat, .. }| match &mut pat.as_ref() {
                    Pat::Ident(pat_ident) => {
                        let arg_name = pat_ident.ident.unraw().to_string();
                        if RESERVED_ARGS.iter().any(|&s| s == arg_name) {
                            return None;
                        }
                        Some(ty)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            quote! {
                #ident(( #( #types ),* ))
            }
        });

        quote! {
            #[derive(taurpc::TS, taurpc::Serialize)]
            #[serde(tag = "proc_name", content = "input_type")]
            #[allow(non_camel_case_types)]
            #vis enum #inputs_ident {
                #( #inputs ),*
            }
        }
    }

    fn output_enum(&self) -> TokenStream2 {
        let ProceduresGenerator {
            methods,
            vis,
            outputs_ident,
            ..
        } = self;

        let outputs = methods.iter().map(|RpcMethod { ident, output, .. }| {
            // TODO: handle Option<T>, Result<T, E>
            let output_ty = match output {
                ReturnType::Default => quote!(()),
                ReturnType::Type(_, ty) => ty.into_token_stream(),
            };

            quote! {
                #ident(#output_ty)
            }
        });

        quote! {
            #[derive(taurpc::TS, taurpc::Serialize)]
            #[serde(tag = "proc_name", content = "output_type")]
            #[allow(non_camel_case_types)]
            #vis enum #outputs_ident {
                #( #outputs ),*
            }
        }
    }

    fn output_futures(&self) -> TokenStream2 {
        let ProceduresGenerator {
            methods,
            trait_ident,
            vis,
            outputs_futures_ident,
            outputs_ident,
            ..
        } = self;

        let outputs = methods.iter().map(|RpcMethod { ident, .. }| {
            let future_ident = method_fut_ident(ident);

            quote! {
                #ident(<P as #trait_ident>::#future_ident)
            }
        });

        let method_idents = methods.iter().map(|RpcMethod { ident, .. }| ident);

        quote! {
            #[allow(non_camel_case_types)]
            #vis enum #outputs_futures_ident<P: #trait_ident> {
                #( #outputs ),*
            }

            impl<P: #trait_ident> std::future::Future for #outputs_futures_ident<P> {
                type Output = #outputs_ident;

                fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>)
                    -> std::task::Poll<#outputs_ident>
                {
                    unsafe {
                        match std::pin::Pin::get_unchecked_mut(self) {
                            #(
                                #outputs_futures_ident::#method_idents(resp) =>
                                    std::pin::Pin::new_unchecked(resp)
                                        .poll(cx)
                                        .map(#outputs_ident::#method_idents),
                            )*
                        }
                    }
                }
            }

        }
    }

    fn procedures_handler(&self) -> TokenStream2 {
        let ProceduresGenerator {
            trait_ident,
            handler_ident,
            vis,
            inputs_ident,
            struct_idents,
            method_names,
            methods,
            outputs_ident,
            outputs_futures_ident,
            ..
        } = self;

        let path = generate_export_path();

        let invoke = format_ident!("__tauri__invoke__");
        let message = format_ident!("__tauri__message__");
        let resolver = format_ident!("__tauri__resolver__");

        let procedure_handlers = method_names.iter().zip(methods.iter()).map(
            |(
                proc_name,
                RpcMethod {
                    ident: method_ident,
                    args,
                    ..
                },
            )| {
                let args = parse_args(args, &message).unwrap();

                quote! { stringify!(#proc_name) => {
                    #outputs_futures_ident::#method_ident(
                        #trait_ident::#method_ident(
                            self.methods, #( #args.unwrap() ),*
                        )
                    )
                }}
            },
        );

        let mut args_map = HashMap::new();
        methods.iter().for_each(|RpcMethod { args, ident, .. }| {
            let args = args
                .iter()
                .filter(|PatType { pat, .. }| match &mut pat.as_ref() {
                    Pat::Ident(pat_ident) => {
                        let arg_name = pat_ident.ident.unraw().to_string();
                        !RESERVED_ARGS.iter().any(|&s| s == arg_name)
                    }
                    _ => false,
                })
                .map(parse_arg_key)
                .map(|r| r.unwrap())
                .collect::<Vec<_>>();

            args_map.insert(ident.to_string(), args);
        });

        let serialized_args_map = serde_json::to_string(&args_map).unwrap();

        quote! {

            #[derive(Clone)]
            #vis struct #handler_ident<P> {
                methods: P,
            }

            use ::tauri::command::private::*;
            impl<P: #trait_ident + Send + 'static, R: tauri::Runtime> taurpc::TauRpcHandler<R> for #handler_ident<P> {
                type Resp = #outputs_ident;

                fn handle_incoming_request(self, #invoke: tauri::Invoke<R>) {
                    #[allow(unused_variables)]
                    let ::tauri::Invoke { message: #message, resolver: #resolver } = #invoke;

                    #resolver.respond_async_serialized(async move {
                        let result: #outputs_futures_ident<P> = match #message.command() {
                            #( #procedure_handlers ),*
                            _ => {
                                // #resolver.reject(format!("message {} not found", #message.command()));
                                return Err(tauri::InvokeError::from("message not found"));
                            }
                        };

                        let kind = (&result).async_kind();
                        kind.future(result).await
                    });
                }

                fn setup() -> String {
                    #serialized_args_map.to_string()
                }

                fn generate_ts_types() {
                    let mut ts_types = String::new();

                    #(
                        let decl = <#struct_idents as taurpc::TS>::decl();
                        ts_types.push_str(&format!("export {}\n", decl));
                    )*

                    let input_enum_decl = <#inputs_ident as taurpc::TS>::decl();
                    ts_types.push_str(&format!("export {}\n", input_enum_decl));

                    let output_enum_decl = <#outputs_ident as taurpc::TS>::decl();
                    ts_types.push_str(&format!("export {}\n", output_enum_decl));

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
            self.procedures_handler(),
            self.input_enum(),
            self.output_enum(),
            self.output_futures(),
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
