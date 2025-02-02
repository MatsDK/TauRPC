use anyhow::{bail, Context, Result};
use heck::ToLowerCamelCase;
use itertools::Itertools;
use specta::datatype::{Function, FunctionResultVariant};
use specta::TypeCollection;
use specta_typescript as ts;
use specta_typescript::Typescript;
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

static BOILERPLATE_TS_IMPORT: &str = r#"

import { createTauRPCProxy as createProxy } from "taurpc"
"#;

static BOILERPLATE_TS_EXPORT: &str = r#"

export const createTauRPCProxy = () => createProxy<Router>(ARGS_MAP)
"#;

/// Export the generated TS types with the code necessary for generating the client proxy.
///
/// By default, if the `export_to` attribute was not specified on the procedures macro, it will be exported
/// to `node_modules/.taurpc` and a `package.json` will also be generated to import the package.
/// Otherwise the code will just be export to the .ts file specified by the user.
pub(super) fn export_types(
    export_path: Option<&'static str>,
    args_map: HashMap<String, String>,
    export_config: ts::Typescript,
    functions: HashMap<String, Vec<Function>>,
    type_map: TypeCollection,
) -> Result<()> {
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
        bail!("`export_to` path should be a ts file");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create directory for exported bindings")?;
    }

    let types = export_config
        .export(&type_map)
        .context("Failed to generate types with specta")?;

    // Put headers always at the top of the file, followed by the module imports.
    let framework_header = export_config.framework_header.as_ref();
    let body = types.split_once(framework_header).unwrap().1;

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .context("Cannot open bindings file")?;

    file.write_all(export_config.header.as_bytes()).unwrap();
    file.write_all(framework_header.as_bytes()).unwrap();
    file.write_all(BOILERPLATE_TS_IMPORT.as_bytes()).unwrap();
    file.write_all(body.as_bytes()).unwrap();

    let args_entries: String = args_map
        .iter()
        .map(|(k, v)| format!("'{k}':'{v}'"))
        .join(", ");
    let router_args = format!("{{ {args_entries} }}");

    file.write_all(format!("const ARGS_MAP = {router_args}\n").as_bytes())
        .unwrap();
    file.write_all(
        generate_functions_router(functions, type_map, &export_config)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    file.write_all(BOILERPLATE_TS_EXPORT.as_bytes()).unwrap();

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(&export_path)
            .parent()
            .map(|path| path.join("package.json"))
            .context("Failed to create 'package.json' path")?;

        std::fs::write(package_json_path, PACKAGE_JSON)
            .context("failed to create 'package.json'")?;
    }

    // Format the output file if the user specified a formatter on `export_config`.
    export_config
        .format(path)
        .context("Failed to format exported bindings")?;
    Ok(())
}

fn generate_functions_router(
    functions: HashMap<String, Vec<Function>>,
    type_map: TypeCollection,
    export_config: &Typescript,
) -> Result<String> {
    let functions = functions
        .iter()
        .map(|(path, functions)| {
            let functions = functions
                .iter()
                .map(|function| generate_function(function, &export_config, &type_map))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join(", \n");

            format!("'{path}': {{ {functions} }}")
        })
        .collect::<Vec<String>>()
        .join(",\n");

    Ok(format!("export type Router = {{ {functions} }};\n"))
}

fn generate_function(
    function: &Function,
    export_config: &Typescript,
    type_map: &TypeCollection,
) -> Result<String> {
    let args = function
        .args()
        .map(|(name, typ)| {
            ts::datatype(
                &export_config,
                &FunctionResultVariant::Value(typ.clone()),
                &type_map,
            )
            .map(|ty| format!("{}: {}", name.to_lower_camel_case(), ty))
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .join(", ");

    let ret_type = match function.result() {
        Some(FunctionResultVariant::Value(t)) => ts::datatype(
            &export_config,
            &FunctionResultVariant::Value(t.clone()),
            &type_map,
        )?,
        // TODO: handle result types
        Some(FunctionResultVariant::Result(t, _e)) => ts::datatype(
            &export_config,
            &FunctionResultVariant::Value(t.clone()),
            &type_map,
        )?,
        None => "void".to_string(),
    };

    // TODO: add docs to functions

    let name = function.name().split_once("_taurpc_fn__").unwrap().1;
    Ok(format!(r#"{name}: ({args}) => Promise<{ret_type}>"#))
}
