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
use std::path::PathBuf;

use common::fixture_rows;
use gp100_core::preset::{escape_value, Document};
use gp100_core::session::StatePage;

/// O `all.prst` — o GROUND TRUTH do nome. Mesmo arranjo de `roundtrip_prst.rs`
/// (CARGO_MANIFEST_DIR sobendo 2 níveis até a raiz do repo).
fn all_prst() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop(); // packages/
    root.pop(); // raiz do repo
    root.push("files");
    root.push("patches");
    root.push("all.prst");
    root
}

/// Os 99 nomes de fábrica, na ordem do documento.
fn nomes_de_fabrica() -> Vec<String> {
    let bytes = std::fs::read(all_prst()).expect("all.prst existe");
    let doc = Document::parse(&bytes).expect("dialeto válido");
    doc.presets()
        .map(|p| p.pp_name().unwrap_or("").to_string())
        .collect()
}

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

/// **O teste que a #155 pede: 198/198, e o ground truth é o artefato.**
///
/// A chave é `pp & 0xFF`: o aparelho tem dois bancos (0x0000..0x0062 e
/// 0x0100..0x0162) e ambos apontam para os mesmos 99 presets do documento —
/// foi assim que a análise medida achou 198/198, e este teste é quem prova.
/// Um caso só não bastaria: se o offset estivesse errado mas casasse por
/// acaso nos primeiros presets, o teste passaria e o bug ia embutido.
///
/// **Comparação no mesmo espaço:** o fio traz o nome literal (`Dub&Vibe`) e
/// o `.prst` guarda a forma escapada (`Dub&amp;Vibe`) — o parser mantém o
/// valor BRUTO, então o ground truth é escapado e o lido é literal. Usar
/// [`escape_value`] (a inversa que o repo já define) em vez de comparar
/// crudo contra escapado evitaria 2 divergências FALSAS sem afrouxar nada:
/// qualquer outra diferença continua estourando.
#[test]
fn nome_bate_com_o_all_prst_em_198_de_198() {
    let esperados = nomes_de_fabrica();
    assert_eq!(esperados.len(), 99, "all.prst tem 99 presets");
    assert!(
        esperados.iter().all(|n| !n.is_empty()),
        "nome vazio no artefato"
    );

    let (mapa, _) = paginas_da_fixture();
    let mut ok = 0;
    for (pp, pags) in &mapa {
        let dec = gp100_core::preset_pages::decode(pags).expect("decode");
        let lido = dec.nome().unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        let idx = usize::from(*pp & 0xFF);
        assert_eq!(
            escape_value(lido),
            esperados[idx],
            "pp {pp:#06x} (indice {idx}) divergiu do all.prst"
        );
        ok += 1;
    }
    assert_eq!(ok, 198, "todos os 198 pps decodificaram o nome certo");
}

// ---------------------------------------------------------------------------
// Prova-negativa: o decode tem de RECUSAR onde não deve.
//
// Sem isto, os 3 testes acima só provam que o caminho feliz funciona. Um
// parser que aceita qualquer coisa também passaria neles.
// ---------------------------------------------------------------------------

/// Monta um `StatePage` no formato real: 4B header `[pp][00][00][PG]` + corpo.
fn pagina_sintetica(idx: u8, corpo_decodificado: &[u8]) -> StatePage {
    let mut raw = vec![0x00, 0x00, 0x00, idx];
    for &b in corpo_decodificado {
        raw.push(b >> 4); // nibble-expande: o fio guarda pares
        raw.push(b & 0x0F);
    }
    StatePage { raw }
}

/// 9 páginas estruturalmente válidas com corpos nos tamanhos REAIS
/// (192B cru = 96B decodificados nas 0..7; 28B cru = 14B na 8).
fn nove_paginas_validas() -> [StatePage; 9] {
    std::array::from_fn(|i| {
        let dec = vec![0u8; if i < 8 { 96 } else { 14 }];
        pagina_sintetica(i as u8, &dec)
    })
}

#[test]
fn recusa_tamanho_errado() {
    let mut pags = nove_paginas_validas();
    pags[3].raw.pop();
    let e = gp100_core::preset_pages::decode(&pags).expect_err("195B não é 196B");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::TamanhoInesperado { pagina: 3, .. }
        ),
        "{e}"
    );
}

#[test]
fn recusa_pagina_fora_de_ordem() {
    let mut pags = nove_paginas_validas();
    pags[5].raw[3] = 7; // o header diz 7, a posição é 5
    let e = gp100_core::preset_pages::decode(&pags).expect_err("PG fora de ordem");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::PaginaForaDeOrdem {
                esperada: 5,
                achada: 7
            }
        ),
        "{e}"
    );
}

#[test]
fn recusa_byte_que_nao_e_nibble() {
    let mut pags = nove_paginas_validas();
    pags[2].raw[4] = 0xA5; // primeiro byte do corpo cru
    let e = gp100_core::preset_pages::decode(&pags).expect_err("0xA5 não é nibble");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::NibbleInvalido {
                pagina: 2,
                offset: 0,
                got: 0xA5
            }
        ),
        "{e}"
    );
}

/// **Review Focus 2:** o nome não pode ser truncado no primeiro NUL.
#[test]
fn recusa_nome_com_nul_interno() {
    // pg0 decodificada: pp 00 00 | "AB\0CD" + pad NUL | cadeia de zeros.
    let mut dec = vec![0u8; 96];
    dec[0] = 0;
    dec[1] = 0;
    dec[2] = b'A';
    dec[3] = b'B';
    dec[4] = 0; // primeiro NUL = fim do nome
    dec[5] = b'C'; // ...mas tem lixo DEPOIS dele
    dec[6] = b'D';

    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, &dec);
    let pag = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    let e = pag.nome().expect_err("tem NUL interno com lixo");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::NomeInvalido { .. }
        ),
        "{e}"
    );
}

#[test]
fn recusa_nome_vazio() {
    let dec = vec![0u8; 96]; // pp 00 00, nome todo NUL
    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, &dec);
    let pag = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    let e = pag.nome().expect_err("nome vazio");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::NomeInvalido { .. }
        ),
        "{e}"
    );
}

#[test]
fn recusa_nome_com_byte_nao_imprimivel() {
    let mut dec = vec![0u8; 96];
    dec[2] = b'"'; // 0x22 é imprimível... use um que não é
    dec[3] = 0x07; // BEL — não é 0x20..=0x7E
    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, &dec);
    let pag = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    let e = pag.nome().expect_err("0x07 não é imprimível");
    assert!(
        matches!(
            e,
            gp100_core::preset_pages::DecodeError::NomeInvalido { .. }
        ),
        "{e}"
    );
}

#[test]
fn aceita_nome_com_pad_nul_normal() {
    let mut dec = vec![0u8; 96];
    let nome = b"Blink OD";
    dec[..2].copy_from_slice(&[0x01, 0x00]); // pp
    dec[2..2 + nome.len()].copy_from_slice(nome);
    // o resto de 2..14 já é NUL (pad) — e a cadeia 14..32 é zeros = vazia
    let mut pags = nove_paginas_validas();
    pags[0] = pagina_sintetica(0, &dec);
    let pag = gp100_core::preset_pages::decode(&pags).expect("estrutura válida");
    assert_eq!(pag.nome().expect("válido"), "Blink OD");
}

// ---------------------------------------------------------------------------
// Fase 2: o BoardView reconstruído das páginas tem de ser o do artefato.
// ---------------------------------------------------------------------------

use gp100_core::model::Dictionary;
use gp100_core::pedalboard::{board_view_for, BoardView};

/// O mesmo valor em representações diferentes.
///
/// O `.prst` guarda a string como o gravador a escreveu (`"9.18355e-41"`) e o
/// fio guarda o f32 (`0.0000…91835` em notação decimal). São o MESMO número:
/// comparar string aqui marcaria divergência por forma, não por conteúdo — e
/// seria um falso positivo escondendo os divergentes de verdade.
///
/// O que continua sendo pego: valores realmente diferentes (`48` vs `35`,
/// `63` vs `50`), porque aí os dois parseiam e não são iguais.
fn mesmo_valor(a: &str, b: &str) -> bool {
    match (a.parse::<f32>(), b.parse::<f32>()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// O `BoardView` produzido do ARTEFATO — o ground truth.
///
/// **Por que a conversão:** `board_view_for` casa `pp` contra `ppID` com
/// `u16::from_str_radix(s, 16)` (a mesma expressão de `apenas_preset`, ver
/// `preset.rs:575`). O `ppID` do arquivo vem como string decimal do índice
/// (`"0"`..`"98"`), então o alvo que ele entende NAO e o pp do aparelho: para
/// o índice 10 o alvo e `from_str_radix("10", 16)` = `0x10`, e o pp do
/// aparelho e `0x000a`. Reaproveito a expressao do proprio repo em vez de
/// reimplementar a construcao do board — o que se compara aqui e o SLOT, nao
/// o numero do preset.
fn board_de_fabrica(doc: &Document, dict: &Dictionary, idx: usize) -> BoardView {
    let preset = doc.presets().nth(idx).expect("preset existe no all.prst");
    let alvo = u16::from_str_radix(preset.pp_id().expect("ppID presente"), 16).expect("ppID hex");
    board_view_for(doc, dict, Some(alvo)).expect("board do all.prst")
}

/// **Fase 2: paridade estrutural + prova de que o OFFSET está certo.**
///
/// O que este teste separa, e por quê:
///
/// * `slot`/`code`/`state`/identidade visual — tem de bater **198/198**.
///   Se um offset estivesse errado, o `code` de um slot viraria o de outro e
///   isto estouraria em massa.
/// * `knobs[].value` — comparado como NÚMERO ([`mesmo_valor`]), não como
///   string, porque o `.prst` e o fio usam formas diferentes para o mesmo f32.
///   Medimos 26552/26554 (99.99%) na análise, e as 2 divergências são valores
///   editados no hardware depois do dump (`48` no fio vs `35` no arquivo,
///   `63` vs `50`), nos dois bancos. Exigir igualdade total seria exigir que
///   o usuário nunca editasse. O número é fixado no assert — passar de 2
///   significaria OFFSET errado, porque aí os valores errariam em massa.
#[test]
fn slots_das_paginas_batem_com_o_all_prst() {
    let bytes = std::fs::read(all_prst()).expect("all.prst existe");
    let doc = Document::parse(&bytes).expect("dialeto válido");
    let dict =
        Dictionary::from_json(gp100_core::model::DICTIONARY_JSON).expect("dicionário válido");
    let off = gp100_core::preset_pages::Offsets::carregado().expect("artefato embutido");

    let (mapa, _) = paginas_da_fixture();
    let mut pps = 0;
    let mut divergencias_de_valor = 0usize;
    let mut detalhe: Vec<String> = Vec::new();

    for (pp, pags) in &mapa {
        let idx = usize::from(*pp & 0xFF);
        let dec = gp100_core::preset_pages::decode(pags).expect("decode");
        let lido = dec
            .slots(*pp, &dict, &off)
            .unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        let esp = board_de_fabrica(&doc, &dict, idx);

        assert_eq!(
            lido.slots.len(),
            esp.slots.len(),
            "pp {pp:#06x} nº de slots"
        );
        for (a, b) in lido.slots.iter().zip(esp.slots.iter()) {
            assert_eq!(a.slot, b.slot, "pp {pp:#06x} posição do slot");
            assert_eq!(
                a.code, b.code,
                "pp {pp:#06x} effectCode da posição {} — offset errado",
                b.slot
            );
            assert_eq!(
                a.state, b.state,
                "pp {pp:#06x} effectState da posição {} — offset errado",
                b.slot
            );
            assert_eq!(
                a.family, b.family,
                "pp {pp:#06x} família do slot {}",
                b.slot
            );
            assert_eq!(
                a.archetype, b.archetype,
                "pp {pp:#06x} arquétipo do slot {}",
                b.slot
            );
            assert_eq!(a.name, b.name, "pp {pp:#06x} nome do slot {}", b.slot);
            assert_eq!(
                a.variant, b.variant,
                "pp {pp:#06x} variante do slot {}",
                b.slot
            );
            assert_eq!(
                a.knobs.len(),
                b.knobs.len(),
                "pp {pp:#06x} nº de knobs do slot {}",
                b.slot
            );
            for (ka, kb) in a.knobs.iter().zip(b.knobs.iter()) {
                assert_eq!(ka.name, kb.name, "pp {pp:#06x} nome do knob");
                assert_eq!(ka.pos, kb.pos, "pp {pp:#06x} pos do knob");
                assert_eq!(ka.kind, kb.kind, "pp {pp:#06x} kind do knob");
                assert_eq!(ka.range, kb.range, "pp {pp:#06x} faixa do knob");
                assert_eq!(ka.options, kb.options, "pp {pp:#06x} opções do knob");
                match (&ka.value, &kb.value) {
                    (Some(va), Some(vb)) if mesmo_valor(va, vb) => {}
                    (Some(va), Some(vb)) => {
                        divergencias_de_valor += 1;
                        detalhe.push(format!(
                            "pp {pp:#06x} pos {} knob {:?}: fio={va} arquivo={vb}",
                            b.slot, kb.name
                        ));
                    }
                    _ => panic!(
                        "pp {pp:#06x} slot {} knob {}: value lido={:?} arquivo={:?}",
                        b.slot, kb.name, ka.value, kb.value
                    ),
                }
            }
        }
        pps += 1;
    }

    assert_eq!(pps, 198, "todos os pps comparados");
    assert!(
        divergencias_de_valor <= 2,
        "{} valores divergentes (maximo 2 medidos) — \
         mais que isso seria offset errado, nao valor editado no aparelho\n{}",
        divergencias_de_valor,
        detalhe.join("\n")
    );
}
