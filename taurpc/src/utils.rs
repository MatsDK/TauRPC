use std::{env, path::PathBuf};

static PACKAGE_JSON: &'static str = r#"
{
	"name": ".taurpc",
	"main": "index.js",
	"types": "index.ts"
}
"#;

pub fn export_files(ts_types: String) {
    let (ts_path, package_json_path) = generate_export_paths();

    if let Some(parent) = ts_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(ts_path, &ts_types).unwrap();

    std::fs::write(package_json_path, &PACKAGE_JSON).unwrap();
}

fn generate_export_paths() -> (PathBuf, PathBuf) {
    let path = env::current_dir()
        .unwrap()
        .parent()
        .map(|p| p.join("node_modules\\.taurpc"));

    match path {
        Some(path) => {
            let ts_path = path.join("index.ts").to_path_buf();
            let package_json_path = path.join("package.json").to_path_buf();

            (ts_path, package_json_path)
        }
        None => panic!("Export path not found"),
    }
}
