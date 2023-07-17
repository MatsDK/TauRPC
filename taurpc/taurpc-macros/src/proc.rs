use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use std::{collections::HashMap, env};
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{self, Parse, ParseStream},
    spanned::Spanned,
    Attribute, FnArg, GenericArgument, Generics, Ident, Pat, PatType, PathArguments, PathSegment,
    ReturnType, Token, Type, TypePath, Visibility,
};

use crate::{method_fut_ident, parse_arg_key, parse_args};

const RESERVED_ARGS: &'static [&'static str] = &["window", "state", "app_handle"];

/// https://github.com/google/tarpc/blob/master/plugins/src/lib.rs#L29
/// Accumulates multiple errors into a result.
/// Only use this for recoverable errors, i.e. non-parse errors. Fatal errors should early exit to
/// avoid further complications.
macro_rules! extend_errors {
    ($errors: ident, $e: expr) => {
        match $errors {
            Ok(_) => $errors = Err($e),
            Err(ref mut errors) => errors.extend($e),
        }
    };
}

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
    pub attrs: Vec<Attribute>,
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
            methods.push(<RpcMethod>::parse(&content)?);
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
        let attrs = input.call(Attribute::parse_outer)?;

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

pub struct ProceduresGenerator<'a> {
    pub trait_ident: &'a Ident,
    pub handler_ident: &'a Ident,
    pub event_trigger_ident: &'a Ident,
    pub inputs_ident: &'a Ident,
    pub outputs_ident: &'a Ident,
    pub output_types_ident: &'a Ident,
    pub outputs_futures_ident: &'a Ident,
    pub vis: &'a Visibility,
    pub generics: &'a Generics,
    pub attrs: &'a [Attribute],
    pub methods: &'a [RpcMethod],
    pub method_output_types: &'a [&'a Type],
    pub method_names: &'a [Ident],
    pub struct_idents: &'a [Ident],
}

impl<'a> ProceduresGenerator<'a> {
    fn procedures_trait(&self) -> TokenStream2 {
        let &ProceduresGenerator {
            trait_ident,
            handler_ident,
            methods,
            vis,
            generics,
            attrs,
            method_output_types,
            event_trigger_ident,
            ..
        } = self;

        let types_and_fns = methods.iter().zip(method_output_types.iter()).map(
            |(
                RpcMethod {
                    ident,
                    args,
                    generics,
                    attrs,
                    ..
                },
                output_ty,
            )| {
                let ty_doc = format!("The response future returned by [`{trait_ident}::{ident}`].");
                let future_type_ident = method_fut_ident(ident);

                quote! {
                    #[allow(non_camel_case_types)]
                    #[doc = #ty_doc]
                    type #future_type_ident: std::future::Future<Output = #output_ty> + Send;

                    #( #attrs )*
                    fn #ident #generics(self, #( #args ),*) -> Self::#future_type_ident;
                }
            },
        );

        quote! {
            #( #attrs )*
            #vis trait #trait_ident #generics: Sized {
                #( #types_and_fns )*

                /// Returns handler used for incoming requests and type generation.
                fn into_handler(self) -> #handler_ident<Self> {
                    #handler_ident { methods: self }
                }
            }
        }
    }

    fn input_enum(&self) -> TokenStream2 {
        let &Self {
            methods,
            vis,
            inputs_ident,
            ..
        } = self;

        let inputs = methods.iter().map(|RpcMethod { ident, args, .. }| {
            // Filter out Tauri's reserved arguments (state, window, app_handle), these args do not need TS types.
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
            #[derive(taurpc::TS, taurpc::Serialize, Clone)]
            #[serde(tag = "proc_name", content = "input_type", rename = "TauRpcInputs")]
            #[allow(non_camel_case_types)]
            #vis enum #inputs_ident {
                #( #inputs ),*
            }
        }
    }

    fn output_enum(&self) -> TokenStream2 {
        let &Self {
            methods,
            vis,
            outputs_ident,
            method_output_types,
            ..
        } = self;

        let outputs = methods.iter().zip(method_output_types.iter()).map(
            |(RpcMethod { ident, .. }, output_ty)| {
                quote! {
                    #ident(#output_ty)
                }
            },
        );

        quote! {
            #[derive(taurpc::Serialize)]
            #[serde(tag = "proc_name", content = "output_type")]
            #[allow(non_camel_case_types)]
            #vis enum #outputs_ident {
                #( #outputs ),*
            }
        }
    }

    // Create enum that is used for generating the TS types, unwrap Result types because
    // ts_rs::TS is not implemented for them.
    fn output_types_enum(&self) -> TokenStream2 {
        let &Self {
            methods,
            vis,
            output_types_ident,
            method_output_types,
            ..
        } = self;

        let outputs = methods.iter().zip(method_output_types.iter()).map(
            |(RpcMethod { ident, .. }, output_ty)| {
                let output_ty = unwrap_result_ty(output_ty);

                quote! {
                    #ident(#output_ty)
                }
            },
        );

        quote! {
            #[derive(taurpc::TS, taurpc::Serialize)]
            #[serde(tag = "proc_name", content = "output_type", rename="TauRpcOutputs")]
            #[allow(non_camel_case_types)]
            #vis enum #output_types_ident {
                #( #outputs ),*
            }
        }
    }

    fn output_futures(&self) -> TokenStream2 {
        let &Self {
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
        let &Self {
            trait_ident,
            handler_ident,
            vis,
            inputs_ident,
            struct_idents,
            method_names,
            methods,
            outputs_ident,
            output_types_ident,
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
                    #resolver.respond_async_serialized(async move {
                        let res = #trait_ident::#method_ident(
                            self.methods, #( #args.unwrap() ),*
                        );
                        let kind = (&res).async_kind();
                        kind.future(res).await
                    });
                }}
            },
        );

        // Generate json object containing the order and names of the arguments for the methods.
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

                    match #message.command() {
                        #( #procedure_handlers ),*
                        _ => {
                            #resolver.reject(format!("message `{}` not found", #message.command()));
                        }
                    };
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

                    let output_enum_decl = <#output_types_ident as taurpc::TS>::decl();
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

    fn event_trigger_struct(&self) -> TokenStream2 {
        let &Self {
            vis,
            event_trigger_ident,
            ..
        } = self;

        quote! {
            #[derive(Clone, Debug)]
            #vis struct #event_trigger_ident(taurpc::EventTrigger);
        }
    }

    fn impl_event_trigger(&self) -> TokenStream2 {
        let &Self {
            event_trigger_ident,
            vis,
            methods,
            inputs_ident,
            ..
        } = self;

        let method_triggers = methods
            .iter()
            .map(
                |RpcMethod {
                     ident,
                     args,
                     generics,
                     attrs,
                     ..
                 }| {
                    let args = args
                        .iter()
                        .filter_map(|arg| match &mut arg.pat.as_ref() {
                            Pat::Ident(pat_ident) => {
                                let arg_name = pat_ident.ident.unraw().to_string();
                                if RESERVED_ARGS.iter().any(|&s| s == arg_name) {
                                    return None;
                                }
                                Some(arg)
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();

                    let arg_pats = args.iter().map(|arg| &*arg.pat).collect::<Vec<_>>();

                    quote! {
                        #[allow(unused)]
                        #( #attrs )*
                        #vis fn #ident #generics(&self, #( #args ),*) -> tauri::Result<()> {
                            let req = #inputs_ident::#ident(( #( #arg_pats ),* ));

                            self.0.call(req)
                        }
                    }
                },
            )
            .collect::<Vec<_>>();

        quote! {
            impl #event_trigger_ident {
                #vis fn new(app_handle: tauri::AppHandle) -> Self {
                    let trigger = taurpc::EventTrigger::new(app_handle);

                    Self(trigger)
                }

                #( #method_triggers )*
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
            self.output_types_enum(),
            self.output_futures(),
            self.event_trigger_struct(),
            self.impl_event_trigger(),
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

// If a method returns a Result<T, E> type, we extract the first generic argument to use
// inside the types enum. This is necessary since the `ts-rs` crate does not support Result types.
// Otherwise just return the original type.
fn unwrap_result_ty(ty: &Type) -> &Type {
    let result_seg = match is_ty_result(ty) {
        Some(seg) => seg,
        None => return ty,
    };

    match &result_seg.arguments {
        PathArguments::AngleBracketed(path_args) => {
            if let GenericArgument::Type(ty) = path_args.args.first().unwrap() {
                return ty;
            }
        }
        _ => {}
    }

    ty
}

// TODO: This might break with other result types e.g.: io::Result.
fn is_ty_result(ty: &Type) -> Option<&PathSegment> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if let Some(seg) = path.segments.last() {
                if seg.ident == "Result" {
                    return Some(seg);
                }
            }
        }
        _ => {}
    }

    None
}
