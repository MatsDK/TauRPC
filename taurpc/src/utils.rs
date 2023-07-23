use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;

static PACKAGE_JSON: &'static str = r#"
{
    "name": ".taurpc",
    "types": "index.ts"
}
"#;

static BOILERPLATE_TS_CODE: &'static str = r#"
import { createTauRPCProxy as createProxy } from "taurpc"

type Router = {
	root: [TauRpcInputs, TauRpcOutputs]
}

export const createTauRPCProxy = () => createProxy<Router>()
"#;

/// Export the generated TS types with the code necessary for generating the client proxy.
///
/// By default, if the `export_to` attribute was not specified on the procedures macro, it will be exported
/// to `node_modules/.taurpc` and a `package.json` will also be generated to import the package.
/// Otherwise the code will just be export to the .ts file specified by the user.
pub fn export_files(export_path: &str) {
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

    file.write_all(BOILERPLATE_TS_CODE.as_bytes()).unwrap();

    if export_path.ends_with("node_modules\\.taurpc\\index.ts") {
        let package_json_path = Path::new(export_path)
            .parent()
            .and_then(|path| Some(path.join("package.json")))
            .unwrap();

        std::fs::write(package_json_path, &PACKAGE_JSON).unwrap();
    }
}
