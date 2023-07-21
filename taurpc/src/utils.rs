use std::fs::OpenOptions;
use std::io::prelude::*;

static BOILERPLATE_TS_CODE: &'static str = r#"
import { createTauRPCProxy as createProxy } from "taurpc"

type Router = {
	root: [TauRpcInputs, TauRpcOutputs]
}

export const createTauRPCProxy = () => createProxy<Router>()
"#;

/// Create the `.taurpc` folder and export types generated using `ts_rs` to `.taurpc/index.ts`,
/// generate a `package.json` so the types can be imported on the frontend.
pub fn export_files(export_path: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(export_path)
        .unwrap();

    file.write_all(BOILERPLATE_TS_CODE.as_bytes()).unwrap();

    // if let Err(e) = writeln!(file, code) {
    //     eprintln!("Couldn't write to file: {}", e);
    // }
    // let (ts_path, package_json_path) = generate_export_paths();

    // if let Some(parent) = ts_path.parent() {
    //     std::fs::create_dir_all(parent).unwrap();
    // }
    // std::fs::write(ts_path, &ts_types).unwrap();

    // std::fs::write(package_json_path, &PACKAGE_JSON).unwrap();
}
