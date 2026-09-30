//! Build script do Tauri: gera o contexto (capabilities, ícones, permissões)
//! a partir de `tauri.conf.json` + `capabilities/`.
fn main() {
    tauri_build::build()
}
