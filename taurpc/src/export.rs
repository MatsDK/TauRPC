use anyhow::{bail, Context, Result};
use heck::ToLowerCamelCase;
use itertools::Itertools;
use specta::datatype::{Function, FunctionResultVariant};
use specta::TypeCollection;
use specta_typescript as ts;
use specta_typescript::Typescript;
use std::collections::BTreeMap;
use std::ffi::OsStr;
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
/// By default, if the `export_to` attribute was not specified on the procedures macro, there will
/// be nothing exported. Otherwise the code will just be export to the .ts file specified by the user.
pub(super) fn export_types(
    export_path: impl AsRef<Path>,
    args_map: BTreeMap<String, String>,
    export_config: ts::Typescript,
    functions: BTreeMap<String, Vec<Function>>,
    mut type_map: TypeCollection,
) -> Result<()> {
    let path = export_path.as_ref();
    if path.extension() != Some(OsStr::new("ts")) {
        bail!("`export_to` path should be a ts file");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create directory for exported bindings")?;
    }

    // Export `types_map` containing all referenced types.
    type_map.remove(<tauri::ipc::Channel<()> as specta::NamedType>::sid());
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
    try_write(&mut file, framework_header);
    try_write(&mut file, BOILERPLATE_TS_IMPORT);
    try_write(&mut file, body);

    let args_entries: String = args_map
        .iter()
        .map(|(k, v)| format!("'{k}':'{v}'"))
        .join(", ");
    let router_args = format!("{{ {args_entries} }}");

    try_write(&mut file, &format!("const ARGS_MAP = {router_args}\n"));
    let functions_router = generate_functions_router(functions, type_map, &export_config);
    try_write(&mut file, &functions_router);
    try_write(&mut file, BOILERPLATE_TS_EXPORT);

    if path
        .to_string_lossy()
        .replace("\\", "/")
        .ends_with("node_modules/.taurpc/index.ts")
    {
        let package_json_path = path
            .parent()
            .map(|path| path.join("package.json"))
            .context("Failed to create 'package.json' path")?;

        std::fs::write(package_json_path, PACKAGE_JSON)
            .context("failed to create 'package.json'")?;
    }

    // Format the output file if the user specified a formatter on `export_config`.
    export_config.format(path).context(
        "Failed to format exported bindings, make sure you have the correct formatter installed",
    )?;
    Ok(())
}

fn generate_functions_router(
    functions: BTreeMap<String, Vec<Function>>,
    type_map: TypeCollection,
    export_config: &Typescript,
) -> String {
    let functions = functions
        .iter()
        .map(|(path, path_functions)| {
            let mut function_names_and_funcs: Vec<_> =
                path_functions.iter().map(|f| (f.name(), f)).collect();
            function_names_and_funcs.sort_by(|a, b| a.0.cmp(b.0));

            let functions = function_names_and_funcs
                .iter()
                .map(|(_, function)| generate_function(function, export_config, &type_map))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| eprintln!("Error generating functions: {e:?}"))
                .unwrap_or_default()
                .join(", \n");

            format!(r#""{path}": {{{functions}}}"#)
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
        .map(|(name, typ)| {
            ts::datatype(
                export_config,
                &FunctionResultVariant::Value(typ.clone()),
                type_map,
            )
            .map(|ty| format!("{}: {}", name.to_lower_camel_case(), ty))
        })
        .collect::<Result<Vec<_>, _>>()
        .context("An error occured while generating command args")?
        .join(", ");

    let return_ty = match function.result() {
        Some(FunctionResultVariant::Value(t)) => ts::datatype(
            export_config,
            &FunctionResultVariant::Value(t.clone()),
            type_map,
        )?,
        // TODO: handle result types
        Some(FunctionResultVariant::Result(t, _e)) => ts::datatype(
            export_config,
            &FunctionResultVariant::Value(t.clone()),
            type_map,
        )?,
        None => "void".to_string(),
    };

    let name = function.name().split_once("_taurpc_fn__").unwrap().1;
    Ok(format!(r#"{name}: ({args}) => Promise<{return_ty}>"#))
}

fn try_write(file: &mut File, data: &str) {
    match file.write_all(data.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error writing to file: {e:?}");
        }
    };
}
