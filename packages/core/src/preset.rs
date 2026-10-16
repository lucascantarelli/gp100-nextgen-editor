//! preset — modelo do arquivo `.prst` com round-trip **byte-idêntico**
//! (regra R4 é sagrada: abrir→salvar sem editar = mesmos bytes).
//!
//! **Estratégia de layout (por que não um writer XML genérico):** os arquivos
//! foram serializados pelo Suite com quebras de linha DENTRO das tags
//! (ex.: `<preset_info … platform="WINDOWS"\r\n               time="…"/>`)
//! e indentação própria. Reproduzir isso com um writer genérico exigiria
//! adivinhar o algoritmo de wrap — chute proibido (R1). Em vez disso, o
//! parser REGISTRA o layout observado de cada elemento (ordem dos atributos,
//! quais atributos iniciam linha nova e o indent da continuação) e o writer
//! o reproduz. Layout é DADO, não algoritmo.
//!
//! **Dialecto suportado** (evidência: os 3 `.prst` de `files/patches/`):
//! `<?xml …?>` + linha em branco + `<GP-100>`; filhos sem texto livre
//! (só elementos); atributos com `="…"`, valores com `&amp;` etc. na forma
//! ESCAPADA (preservamos os bytes crus — decodificar é trabalho da UI/M1);
//! fim de linha CRLF. Qualquer coisa fora disso (comentário, CDATA, texto)
//! é rejeitada com `ProtocolError::InvalidShape` — strict de propósito.
//!
//! **Camadas:**
//! 1. [`Document`]/[`Element`] — DOM mínimo com layout preservado + writer;
//! 2. Views tipadas ([`PresetView`], [`EffectView`]) para o vocabulário
//!    `pp*`/`Effect`/`params_0..14` (§13.9), SEM esconder atributos
//!    desconhecidos (preservação obrigatória).
//!
//! **Testes:** `gp100-core/tests/roundtrip_prst.rs` — round-trip byte-a-byte
//! dos 3 arquivos reais + edição estável (modelo híbrido da skill
//! rust-practices).

mod elemento;
mod instantaneo;
mod leitura;
mod pp;
mod recorte;
mod view;

pub use elemento::{escape_value, Attr};
pub use instantaneo::{DocumentSnapshot, ElementSnapshot, Quebra};
pub use pp::{indice_do_documento, pp_id_decimal};
pub use view::{EffectView, PresetView};

use crate::ProtocolError;

/// Constrói `ProtocolError::InvalidShape` com posição do arquivo (byte offset).
fn shape(expected: impl Into<String>, got: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.into(),
        got: got.into(),
    }
}

/// Quebra de linha registrada pelo parser: o atributo `at_attr` inicia
/// linha nova com `cont_indent` espaços (o writer reproduz exatamente).
#[derive(Debug, Clone)]
struct LineBreak {
    /// Índice do atributo que começa a linha de continuação (>= 1).
    at_attr: usize,
    /// Espaços de indentação da linha de continuação.
    cont_indent: usize,
}

/// Elemento do dialecto `.prst` com layout preservado.
///
/// Campos privados de propósito: o acesso é por [`Element::attr`]/[`Element::set_attr`]
/// etc., para que as invariantes de layout (ordem/quebras) nunca sejam
/// quebradas por quem edita.
#[derive(Debug, Clone)]
pub struct Element {
    name: String,
    /// Whitespace bruto ANTES do `<` (inclui `\r\n` e indent — reproduzido).
    pre_ws: Vec<u8>,
    attrs: Vec<Attr>,
    breaks: Vec<LineBreak>,
    self_closing: bool,
    children: Vec<Element>,
    /// Whitespace antes de `</nome>` (só quando `!self_closing`).
    close_pre_ws: Vec<u8>,
}

/// Documento `.prst` completo: declaração XML + `<GP-100>` + whitespace final.
#[derive(Debug, Clone)]
pub struct Document {
    /// Bytes crus da declaração `<?xml …?>`.
    decl: Vec<u8>,
    /// Raiz `<GP-100>` (pre_ws inclui a linha em branco pós-declaração).
    root: Element,
    /// Whitespace após `</GP-100>` (o `\r\n` final).
    trailing: Vec<u8>,
}

impl Document {
    /// Serializa de volta. Sem edições, o resultado é BYTE-IDÊNTICO ao
    /// original (contrato R4 — provado em `tests/roundtrip_prst.rs`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.root.pre_ws.len() + self.decl.len() * 2);
        out.extend_from_slice(&self.decl);
        self.root.write(&mut out);
        out.extend_from_slice(&self.trailing);
        out
    }

    /// Raiz `<GP-100>` (leitura).
    pub fn root(&self) -> &Element {
        &self.root
    }

    /// Raiz `<GP-100>` (edição).
    pub fn root_mut(&mut self) -> &mut Element {
        &mut self.root
    }

    /// O `<preset_info …/>` (metadados do arquivo: software/firmware/count…).
    pub fn preset_info(&self) -> Option<&Element> {
        self.root.child("preset_info")
    }

    /// Todos os blocos `<presets>` (um por preset; all.prst tem 99).
    pub fn presets(&self) -> impl Iterator<Item = PresetView<'_>> {
        self.root
            .children()
            .iter()
            .filter(|c| c.name == "presets")
            .map(|el| PresetView { el })
    }
}
