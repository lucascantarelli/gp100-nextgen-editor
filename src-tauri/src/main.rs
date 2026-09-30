//! Binário do GP-100 NextGen Editor (shell Tauri); toda a lógica vive na
//! lib (`gp100_ui_lib`) para os testes cobrirem os commands sem subir janela.
//!
//! # Panics
//! `run()` propaga erro fatal de setup/runtime — exit code ≠ 0 via `std::process::exit`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = gp100_ui_lib::run() {
        eprintln!("[!] falha fatal no app: {e}");
        std::process::exit(1);
    }
}
