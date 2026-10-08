//! Contratos do decode das páginas 13xx (spec:
//! `docs/superpowers/specs/2026-10-08-decode-paginas-13xx-design.md` · #155).
//!
//! O falso positivo que estes testes fecham: "o parser não crasheou" não prova
//! que leu o layout certo. Aqui o dataset INTEIRO da captura é decodificado e
//! o nome é conferido contra o `all.prst` — a mesma prova byte-a-byte do
//! `roundtrip_prst.rs`, aplicada ao fio.
//!
//! **Por que o agrupamento é por `payload[3]` e não pelo PG do pedido.**
//! Medido no `boot.jsonl`: as 1783 respostas de `13010003` trazem o índice da
//! página em `payload[3]`, indo de 0 a 8; o pp vem em `payload[0..2]`. Um pp
//! tem 9 páginas (0..=7 @196B e 8 @32B), e o pp 0x0100 tem a pg0 **duas
//! vezes** (open duplicado do §13.4) — as duas cópias têm de ser idênticas.

mod common;

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use common::fixture_rows;
use gp100_core::session::StatePage;

/// Hex crua -> bytes (local de propósito: nenhum teste depende de assinatura
/// alheia para o caminho crítico).
fn hex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "hex com tamanho ímpar: {s}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("dígito hex válido"))
        .collect()
}

/// pp -> (pagina -> StatePage), das linhas `13010003` de `boot.jsonl`.
///
/// Retorna também a contagem CRUA: o quirk do §13.4 manda a pg0 do preset
/// atual duas vezes, então 1783 linhas têm de virar 1782 pares únicos —
/// e as duas cópias têm de ser IDÊNTICAS (Review Focus 1).
fn paginas_da_fixture() -> (BTreeMap<u16, [StatePage; 9]>, usize) {
    let mut brutas: BTreeMap<u16, BTreeMap<u8, Vec<u8>>> = BTreeMap::new();
    let mut n_linhas = 0usize;
    for r in fixture_rows("boot.jsonl") {
        let dir = r["dir"].as_str().unwrap_or("");
        let addr = r["addr"].as_str().unwrap_or("");
        if addr != "13010003" || dir != "in" {
            continue;
        }
        n_linhas += 1;
        let bytes = hex(r["data"].as_str().expect("data é hex"));
        assert!(bytes.len() >= 4, "frame curto: {}", bytes.len());
        let pp = u16::from_be_bytes([bytes[0], bytes[1]]);
        let pg = bytes[3];
        match brutas.entry(pp).or_default().entry(pg) {
            Entry::Vacant(e) => {
                e.insert(bytes);
            }
            Entry::Occupied(e) => {
                // DUPLICATA é esperada (quirk), mas divergente é corrupção.
                assert_eq!(
                    e.get(),
                    &bytes,
                    "pp {pp:#06x} pg {pg}: as duas leituras divergem"
                );
            }
        }
    }
    let mut saida = BTreeMap::new();
    for (pp, pags) in brutas {
        assert_eq!(pags.len(), 9, "pp {pp:#06x} tem {} páginas", pags.len());
        let arr: [StatePage; 9] = std::array::from_fn(|i| StatePage {
            raw: pags[&(i as u8)].clone(),
        });
        saida.insert(pp, arr);
    }
    (saida, n_linhas)
}

#[test]
fn dataset_inteiro_agrupa_em_198_preset_x_9_paginas() {
    let (mapa, n_linhas) = paginas_da_fixture();
    assert_eq!(mapa.len(), 198, "198 pps (banco 0x00xx + 0x01xx)");
    assert_eq!(n_linhas, 1783, "1782 pares únicos + 1 duplicata do §13.4");
    for (pp, pags) in &mapa {
        for (i, p) in pags.iter().enumerate().take(8) {
            assert_eq!(p.raw.len(), 196, "pp {pp:#06x} pg {i} (4B header + 192B)");
        }
        assert_eq!(pags[8].raw.len(), 32, "pp {pp:#06x} pg 8 (4B header + 28B)");
    }
}

#[test]
fn decode_devolve_8x96_mais_1x14_para_todo_preset() {
    let (mapa, _) = paginas_da_fixture();
    for (pp, pags) in &mapa {
        let dec =
            gp100_core::preset_pages::decode(pags).unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        for i in 0..8usize {
            assert_eq!(dec.corpo(i).len(), 96, "pp {pp:#06x} pg {i} decodificado");
        }
        assert_eq!(dec.corpo(8).len(), 14, "pp {pp:#06x} pg 8 decodificado");
    }
}
