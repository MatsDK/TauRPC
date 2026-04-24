use anyhow::{Context, Result, bail};
use heck::ToLowerCamelCase;
use itertools::Itertools;
use specta::TypeCollection;
use specta::datatype::{Function, FunctionResultVariant};
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
    "types": "index.ts",
    "exports": {
        ".": "./index.ts",
        "./proxy": "./proxy.ts"
    }
}
"#;

static BINDINGS_PRELUDE: &str = r#"
type TAURI_CHANNEL<T> = (response: T) => void
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
        match std::fs::create_dir_all(parent) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to create directory for exported bindings: {:?}", e);
            }
        }
    }

    // Export `types_map` containing all referenced types.
    type_map.remove(<tauri::ipc::Channel<()> as specta::NamedType>::sid());
    let types = match export_config
        .export(&type_map)
        .context("Failed to generate types with specta")
    {
        Ok(types) => types,
        Err(e) => {
            println!("Failed to generate types with specta: {:?}", e);
            "".to_string()
        }
    };

    // Put headers always at the top of the file, followed by the module imports.
    let framework_header = export_config.framework_header.as_ref();
    let body = match types.split_once(framework_header) {
        Some((_, body)) => body,
        None => {
            println!(
                "Failed to split types with framework header\nbody will be empty string\ntaurpc will continue with router creation."
            );
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
    try_write(&mut file, BINDINGS_PRELUDE);
    try_write(&mut file, body);

    let args_entries: String = args_map
        .iter()
        .map(|(k, v)| format!("'{k}':'{v}'"))
        .join(", ");
    let router_args = format!("{{ {args_entries} }}");

    try_write(
        &mut file,
        &format!("export const ARGS_MAP = {router_args}\n"),
    );
    let functions_router = generate_functions_router(functions, type_map, &export_config);
    try_write(&mut file, &functions_router);

    let bindings_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("bindings path has no valid file stem")?;
    let proxy_path = path.with_file_name("proxy.ts");
    write_proxy_file(&proxy_path, bindings_stem, &export_config, framework_header)?;

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

    // Format the output files if the user specified a formatter on `export_config`.
    if export_config.formatter.is_some() {
        match export_config.format(path) {
            Ok(_) => (),
            Err(e) => println!("Error formatting bindings file: {}", e),
        }
        match export_config.format(&proxy_path) {
            Ok(_) => (),
            Err(e) => println!("Error formatting proxy file: {}", e),
        }
    }
    Ok(())
}

/// Write the runtime half of the generated output. `proxy.ts` imports `ARGS_MAP` and
/// `Router` from the sibling bindings file and exports `createTauRPCProxy`. Kept in its
/// own module so `bindings.ts` stays free of npm imports — Vite's optimizeDeps scanner
/// won't pre-bundle `@fltsci/taurpc` when a consumer only `import type`s from bindings.
fn write_proxy_file(
    proxy_path: &Path,
    bindings_stem: &str,
    export_config: &ts::Typescript,
    framework_header: &str,
) -> Result<()> {
    let mut proxy = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(proxy_path)
        .context("Cannot open proxy file")?;

    try_write(&mut proxy, &export_config.header);
    try_write(&mut proxy, framework_header);
    try_write(
        &mut proxy,
        &format!(
            "\nimport {{ createTauRPCProxy as createProxy }} from '@fltsci/taurpc'\n\
             import {{ ARGS_MAP, type Router }} from './{bindings_stem}'\n\n\
             export type {{ InferCommandOutput }} from '@fltsci/taurpc'\n\n\
             export const createTauRPCProxy = (): ReturnType<typeof createProxy<Router>> =>\n  \
             createProxy<Router>(ARGS_MAP)\n"
        ),
    );
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

            let functions = match function_names_and_funcs
                .iter()
                .map(|(_, function)| generate_function(function, export_config, &type_map))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(functions) => functions.join(", \n"),
                Err(e) => {
                    eprintln!("Error generating functions: {e:?}");
                    "".to_string()
                }
            };

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

    let name = match function.name().split_once("_taurpc_fn__") {
        Some(thing) => thing.1,
        None => return Err(anyhow::anyhow!("Function name is not valid")),
    };

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
