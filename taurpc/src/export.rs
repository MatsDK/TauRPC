use itertools::Itertools;
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
    handlers: Vec<(&'static str, &'static str)>,
    args_map: HashMap<String, String>,
    export_config: specta_typescript::Typescript,
) {
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

    // Put headers always at the top of the file, followed by the module imports.
    let framework_header = export_config.framework_header.as_ref();
    let body = types.split_once(framework_header).unwrap().1;

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .unwrap();

    file.write_all(export_config.header.as_bytes()).unwrap();
    file.write_all(framework_header.as_bytes()).unwrap();
    file.write_all(BOILERPLATE_TS_IMPORT.as_bytes()).unwrap();
    file.write_all(body.as_bytes()).unwrap();

    let args_entries: String = args_map
        .iter()
        .map(|(k, v)| format!("'{}':'{}'", k, v))
        .join(", ");
    let router_args = format!("{{{}}}", args_entries);

    file.write_all(format!("const ARGS_MAP = {}", router_args).as_bytes())
        .unwrap();
    file.write_all(generate_router_type(handlers).as_bytes())
        .unwrap();
    file.write_all(BOILERPLATE_TS_EXPORT.as_bytes()).unwrap();

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(&export_path)
            .parent()
            .map(|path| path.join("package.json"))
            .unwrap();

        std::fs::write(package_json_path, PACKAGE_JSON).unwrap();
    }

    // Format the output file if the user specified a formatter on `export_config`.
    if export_config.formatter.is_some() {
        match export_config.format(path) {
            Ok(_) => println!("Bindings file formatted successfully!"),
            Err(e) => eprintln!("Error formatting bindings file: {}", e),
        }
    }
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
