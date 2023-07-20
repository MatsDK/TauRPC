use crate::args::{parse_arg_key, parse_args};
use crate::format_method_name;
use crate::{method_fut_ident, proc::IpcMethod};

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use syn::{
    ext::IdentExt, Attribute, GenericArgument, Generics, Ident, Pat, PatType, PathArguments,
    PathSegment, Type, TypePath, Visibility,
};
use syn::{parse_quote, TypeTuple, Variant};

const RESERVED_ARGS: &'static [&'static str] = &["window", "state", "app_handle"];

pub struct ProceduresGenerator<'a> {
    pub trait_ident: &'a Ident,
    pub handler_ident: &'a Ident,
    pub event_trigger_ident: &'a Ident,
    pub inputs_ident: &'a Ident,
    pub outputs_ident: &'a Ident,
    pub output_types_ident: &'a Ident,
    pub output_futures_ident: &'a Ident,
    pub vis: &'a Visibility,
    pub generics: &'a Generics,
    pub attrs: &'a [Attribute],
    pub methods: &'a [IpcMethod],
    pub method_output_types: &'a [&'a Type],
    pub alias_method_idents: &'a [Ident],
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
            ..
        } = self;

        let types_and_fns = methods.iter().zip(method_output_types.iter()).map(
            |(
                IpcMethod {
                    ident,
                    args,
                    generics,
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

                    // #( #attrs )*
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
            alias_method_idents,
            ..
        } = self;

        let inputs =
            alias_method_idents
                .iter()
                .zip(methods)
                .map(|(ident, IpcMethod { args, .. })| {
                    // Filter out Tauri's reserved arguments (state, window, app_handle), these args do not need TS types.
                    let types = args
                        .iter()
                        .filter(filter_reserved_args)
                        .map(|PatType { ty, .. }| ty)
                        .collect::<Vec<_>>();

                    let ty: Type = if types.len() == 1 {
                        let t = types[0];
                        parse_quote! {#t}
                    } else {
                        parse_quote! {
                            ( #( #types ),* )
                        }
                    };
                    quote! {
                        #ident(#ty)
                    }
                });

        quote! {
            // #[derive(taurpc::TS, taurpc::Serialize, Clone)]
            #[derive(specta::Type, taurpc::Serialize, Clone)]
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
            |(IpcMethod { ident, .. }, output_ty)| {
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
            vis,
            output_types_ident,
            method_output_types,
            alias_method_idents,
            ..
        } = self;

        let outputs = alias_method_idents
            .iter()
            .zip(method_output_types.iter())
            .map(|(ident, output_ty)| {
                let output_ty = unwrap_result_ty(output_ty);

                quote! {
                    #ident(#output_ty)
                }
            });

        quote! {
            // #[derive(taurpc::TS, taurpc::Serialize)]
            #[derive(specta::Type, taurpc::Serialize)]
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
            output_futures_ident,
            outputs_ident,
            ..
        } = self;

        let outputs = methods.iter().map(|IpcMethod { ident, .. }| {
            let future_ident = method_fut_ident(ident);

            quote! {
                #ident(<P as #trait_ident>::#future_ident)
            }
        });

        let method_idents = methods.iter().map(|IpcMethod { ident, .. }| ident);

        quote! {
            #[allow(non_camel_case_types)]
            #vis enum #output_futures_ident<P: #trait_ident> {
                #( #outputs ),*
            }

            impl<P: #trait_ident> std::future::Future for #output_futures_ident<P> {
                type Output = #outputs_ident;

                fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>)
                    -> std::task::Poll<#outputs_ident>
                {
                    unsafe {
                        match std::pin::Pin::get_unchecked_mut(self) {
                            #(
                                #output_futures_ident::#method_idents(resp) =>
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
            alias_method_idents,
            methods,
            outputs_ident,
            output_types_ident,
            ..
        } = self;

        let invoke = format_ident!("__tauri__invoke__");
        let message = format_ident!("__tauri__message__");
        let resolver = format_ident!("__tauri__resolver__");

        let procedure_handlers = alias_method_idents.iter().zip(methods.iter()).map(
            |(proc_name, IpcMethod { ident, args, .. })| {
                let args = parse_args(args, &message, ident).unwrap();
                let proc_name = format_method_name(proc_name);

                quote! { stringify!(#proc_name) => {
                    #resolver.respond_async_serialized(async move {
                        let res = #trait_ident::#ident(
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
        alias_method_idents
            .iter()
            .zip(methods)
            .for_each(|(ident, IpcMethod { args, .. })| {
                let args = args
                    .iter()
                    .filter(filter_reserved_args)
                    .map(parse_arg_key)
                    .map(|r| r.unwrap())
                    .collect::<Vec<_>>();

                args_map.insert(ident.to_string(), args);
            });

        let serialized_args_map = serde_json::to_string(&args_map).unwrap();

        let path = std::env::current_dir()
            .unwrap()
            .parent()
            .map(|p| p.join("node_modules\\.taurpc"));

        let export_path = match path {
            Some(path) => path.join("index.ts"),
            None => panic!("Export path not found"),
        };

        let export_path = export_path.to_str().unwrap();

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
                    // let mut ts_types = String::new();

                    // #(
                    //     let decl = <#struct_idents as taurpc::TS>::decl();
                    //     ts_types.push_str(&format!("export {}\n", decl));
                    // )*

                    // let input_enum_decl = specta::ts::export::<#inputs_ident>(&specta::ts::ExportConfiguration::default()).unwrap();
                    // // let input_enum_decl = <#inputs_ident as taurpc::TS>::decl();
                    // ts_types.push_str(&format!("{}\n", input_enum_decl));

                    // let output_enum_decl = specta::ts::export::<#output_types_ident>(&specta::ts::ExportConfiguration::default()).unwrap();
                    // // let output_enum_decl = <#output_types_ident as taurpc::TS>::decl();
                    // ts_types.push_str(&format!("{}\n", output_enum_decl));

                    specta::export::ts(#export_path).unwrap();
                    // Export to .ts file in `node_modules/.taurpc`
                    taurpc::export_files(#export_path);
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
            alias_method_idents,
            ..
        } = self;

        let method_triggers = alias_method_idents
            .iter()
            .zip(methods)
            .map(
                |(
                    alias_ident,
                    IpcMethod {
                        ident,
                        args,
                        generics,
                        ..
                    },
                )| {
                    let args = args.iter().filter(filter_reserved_args).collect::<Vec<_>>();

                    let arg_pats = args.iter().map(|arg| &*arg.pat).collect::<Vec<_>>();

                    quote! {
                        #[allow(unused)]
                        // #( #attrs )*
                        #vis fn #ident #generics(&self, #( #args ),*) -> tauri::Result<()> {
                            let req = #inputs_ident::#alias_ident(( #( #arg_pats ),* ));

                            self.0.call(req)
                        }
                    }
                },
            )
            .collect::<Vec<_>>();

        quote! {
            impl #event_trigger_ident {
                /// Generate a new client to trigger events on the client-side.
                #vis fn new(app_handle: tauri::AppHandle) -> Self {
                    let trigger = taurpc::EventTrigger::new(app_handle);

                    Self(trigger)
                }

                /// Trigger an event on a specific window by label.
                #vis fn send_to(&self, label: &str) -> Self {
                    let trigger = taurpc::EventTrigger::new_scoped_from_trigger(self.0.clone(), label);
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

/// Filter out Tauri's reserved argument names (state, window, app_handle), since
/// we should not generate the types for these.
fn filter_reserved_args(arg: &&PatType) -> bool {
    match &mut arg.pat.as_ref() {
        Pat::Ident(pat_ident) => {
            let arg_name = pat_ident.ident.unraw().to_string();
            !RESERVED_ARGS.iter().any(|&s| s == arg_name)
        }
        _ => false,
    }
}
