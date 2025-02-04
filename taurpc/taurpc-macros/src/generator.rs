use crate::args::{parse_arg_key, parse_args};
use crate::{method_fut_ident, proc::IpcMethod};

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use syn::{parse_quote, Attribute, Generics, Ident, Type, Visibility};

pub struct ProceduresGenerator<'a> {
    pub trait_ident: &'a Ident,
    pub handler_ident: &'a Ident,
    pub event_trigger_ident: &'a Ident,
    pub export_path: Option<String>,
    pub path_prefix: String,
    pub inputs_ident: &'a Ident,
    pub outputs_ident: &'a Ident,
    pub output_futures_ident: &'a Ident,
    pub vis: &'a Visibility,
    pub generics: &'a Generics,
    pub attrs: &'a [Attribute],
    pub methods: &'a [IpcMethod],
    pub method_output_types: &'a [&'a Type],
    pub alias_method_idents: &'a [Ident],
}

impl ProceduresGenerator<'_> {
    fn procedures_trait(&self) -> TokenStream2 {
        let &ProceduresGenerator {
            trait_ident,
            handler_ident,
            methods,
            vis,
            generics,
            attrs,
            method_output_types,
            alias_method_idents,
            ..
        } = self;

        let fn_types = alias_method_idents.iter().zip(methods).map(
            |(ident, IpcMethod { output, args, .. })| {
                let args = args.iter().filter(|&arg| !arg.skip_type);
                let fn_ident = fn_ident(trait_ident, ident);

                quote! {
                    #[specta::specta]
                    #[allow(non_snake_case, unused_variables)]
                    fn #fn_ident( #( #args ),*) #output {
                        unimplemented!();
                    }
                }
            },
        );

        let types_and_fns = methods.iter().zip(method_output_types.iter()).filter_map(
            |(
                IpcMethod {
                    ident,
                    args,
                    generics,
                    attrs,
                    ..
                },
                output_ty,
            )| {
                // skip methods that are marked as events, these methods don't need an implementation
                if attrs.is_event {
                    return None;
                }
                let ty_doc = format!("The response future returned by [`{trait_ident}::{ident}`].");
                let future_type_ident = method_fut_ident(ident);

                Some(quote! {
                    #[allow(non_camel_case_types)]
                    #[doc = #ty_doc]
                    type #future_type_ident: std::future::Future<Output = #output_ty> + Send;

                    fn #ident #generics(self, #( #args ),*) -> Self::#future_type_ident;
                })
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

            #( #fn_types )*
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
                    // Filter out Tauri's reserved arguments (state, window, app_handle).
                    let types = args
                        .iter()
                        .filter(|&arg| !arg.skip_type)
                        .map(|arg| arg.ty())
                        .collect::<Vec<_>>();

                    // Tuples with 1 element were parsed as Type::Paren, which is not supported by specta.
                    // This may not be necessary and there is probably a better solution, but this works.
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
            #[derive(taurpc::serde::Serialize, Clone)]
            #[serde(tag = "proc_name", content = "input_type")]
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
            #[derive(taurpc::serde::Serialize)]
            #[serde(tag = "proc_name", content = "output_type")]
            #[allow(non_camel_case_types)]
            #vis enum #outputs_ident {
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

        let outputs = methods
            .iter()
            .filter_map(|IpcMethod { ident, attrs, .. }| {
                if attrs.is_event {
                    return None;
                }
                let future_ident = method_fut_ident(ident);

                Some(quote! {
                    #ident(<P as #trait_ident>::#future_ident)
                })
            })
            .collect::<Vec<_>>();

        // If there are no commands, there are no future outputs and the generic P will be unused resulting in errors.
        if outputs.is_empty() {
            return quote! {};
        }

        let method_idents = methods
            .iter()
            .filter(|IpcMethod { attrs, .. }| !attrs.is_event)
            .map(|IpcMethod { ident, .. }| ident);

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
            alias_method_idents,
            methods,
            ref export_path,
            ref path_prefix,
            ..
        } = self;

        let invoke = format_ident!("__tauri_invoke__");
        let message = format_ident!("__tauri_message__");
        let resolver = format_ident!("__tauri_resolver__");

        let procedure_handlers = alias_method_idents.iter().zip(methods.iter()).filter_map(
            |(
                proc_name,
                IpcMethod {
                    ident, args, attrs, ..
                },
            )| {
                if attrs.is_event {
                    return None;
                }
                let args = parse_args(args, &message, ident).unwrap();

                Some(quote! { stringify!(#proc_name) => {
                    #resolver.respond_async_serialized(async move {
                        let res = #trait_ident::#ident(
                            self.methods, #( #args.unwrap() ),*
                        );
                        let kind = (&res).async_kind();
                        kind.future(res).await
                    });
                }})
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
                    .filter(|arg| !arg.skip_type)
                    .map(parse_arg_key)
                    .map(|r| r.unwrap())
                    .collect::<Vec<_>>();

                args_map.insert(ident.to_string(), args);
            });

        let serialized_args_map = serde_json::to_string(&args_map).unwrap();
        let export_path = match export_path {
            Some(path) => quote! { Some(#path) },
            None => quote! { None },
        };

        let fn_names = alias_method_idents
            .iter()
            .map(|ident| fn_ident(trait_ident, ident));

        quote! {
            #[derive(Clone)]
            #vis struct #handler_ident<P> {
                methods: P,
            }

            use ::tauri::ipc::private::*;
            impl<R: ::tauri::Runtime, P: #trait_ident + Clone + Send + 'static> taurpc::TauRpcHandler<R> for #handler_ident<P> {
                const TRAIT_NAME: &'static str = stringify!(#trait_ident);
                const PATH_PREFIX: &'static str = #path_prefix;
                const EXPORT_PATH: Option<&'static str> = #export_path;

                fn handle_incoming_request(self, #invoke: tauri::ipc::Invoke<R>) {
                    #[allow(unused_variables)]
                    let ::tauri::ipc::Invoke { message: #message, resolver: #resolver, .. } = #invoke;

                    // Remove `TauRpc__` prefix
                    let prefix = #message.command()[8..].to_string();
                    let mut prefix = prefix.split(".").collect::<Vec<_>>();
                    // // Get the actual name of the command
                    let cmd_name = prefix.pop().unwrap().to_string();

                    match cmd_name.as_str() {
                        #( #procedure_handlers ),*
                        _ => {
                            #resolver.reject(format!("message `{}` not found", #message.command()));
                        }
                    };
                }

                fn spawn(self) -> tokio::sync::broadcast::Sender<std::sync::Arc<tauri::ipc::Invoke<R>>> {
                    let (tx, mut rx) = tokio::sync::broadcast::channel(32);

                    tokio::spawn(async move {
                        while let Ok(invoke) = rx.recv().await {
                            if let Some(invoke) = std::sync::Arc::into_inner(invoke) {
                                self.clone().handle_incoming_request(invoke);
                            }
                        }
                    });

                    tx
                }

                fn args_map() -> String {
                    #serialized_args_map.to_string()
                }

                fn collect_fn_types(mut types_map: &mut specta::TypeCollection) -> Vec<specta::datatype::Function> {
                    specta::function::collect_functions![#( #fn_names ),*](&mut types_map)
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
            #vis struct #event_trigger_ident<RT: ::tauri::Runtime>(taurpc::EventTrigger<RT>);
        }
    }

    fn impl_event_trigger(&self) -> TokenStream2 {
        let &Self {
            event_trigger_ident,
            vis,
            methods,
            inputs_ident,
            alias_method_idents,
            ref path_prefix,
            ..
        } = self;

        let method_triggers = alias_method_idents
            .iter()
            .zip(methods)
            .filter_map(
                |(
                    alias_ident,
                    IpcMethod {
                        ident,
                        args,
                        generics,
                        attrs,
                        ..
                    },
                )| {
                    // skip methods that are not marked as events
                    if !attrs.is_event {
                        return None;
                    }

                    let args = args.iter().filter(|arg| !arg.skip_type).collect::<Vec<_>>();
                    let arg_pats = args.iter().map(|arg| arg.pat()).collect::<Vec<_>>();

                    Some(quote! {
                        #[allow(unused)]
                        #vis fn #ident #generics(&self, #( #args ),*) -> tauri::Result<()> {
                            let proc_name = stringify!(#alias_ident);
                            let req = #inputs_ident::#alias_ident(( #( #arg_pats ),* ));

                            self.0.call(proc_name, req)
                        }
                    })
                },
            )
            .collect::<Vec<_>>();

        quote! {
            impl<RT: ::tauri::Runtime> #event_trigger_ident<RT> {
                /// Generate a new client to trigger events on the client-side.
                #vis fn new(app_handle: tauri::AppHandle<RT>) -> Self {
                    let trigger = taurpc::EventTrigger::new(app_handle, String::from(#path_prefix));

                    Self(trigger)
                }

                /// Trigger an event with a specific scope.
                ///
                /// Options:
                ///    - Windows::All (default)
                ///    - Windows::One(String)
                ///    - Windows::N(Vec<String>)
                #vis fn send_to(&self, scope: taurpc::Windows) -> Self {
                    let trigger = taurpc::EventTrigger::new_scoped_from_trigger(self.0.clone(), scope);
                    Self(trigger)
                }

                #( #method_triggers )*
            }
        }
    }
}

impl ToTokens for ProceduresGenerator<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(vec![
            self.procedures_trait(),
            self.procedures_handler(),
            self.input_enum(),
            self.output_enum(),
            self.output_futures(),
            self.event_trigger_struct(),
            self.impl_event_trigger(),
        ])
    }
}

fn fn_ident(trait_ident: &Ident, fn_ident: &Ident) -> Ident {
    format_ident!("{trait_ident}_taurpc_fn__{fn_ident}")
}
