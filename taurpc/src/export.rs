use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

static PACKAGE_JSON: &'static str = r#"
{
    "name": ".taurpc",
    "types": "index.ts"
}
"#;

static BOILERPLATE_TS_CODE: &'static str = r#"
import { createTauRPCProxy as createProxy } from "taurpc"

export const createTauRPCProxy = () => createProxy<Router>(ARGS_MAP)
"#;

/// Export the generated TS types with the code necessary for generating the client proxy.
///
/// By default, if the `export_to` attribute was not specified on the procedures macro, it will be exported
/// to `node_modules/.taurpc` and a `package.json` will also be generated to import the package.
/// Otherwise the code will just be export to the .ts file specified by the user.
pub(super) fn export_types(
    export_path: Option<String>,
    handlers: Vec<(String, String)>,
    args_map: String,
) {
    let export_path =
        export_path.unwrap_or(generate_default_export_path().to_str().unwrap().to_string());
    let export_path = export_path.as_str();

    let path = Path::new(export_path);
    if path.is_dir() {
        panic!("`export_to` path should be a ts file");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    specta::export::ts(export_path).unwrap();

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .unwrap();

    file.write_all(format!("const ARGS_MAP = {}", args_map).as_bytes())
        .unwrap();
    file.write_all(BOILERPLATE_TS_CODE.as_bytes()).unwrap();
    file.write_all(generate_router_type(handlers).as_bytes())
        .unwrap();

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(export_path)
            .parent()
            .and_then(|path| Some(path.join("package.json")))
            .unwrap();

        std::fs::write(package_json_path, &PACKAGE_JSON).unwrap();
    }
}

fn generate_router_type(handlers: Vec<(String, String)>) -> String {
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

// // Generate the default path for exporting the types: `node_modules/.taurpc/index.ts`
// fn generate_default_export_path() -> PathBuf {
//     let path = std::env::current_dir()
//         .unwrap()
//         .parent()
//         .map(|p| p.join("node_modules\\.taurpc"));

//     match path {
//         Some(path) => path.join("index.ts"),
//         None => panic!("Export path not found"),
//     }
// }

fn generate_default_export_path() -> PathBuf {
    std::env::current_dir().unwrap().join("../bindings.ts")
}
