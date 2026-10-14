//! o que o mock PROJETA: do template do golden para a resposta no fio.

use super::shape_err;
use crate::golden::GoldenFile;
use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// O template `push` do endpoint (lado IN), se houver. Casa por ADDR: os
/// pushes do golden têm IN func 12, mas os READS chegam com func 11 — a
/// resposta deles é o push do MESMO endereço (func 12, como no fio).
pub(super) fn push_template(
    golden: &GoldenFile,
    addr: [u8; 4],
) -> Result<&crate::golden::Template, ProtocolError> {
    golden
        .templates()
        .iter()
        .find(|t| {
            t.template_type == "push"
                && t.in_
                    .as_ref()
                    .is_some_and(|e| e.parsed().map(|(_, a)| a).is_ok_and(|a| a == addr))
        })
        .ok_or_else(|| shape_err("template push no golden", &format!("addr={addr:02x?}")))
}

/// Bytes do exemplo congelado do LADO IN (response_hex/hex).
fn example_bytes(tpl: &crate::golden::Template) -> Vec<u8> {
    let ex = &tpl.example;
    let hex = ex
        .response_hex
        .as_deref()
        .or(ex.hex.as_deref())
        .unwrap_or("");
    crate::golden::hex_decode(hex).unwrap_or_default()
}

/// O `i`-ésimo segmento var do exemplo (posicional), redimensionado para
/// `count` (o exemplo É a evidência; state-first entra por outro caminho).
pub(super) fn example_var(
    tpl: &crate::golden::Template,
    len_hint: Option<usize>,
    i: usize,
    count: usize,
) -> Vec<u8> {
    let ex = example_bytes(tpl);
    let pat = tpl.response_pattern();
    let vars: Vec<Vec<u8>> = match pat {
        Some(p) => p
            .extract_vars(&ex)
            .map(|v| v.iter().map(|s| s.to_vec()).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    };
    // by-len: se o exemplo não casou (len ≠ sub do exemplo), tenta a partir
    // dos vars concatenados do sub-padrão pedido — simplificação: devolve
    // zeros para além do exemplo (shape do mock, documentado no módulo).
    match vars.get(i) {
        Some(v) => {
            let mut b = v.clone();
            b.resize(count, 0);
            b
        }
        None => {
            let _ = len_hint;
            vec![0u8; count]
        }
    }
}

/// Constrói a resposta do template com o exemplo congelado (`fill_custom`
/// substitui vars específicas — pp BE, echo de chunk).
pub(super) fn build_from_example(
    tpl: &crate::golden::Template,
    desired_len: Option<usize>,
    fill_custom: Option<&dyn Fn(usize, usize) -> Vec<u8>>,
) -> Result<Vec<u8>, ProtocolError> {
    build_with_fill(tpl, desired_len, &mut |i, count| match fill_custom {
        Some(f) => f(i, count),
        None => example_var(tpl, desired_len, i, count),
    })
}

/// Constrói a resposta com fill arbitrário (var indexado).
pub(super) fn build_with_fill(
    tpl: &crate::golden::Template,
    desired_len: Option<usize>,
    mut fill: impl FnMut(usize, usize) -> Vec<u8>,
) -> Result<Vec<u8>, ProtocolError> {
    tpl.build_response(&mut fill, desired_len)
}

/// Envelope SysEx completo (helper local sobre o header do crate).
pub(super) fn envelope_of(func: u8, addr: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(SYSEX_HEADER.len() + 5 + payload.len() + 1);
    m.extend_from_slice(&SYSEX_HEADER);
    m.push(func);
    m.extend_from_slice(&addr);
    m.extend_from_slice(payload);
    m.push(SYSEX_EOX);
    m
}
