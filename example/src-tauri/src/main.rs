#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let res = taurpc::create_example_defs().unwrap();

    tauri::Builder::default()
        // .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
