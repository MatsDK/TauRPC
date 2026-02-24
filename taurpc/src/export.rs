use anyhow::{Context, Result, bail};
use heck::ToLowerCamelCase;
use specta::TypeCollection;
use specta::datatype::{DataType, Field, Function, FunctionReturnType, Reference, Struct};
use specta_typescript::{self as ts, define, primitives};
use specta_typescript::{Error, Typescript};
use std::borrow::Cow;
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
    type_map: TypeCollection,
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

    let types = export_config
        .export(&type_map)
        .context("Failed to generate types with specta")?;

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .context("Cannot open bindings file")?;

    try_write(&mut file, &types);
    try_write(&mut file, BOILERPLATE_TS_IMPORT);

    try_write(
        &mut file,
        &format!(
            "const ARGS_MAP = {}\n",
            serde_json::to_string_pretty(&args_map).expect("argument map is not valid")
        ),
    );
    let functions_router = generate_functions_router(functions, type_map, &export_config)
        .context("Failed to generate router type")?;
    try_write(&mut file, &functions_router);
    try_write(&mut file, BOILERPLATE_TS_EXPORT);

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(&export_path)
            .parent()
            .map(|path| path.join("package.json"))
            .context("Failed to create 'package.json' path")?;

        std::fs::write(package_json_path, PACKAGE_JSON)
            .context("failed to create 'package.json'")?;
    }

    Ok(())
}

fn generate_functions_router(
    functions: BTreeMap<String, Vec<Function>>,
    type_map: TypeCollection,
    export_config: &Typescript,
) -> std::result::Result<String, Error> {
    let mut router = Struct::named();

    for (path, path_functions) in &functions {
        let mut function_names_and_funcs: Vec<_> =
            path_functions.iter().map(|f| (f.name(), f)).collect();
        function_names_and_funcs.sort_by(|a, b| a.0.cmp(b.0));

        let mut path_router = Struct::named();
        for (_, function) in function_names_and_funcs {
            let (name, field) = generate_function_field(function, export_config, &type_map)?;
            path_router = path_router.field(name, field);
        }

        router = router.field(path.clone(), Field::new(path_router.build()));
    }

    let router_type = primitives::inline(export_config, &type_map, &router.build())?;
    Ok(format!("export type Router = {router_type};\n"))
}

fn generate_function_field(
    function: &Function,
    export_config: &Typescript,
    type_map: &TypeCollection,
) -> std::result::Result<(String, Field), Error> {
    let args = function
        .args()
        .iter()
        .map(|(name, typ)| {
            render_reference_dt(typ, export_config, type_map)
                .map(|ty| format!("{}: {ty}", name.to_lower_camel_case()))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join(", ");

    let return_ty = match function.result() {
        Some(FunctionReturnType::Value(t)) => render_reference_dt(t, export_config, type_map)?,
        Some(FunctionReturnType::Result(t, _e)) => render_reference_dt(t, export_config, type_map)?,
        None => "void".to_string(),
    };

    let name = function.name().split_once("_taurpc_fn__").unwrap().1;
    Ok((
        name.to_string(),
        Field::new(define(format!("({args}) => Promise<{return_ty}>")).into()),
    ))
}

// Render a `DataType` as a reference (or fallback to inline).
// Also handles Tauri channel references.
fn render_reference_dt(
    dt: &DataType,
    exporter: &Typescript,
    types: &TypeCollection,
) -> Result<String, Error> {
    if let DataType::Reference(Reference::Named(r)) = dt
        && let Some(ndt) = r.get(types)
        && ndt.name() == "TAURI_CHANNEL"
        && ndt.module_path().starts_with("tauri::")
    {
        let generic = if let Some((_, dt)) = r.generics().first() {
            match &dt {
                DataType::Reference(r) => primitives::reference(exporter, types, r)?,
                dt => primitives::inline(exporter, types, dt)?,
            }
            .into()
        } else {
            Cow::Borrowed("never")
        };
        Ok(format!("(response: {generic}) => void"))
    } else {
        match &dt {
            DataType::Reference(r) => primitives::reference(exporter, types, r),
            dt => primitives::inline(exporter, types, dt),
        }
    }
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
