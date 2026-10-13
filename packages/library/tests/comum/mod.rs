#![allow(dead_code)]
//! Ajudantes compartilhados pelos testes do crate.
//!
//! Cada arquivo de teste declara `mod comum;` e usa um pedaço diferente do
//! módulo — sem este `allow`, o que sobrou ficaria com aviso de codigo morto.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static CONTADOR: AtomicU32 = AtomicU32::new(0);

/// Caminho de um arquivo de banco que não existe ainda e é apagado no fim.
///
/// Vai para `target/tmp` e não para `/tmp`: o build já tem esse diretório em
/// todas as plataformas, e um teste que só roda no Linux não é um teste.
pub fn caminho_temporario(nome: &str) -> PathBuf {
    let n = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("o crate tem pai")
        .parent()
        .expect("packages tem pai")
        .join("target")
        .join("tmp");
    std::fs::create_dir_all(&dir).expect("cria target/tmp");
    let p = dir.join(format!("teste-{nome}-{}-{n}.db", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

/// Caminho + já apaga o arquivo no drop do teste. Menos ruído no corpo.
pub struct ArquivoTemporario(pub PathBuf);

impl ArquivoTemporario {
    pub fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ArquivoTemporario {
    fn drop(&mut self) {
        // O SQLite deixa `-wal` e `-shm` ao lado; limpamos os três senão o
        // diretório de teste enche a cada execução.
        let _ = std::fs::remove_file(&self.0);
        for sufixo in ["-wal", "-shm"] {
            let mut p = self.0.clone();
            p.set_file_name(format!(
                "{}{sufixo}",
                self.0.file_name().unwrap().to_string_lossy()
            ));
            let _ = std::fs::remove_file(p);
        }
    }
}

/// [`caminho_temporario`] com limpeza automática.
pub fn arquivo_temporario(nome: &str) -> ArquivoTemporario {
    ArquivoTemporario(caminho_temporario(nome))
}

/// Registro de fábrica pronto para gravar (o que o seed produz).
pub fn preset_de_fabrica(pp: u16, nome: &str, pp_type: u16, rotulo: &str) -> gp100_library::Preset {
    gp100_library::Preset {
        id: format!("f{pp}"),
        bank: gp100_library::Bank::Factory,
        pp: Some(pp),
        name: nome.to_string(),
        pp_type,
        pp_type_name: rotulo.to_string(),
        saved_at: "2026-01-01T00:00:00Z".to_string(),
        payload: None,
    }
}

/// Um slot de cadeia serializado no MESMO formato do palco (`BoardSlot[]`).
///
/// Os testes de versao/diff nao usam um JSON inventado: usam os campos que
/// `userPatches.ts` realmente grava (`slot`, `name`, `state`, `knobs[]` com
/// `pos`/`name`/`value`). Um helper que inventasse outro formato faria o teste
/// provar o diff contra um dado que o app nunca produz.
pub fn slot_json(
    slot: u32,
    nome: &str,
    ligado: bool,
    knobs: &[(u32, &str, Option<&str>)],
) -> String {
    let knobs: Vec<serde_json::Value> = knobs
        .iter()
        .map(|(pos, nome, valor)| {
            let mut o = serde_json::json!({ "pos": pos, "name": nome, "kind": "knob" });
            if let Some(v) = valor {
                o["value"] = serde_json::Value::String((*v).to_string());
            }
            o
        })
        .collect();
    serde_json::json!({
        "slot": slot,
        "family": "AMP",
        "archetype": "AMPLIFIER",
        "name": nome,
        "variant": nome.to_lowercase(),
        "state": ligado,
        "code": 1,
        "knobs": knobs,
    })
    .to_string()
}

/// Cadeia serializada a partir de slots.
pub fn cadeia_json(slots: &[String]) -> String {
    format!("[{}]", slots.join(","))
}

/// Registro de usuário pronto para gravar (patch do dono).
pub fn patch_de_usuario(id: &str, nome: &str, payload: &str) -> gp100_library::Preset {
    gp100_library::Preset {
        id: id.to_string(),
        bank: gp100_library::Bank::User,
        pp: None,
        name: nome.to_string(),
        pp_type: 4,
        pp_type_name: "Rock".to_string(),
        saved_at: "2026-02-02T00:00:00Z".to_string(),
        payload: Some(payload.to_string()),
    }
}
