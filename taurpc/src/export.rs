use anyhow::{bail, Context, Result};
use heck::ToLowerCamelCase;
use itertools::Itertools;
use specta::datatype::{Function, FunctionReturnType};
use specta::TypeCollection;
use specta_typescript::Typescript;
use specta_typescript::{self as ts, primitives};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

static PACKAGE_JSON: &str = r#"
{
    "name": ".taurpc",
    "types": "index.ts"
}
"#;

static BOILERPLATE_TS_IMPORT: &str = r#"

import { createTauRPCProxy as createProxy, type InferCommandOutput } from 'taurpc'
type TAURI_CHANNEL<T> = (response: T) => void
"#;

static BOILERPLATE_TS_EXPORT: &str = r#"

export const createTauRPCProxy = () => createProxy<Router>(ARGS_MAP)
export type { InferCommandOutput }
"#;

/// Export the generated TS types with the code necessary for generating the client proxy.
///
/// By default, if the `export_to` attribute was not specified on the procedures macro, it will be exported
/// to `node_modules/.taurpc` and a `package.json` will also be generated to import the package.
/// Otherwise the code will just be export to the .ts file specified by the user.
pub(super) fn export_types(
    export_path: Option<&'static str>,
    args_map: BTreeMap<String, String>,
    export_config: ts::Typescript,
    functions: BTreeMap<String, Vec<Function>>,
    mut type_map: TypeCollection,
) -> Result<()> {
    let export_path = get_export_path(export_path);
    let path = Path::new(&export_path);

    if path.is_dir() || !export_path.ends_with(".ts") {
        bail!("`export_to` path should be a ts file");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create directory for exported bindings")?;
    }

    // Export `types_map` containing all referenced types.
    // type_map.remove(<tauri::ipc::Channel<()> as specta::NamedType>::ID);
    let types = export_config
        .export(&type_map)
        .context("Failed to generate types with specta")?;

    // Put headers always at the top of the file, followed by the module imports.
    let framework_header = export_config.framework_header.as_ref();
    let body = match types.split_once(framework_header) {
        Some((_, body)) => body,
        None => {
            eprintln!("Failed to split types with framework header");
            ""
        }
    };

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .context("Cannot open bindings file")?;

    try_write(&mut file, &export_config.header);
    try_write(&mut file, &framework_header);
    try_write(&mut file, &BOILERPLATE_TS_IMPORT);
    try_write(&mut file, &body);

    let args_entries: String = args_map
        .iter()
        .map(|(k, v)| format!("'{k}':'{v}'"))
        .join(", ");
    let router_args = format!("{{ {args_entries} }}");

    try_write(&mut file, &format!("const ARGS_MAP = {router_args}\n")); // TODO: Do this with `serde_json`
    let functions_router = generate_functions_router(functions, type_map, &export_config); // TODO: Using object primitive
    try_write(&mut file, &functions_router);
    try_write(&mut file, &BOILERPLATE_TS_EXPORT);

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(&export_path)
            .parent()
            .map(|path| path.join("package.json"))
            .context("Failed to create 'package.json' path")?;

        std::fs::write(package_json_path, PACKAGE_JSON)
            .context("failed to create 'package.json'")?;
    }

    // Format the output file if the user specified a formatter on `export_config`.
    // export_config.format(path).context(
    //     "Failed to format exported bindings, make sure you have the correct formatter installed",
    // )?; // TODO: Specta no longer supports this
    Ok(())
}

fn generate_functions_router(
    functions: BTreeMap<String, Vec<Function>>,
    type_map: TypeCollection,
    export_config: &Typescript,
) -> String {
    let functions = functions
        .iter()
        .filter_map(|(path, path_functions)| {
            let mut function_names_and_funcs: Vec<_> =
                path_functions.iter().map(|f| (f.name(), f)).collect();
            function_names_and_funcs.sort_by(|a, b| a.0.cmp(b.0));

            let functions = function_names_and_funcs
                .iter()
                .map(|(_, function)| generate_function(function, export_config, &type_map))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| eprintln!("Error generating functions: {:?}", e))
                .unwrap_or_default()
                .join(", \n");

            Some(format!(r#""{path}": {{{functions}}}"#))
        })
        .collect::<Vec<String>>()
        .join(",\n");

    format!("export type Router = {{ {functions} }};\n")
}

fn generate_function(
    function: &Function,
    export_config: &Typescript,
    type_map: &TypeCollection,
) -> Result<String> {
    let args = function
        .args()
        .into_iter()
        .map(|(name, typ)| {
            primitives::reference(export_config, type_map, typ)
                .map(|ty| format!("{}: {}", name.to_lower_camel_case(), ty))
        })
        .collect::<Result<Vec<_>, _>>()
        .context("An error occured while generating command args")?
        .join(", ");

    let return_ty = match function.result() {
        Some(FunctionReturnType::Value(t)) => primitives::reference(export_config, type_map, t)?,
        // TODO: handle result types
        Some(FunctionReturnType::Result(t, _e)) => {
            primitives::reference(export_config, type_map, t)?
        }
        None => "void".to_string(),
    };

    let name = function.name().split_once("_taurpc_fn__").unwrap().1;
    Ok(format!(r#"{name}: ({args}) => Promise<{return_ty}>"#))
}

fn default_export_path() -> String {
    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting current directory: {:?}", e);
            return "bindings.ts".to_string();
        }
    };

    match current_dir
        .join("../bindings.ts")
        .into_os_string()
        .into_string()
    {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error getting default export path: {:?}", e);
            "bindings.ts".to_string()
        }
    }
}

fn get_export_path(export_path: Option<&'static str>) -> String {
    export_path
        .map(|p| p.to_string())
        .unwrap_or(default_export_path())
}

fn try_write(file: &mut File, data: &str) {
    match file.write_all(data.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error writing to file: {:?}", e);
        }
    };
}
