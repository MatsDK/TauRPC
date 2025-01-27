use heck::ToLowerCamelCase;
use itertools::Itertools;
use specta::datatype::{Function, FunctionResultVariant};
use specta::TypeCollection;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

static PACKAGE_JSON: &str = r#"
{
    "name": ".taurpc",
    "types": "index.ts"
}
"#;

static BOILERPLATE_TS_CODE: &str = r#"
import { createTauRPCProxy as createProxy } from "taurpc"

export const createTauRPCProxy = () => createProxy<Router>(ARGS_MAP)
"#;

/// Export the generated TS types with the code necessary for generating the client proxy.
///
/// By default, if the `export_to` attribute was not specified on the procedures macro, it will be exported
/// to `node_modules/.taurpc` and a `package.json` will also be generated to import the package.
/// Otherwise the code will just be export to the .ts file specified by the user.
pub(super) fn export_types(
    export_path: Option<&'static str>,
    handlers: Vec<(&'static str, &'static str)>,
    args_map: HashMap<String, String>,
    export_config: specta_typescript::Typescript,
    functions: HashMap<String, Vec<Function>>,
    type_map: TypeCollection,
) {
    println!("{functions:?}");
    let functions = functions
        .iter()
        .map(|(path, functions)| {
            let functions = functions
                .iter()
                .map(|function| {
                    let arg_defs = function
                        .args()
                        .map(|(name, typ)| {
                            specta_typescript::datatype(
                                &export_config,
                                &FunctionResultVariant::Value(typ.clone()),
                                &type_map,
                            )
                            .map(|ty| format!("{}: {}", name.to_lower_camel_case(), ty))
                        })
                        //TODO: remove unwrap
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();

                    println!("{:?}", function.result());

                    // let ret_type =
                    //     js_ts::handle_result(function, &cfg.type_map, ts, cfg.error_handling)?;

                    //                 let docs = {
                    //                     let mut builder = js_doc::Builder::default();

                    //                     if let Some(d) = &function.deprecated() {
                    //                         builder.push_deprecated(d);
                    //                     }

                    //                     if !function.docs().is_empty() {
                    //                         builder.extend(function.docs().split("\n"));
                    //                     }

                    //                     builder.build()
                    //                 };
                    Ok(generate_function(
                        // &docs,
                        // &function.name().to_lower_camel_case(),
                        &function.name(),
                        &arg_defs,
                        // Some(&ret_type),
                        // &js_ts::command_body(&cfg.plugin_name, function, true, cfg.error_handling),
                    ))
                })
                .collect::<Result<Vec<_>, ()>>()
                .unwrap()
                .join(", \n");

            format!("'{path}': {{ {functions} }}")
            // println!("{functions}");
            // functions
        })
        .collect::<Vec<String>>()
        .join(",\n");
    println!("{functions}");
    // let ret_type =
    // let ret_type =
    // let ret_type =

    let export_path = export_path.map(|p| p.to_string()).unwrap_or(
        std::env::current_dir()
            .unwrap()
            .join("../bindings.ts")
            .into_os_string()
            .into_string()
            .unwrap(),
    );
    let path = Path::new(&export_path);

    if path.is_dir() || !export_path.ends_with(".ts") {
        panic!("`export_to` path should be a ts file");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let types = export_config.export(&specta::export()).unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .unwrap();

    file.write_all(types.as_bytes()).unwrap();

    let args_entries: String = args_map
        .iter()
        .map(|(k, v)| format!("'{}':'{}'", k, v))
        .join(", ");
    let router_args = format!("{{{}}}", args_entries);

    file.write_all(format!("const ARGS_MAP = {}", router_args).as_bytes())
        .unwrap();
    file.write_all(BOILERPLATE_TS_CODE.as_bytes()).unwrap();
    file.write_all(generate_router_type(handlers).as_bytes())
        .unwrap();

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(&export_path)
            .parent()
            .map(|path| path.join("package.json"))
            .unwrap();

        std::fs::write(package_json_path, PACKAGE_JSON).unwrap();
    }
}

fn generate_function(
    // docs: &str,
    name: &str,
    args: &[String],
    // return_type: Option<&str>,
    // body: &str,
) -> String {
    let args = args.join(", ");
    format!(r#"{name}: ({args}) => Promise<unknown>"#)
}

fn generate_router_type(handlers: Vec<(&'static str, &'static str)>) -> String {
    let mut output = String::from("\ntype Router = {\n");

    for (path, handler_name) in handlers {
        output += &format!(
            "\t'{}': [TauRpc{}InputTypes, TauRpc{}OutputTypes],\n",
            path, handler_name, handler_name
        );
    }

    output += "}";
    output
}
