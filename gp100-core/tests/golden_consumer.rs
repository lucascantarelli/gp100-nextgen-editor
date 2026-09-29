//! Testes de CONTRATO do consumidor do golden — ROADMAP M0.3 (API revisada).
//!
//! Ângulo caixa-preta, com a API que a FSM (M0.6) vai usar: endpoints TIPADOS
//! `(func u8, addr 4B BE)` — a forma que o fio entrega — e desambiguação de
//! templates duplicados por nº de vars.
//!
//! ⚠️ Convenção do golden (build_golden.py): `example.hex`/`request_hex`/
//! `response_hex` são os PAYLOADS (data), NÃO o SysEx completo.

use gp100_core::golden::{hex_decode, GoldenFile};

/// Endereços recorrentes dos testes (u32 BE como no fio).
const ADDR_PAGE: [u8; 4] = [0x13, 0x01, 0x00, 0x04]; // página de preset
const ADDR_OPEN: [u8; 4] = [0x13, 0x01, 0x00, 0x02]; // abre/avança páginas
const ADDR_TABLE: [u8; 4] = [0x12, 0x00, 0x10, 0x02]; // IRs/resync/ACK
const ADDR_SELECT: [u8; 4] = [0x13, 0x01, 0x00, 0x00]; // select de preset
const ADDR_KEEPALIVE: [u8; 4] = [0x00, 0x02, 0x00, 0x01]; // ping

/// O golden embutido carrega com 40 templates (DoD M0.3) — a carga AGORA
/// valida coerência de lados por tipo e parse dos endpoints (hex 1B/4B).
#[test]
fn embedded_golden_loads_40_templates() {
    let g = GoldenFile::embedded().expect("golden válido (validação de carga passou)");
    assert_eq!(g.templates().len(), 40, "40 templates (baseline v1.0)");
    let (mut w, mut p, mut r) = (0, 0, 0);
    for t in g.templates() {
        match t.template_type.as_str() {
            "write" => w += 1,
            "push" => p += 1,
            "req" => r += 1,
            other => panic!("tipo inesperado: {other}"),
        }
    }
    assert_eq!((w, p, r), (22, 10, 8));
}

/// PROPRIEDADE FUNDAMENTAL para os 40 templates: exemplo (payload) → extract
/// → build == exemplo (byte a byte), via a API TIPADA (build pelo GoldenFile
/// com desambiguação — que NÃO pode quebrar a reconstrução dos duplicados).
#[test]
fn build_and_extract_are_exact_inverses_for_all_templates() {
    let g = GoldenFile::embedded().expect("golden válido");
    let (mut checked_req, mut checked_write, mut checked_push) = (0, 0, 0);

    for t in g.templates() {
        let label = t
            .semantic()
            .map(|s| s.chars().take(36).collect::<String>())
            .unwrap_or_else(|| t.template_type.clone());
        match t.template_type.as_str() {
            "req" => {
                let req = hex_decode(t.example.request_hex.as_deref().expect("request_hex"))
                    .expect("hex do request");
                let pat = t.request_pattern().expect("req tem request_payload");
                let vars: Vec<u8> = pat
                    .extract_vars(&req)
                    .unwrap_or_else(|| panic!("request não casa o próprio template: {label}"))
                    .concat();
                let (func, addr) = t.out.as_ref().unwrap().parsed().expect("endpoint OUT");
                // rota da FSM: resolve pelo endpoint + nº de vars e constrói
                let rebuilt = g
                    .build_request(func, &addr, &vars)
                    .expect("build via GoldenFile (desambiguação)");
                assert_eq!(
                    rebuilt.len(),
                    8 + 1 + 4 + req.len() + 1,
                    "envelope completo: {label}"
                );
                assert_eq!(&rebuilt[13..13 + req.len()], &req[..], "payload: {label}");
                assert_eq!(rebuilt.last(), Some(&0xF7), "EOX: {label}");
                assert_eq!(
                    &rebuilt[..8],
                    &[0xF0, 0x21, 0x25, 0x7F, 0x47, 0x50, 0x2D, 0x64]
                );
                assert_eq!(rebuilt[8], func, "func no envelope");

                // lado IN: resposta do exemplo casa o padrão de resposta
                let rsp = hex_decode(t.example.response_hex.as_deref().expect("response_hex"))
                    .expect("hex da resposta");
                assert!(
                    t.matches_response(&rsp).is_some(),
                    "resposta do exemplo não casa: {label}"
                );
                checked_req += 1;
            }
            "write" => {
                let req = hex_decode(t.example.hex.as_deref().expect("hex")).expect("hex");
                let pat = t.request_pattern().expect("write tem request_payload");
                let vars: Vec<u8> = pat
                    .extract_vars(&req)
                    .unwrap_or_else(|| panic!("write não casa o próprio template: {label}"))
                    .concat();
                let rebuilt = t.build_request(&vars).expect("build");
                assert_eq!(&rebuilt[13..13 + req.len()], &req[..], "{label}");
                assert_eq!(rebuilt.last(), Some(&0xF7));
                checked_write += 1;
            }
            "push" => {
                let data = hex_decode(t.example.hex.as_deref().expect("hex")).expect("hex");
                assert!(
                    t.matches_response(&data).is_some(),
                    "push não casa o próprio exemplo: {label}"
                );
                assert!(
                    t.build_request(&[]).is_err(),
                    "push NÃO deve ter build_request"
                );
                checked_push += 1;
            }
            other => panic!("tipo inesperado: {other}"),
        }
    }
    assert_eq!((checked_req, checked_write, checked_push), (8, 22, 10));
}

/// ACHADO estrutural virando CONTRATO: 3 endpoints OUT têm DOIS templates
/// (caso congelado do boot × caso geral). `request_template` desambigua por
/// nº de vars; nº sem template = erro; endpoint inexistente = erro.
#[test]
fn duplicate_out_endpoints_disambiguate_by_var_count() {
    let g = GoldenFile::embedded().expect("golden válido");

    // 12|13010002: congelado `01 00 01` (0 vars) × geral `[pp 2v] 01` (2 vars)
    let frozen = g
        .request_template(0x12, &ADDR_OPEN, 0)
        .expect("caso congelado");
    let general = g.request_template(0x12, &ADDR_OPEN, 2).expect("caso geral");
    assert_ne!(frozen.template_type, "@", "sanidade");
    assert_eq!(
        frozen.request_pattern().unwrap().expected_len(),
        Some(3),
        "congelado: payload 3B"
    );
    assert_eq!(
        general.request_pattern().unwrap().expected_len(),
        Some(3),
        "geral: também 3B (pp+01) — desambiguação é por VARS, não por len"
    );
    assert!(
        g.request_template(0x12, &ADDR_OPEN, 1).is_err(),
        "nº sem template"
    );
    assert!(
        g.request_template(0x12, &ADDR_OPEN, 3).is_err(),
        "nº sem template (2)"
    );

    // 12|13010004: 3 vars (página geral) × 2 vars (variante 000801)
    assert!(g.request_template(0x12, &ADDR_PAGE, 3).is_ok());
    assert!(g.request_template(0x12, &ADDR_PAGE, 2).is_ok());
    assert!(g.request_template(0x12, &ADDR_PAGE, 0).is_err());

    // endpoint que não existe no golden
    assert!(g
        .request_template(0x12, &[0xDE, 0xAD, 0xBE, 0xEF], 0)
        .is_err());

    // rota completa da FSM para o caso GERAL: pp 0x0200, página 0
    let sysex = g
        .build_request(0x12, &ADDR_PAGE, &[0x02, 0x00, 0x00, 0x00, 0x01][..3])
        .expect("3 vars (pp + pg hi)");
    // (o padrão de 3 vars é [v,v,00,v,01]: pp + const 00 + pg byte + 01)
    assert_eq!(sysex[8], 0x12);
    assert_eq!(&sysex[9..13], &ADDR_PAGE);
    assert_eq!(sysex.last(), Some(&0xF7));
}

/// Despacho por by-len no IN (endereço compartilhado por 3 formatos):
/// resync 4B `[01][02][idx][01]`, tabela 75B (push) e ACK 4B do req (§13.7).
#[test]
fn response_dispatch_on_shared_endpoints() {
    let g = GoldenFile::embedded().expect("golden válido");
    let cands = g.for_response(0x12, &ADDR_TABLE);
    assert!(cands.len() >= 2, "push (by-len) + req (ACK) no endereço");

    // resync de save (§13.12): [01][02][idx][01]
    let resync: &[u8] = &[0x01, 0x02, 0x08, 0x01];
    let (t_resync, vars) = g
        .match_response(0x12, &ADDR_TABLE, resync)
        .expect("resync 4B despacha");
    assert_eq!(t_resync.template_type, "push");
    assert_eq!(vars, vec![&resync[2..3]], "var do resync = idx");

    // tabela de tipos 75B (exemplo real do push)
    let push = cands
        .iter()
        .copied()
        .find(|t| t.template_type == "push")
        .expect("push 12001002");
    let table = hex_decode(push.example.hex.as_deref().expect("exemplo 75B")).expect("hex");
    assert_eq!(table.len(), 75);
    let (t_tab, _) = g
        .match_response(0x12, &ADDR_TABLE, &table)
        .expect("tabela 75B despacha");
    assert_eq!(t_tab.template_type, "push");

    // ACK de chunk de IR (req): [3 vars][01]
    let ack = hex_decode(
        cands
            .iter()
            .copied()
            .find(|t| t.template_type == "req")
            .expect("req 12001002")
            .example
            .response_hex
            .as_deref()
            .expect("response_hex"),
    )
    .expect("hex");
    assert_eq!(ack.len(), 4);
    let (t_ack, av) = g
        .match_response(0x12, &ADDR_TABLE, &ack)
        .expect("ACK despacha");
    assert_eq!(t_ack.template_type, "req");
    assert_eq!(av.concat(), &ack[..3]);

    // endpoint IN COMPARTILHADO com bytes ambíguos: no 13010001, o push
    // ([var 2][0c1c0140]) e o req (const 01000c1c0140 inteiro) casam com os
    // MESMOS 6B — o despacho devolve o PRIMEIRO da ordem do arquivo (push),
    // extraindo as 2 vars iniciais. A FSM resolve pelo contexto do que
    // pediu (M0.6); aqui o contrato é: dispatch determinístico por ordem.
    let meta: &[u8] = &[0x01, 0x00, 0x0C, 0x1C, 0x01, 0x40];
    let (t_meta, mv) = g
        .match_response(0x12, &[0x13, 0x01, 0x00, 0x01], meta)
        .expect("meta despacha");
    assert_eq!(t_meta.template_type, "push", "ordem do arquivo vence");
    assert_eq!(mv.concat(), &meta[..2], "vars do push = 2 primeiros bytes");
    // payload divergente do const = não casa (despacho descarta)
    assert!(g
        .match_response(
            0x12,
            &[0x13, 0x01, 0x00, 0x01],
            &[0x01, 0x00, 0x0C, 0x1C, 0x01, 0x41]
        )
        .is_none());
}

/// `GoldenFile::build_request` monta o ENVELOPE completo (keepalive 0 vars).
#[test]
fn build_request_produces_full_sysex() {
    let g = GoldenFile::embedded().expect("golden válido");
    let sysex = g
        .build_request(0x12, &ADDR_KEEPALIVE, &[])
        .expect("keepalive sem vars");
    assert_eq!(sysex.len(), 8 + 1 + 4 + 4 + 1);
    assert_eq!(
        &sysex[..8],
        &[0xF0, 0x21, 0x25, 0x7F, 0x47, 0x50, 0x2D, 0x64]
    );
    assert_eq!(sysex[8], 0x12);
    assert_eq!(&sysex[9..13], &ADDR_KEEPALIVE);
    assert_eq!(&sysex[13..17], &[0x00, 0x00, 0x00, 0x00]); // payload const
    assert_eq!(sysex[17], 0xF7);
}

/// `Template::build_request` continua STRICT (faltam/excedem vars; push sem
/// request); `request_template` com endpoint inexistente erra com evidência.
#[test]
fn build_request_is_strict() {
    let g = GoldenFile::embedded().expect("golden válido");
    // select de preset (11|13010000): [pp hi][pp lo]
    let t = g.request_template(0x11, &ADDR_SELECT, 2).expect("select");
    assert!(t.build_request(&[]).is_err(), "faltam vars");
    assert!(
        t.build_request(&[0x01, 0x00, 0x99]).is_err(),
        "vars em excesso"
    );
    let ok = t.build_request(&[0x01, 0x00]).expect("2 vars");
    assert_eq!(&ok[13..15], &[0x01, 0x00]);

    let push = g
        .templates()
        .iter()
        .find(|t| t.template_type == "push")
        .unwrap();
    assert!(push.build_request(&[]).is_err());
}

/// extract_vars devolve segmentos POSICIONAIS — provado na página geral
/// (padrão real [v,v][00][v][01] = 3 vars; a RESPOSTA 13010003 é
/// variable-len: páginas de 196/32B, sem padrão de conteúdo no golden).
#[test]
fn extract_vars_are_positional() {
    let g = GoldenFile::embedded().expect("golden válido");
    let t = g
        .request_template(0x12, &ADDR_PAGE, 3)
        .expect("página geral");
    let pat = t.request_pattern().expect("request_payload");
    let wire = [0x01, 0x00, 0x00, 0x00, 0x01];
    let vars = pat.extract_vars(&wire).expect("casa");
    // 2 SEGMENTOS var (2B + 1B) = 3 bytes variáveis no total (var_count)
    assert_eq!(vars.len(), 2, "uma entrada por segmento var");
    assert_eq!(vars[0].len(), 2, "pp u16");
    assert_eq!(vars[1], &[0x00], "byte da página (depois do const 00)");
    let rebuilt = t.build_request(&vars.concat()).expect("rebuild");
    assert_eq!(&rebuilt[13..], &[0x01, 0x00, 0x00, 0x00, 0x01, 0xF7][..]); // a resposta de página é BY-LEN (32B meta-curta × 196B página cheia).
                                                                           // Propriedade: extract devolve EXATAMENTE os bytes var do sub-padrão
                                                                           // (soma var + soma const = len) — nenhum const vaza para as vars. O
                                                                           // payload de teste é construído a PARTIR do sub-padrão (consts + zeros
                                                                           // nas vars) — os consts de página são dados do boot, não arbitrários.
    let extract_totals = |len: usize| -> (usize, usize) {
        // 3 templates respondem em 13010003 (open→mixed196, page→by-len,
        // push→const196); o by-len é o do REQUEST DE PÁGINA (13010004)
        let t = g
            .for_response(0x12, &[0x13, 0x01, 0x00, 0x03])
            .into_iter()
            .find(|t| {
                t.template_type == "req"
                    && t.response_pattern()
                        .unwrap()
                        .by_len
                        .contains_key(&len.to_string())
            })
            .unwrap_or_else(|| panic!("req by-len com sub-padrão {len}B"));
        let sub = t
            .response_pattern()
            .unwrap()
            .by_len
            .get(&len.to_string())
            .unwrap_or_else(|| panic!("sub-padrão {len}B existe"));
        // monta o payload: consts onde manda o padrão, 0x00 nas vars
        let mut buf = Vec::with_capacity(len);
        let mut consts = sub
            .segments
            .iter()
            .filter(|s| s.segment_kind == gp100_core::golden::SegmentKind::Const)
            .map(|s| hex_decode(&s.hex).expect("const hex"))
            .collect::<Vec<_>>()
            .into_iter();
        for s in &sub.segments {
            match s.segment_kind {
                gp100_core::golden::SegmentKind::Const => {
                    buf.extend_from_slice(&consts.next().expect("const pareado"))
                }
                gp100_core::golden::SegmentKind::Var => {
                    buf.extend(std::iter::repeat_n(0u8, s.count))
                }
            }
        }
        let (t, v) = g
            .match_response(0x12, &[0x13, 0x01, 0x00, 0x03], &buf)
            .unwrap_or_else(|| panic!("página {len}B despacha"));
        assert_eq!(t.template_type, "req");
        let pat = t.response_pattern().unwrap();
        let sub = pat
            .by_len
            .get(&len.to_string())
            .expect("sub-padrão por comprimento");
        let mut want_vars = 0;
        let mut want_consts = 0;
        for s in &sub.segments {
            match s.segment_kind {
                gp100_core::golden::SegmentKind::Var => want_vars += s.count,
                gp100_core::golden::SegmentKind::Const => want_consts += s.hex.len() / 2,
            }
        }
        assert_eq!(want_vars + want_consts, len, "sub-padrão cobre o payload");
        (v.concat().len(), want_vars)
    };
    let (got196, want196) = extract_totals(196);
    assert_eq!(got196, want196, "196B: só bytes var extraídos");
    let (got32, want32) = extract_totals(32);
    assert_eq!(got32, want32, "32B: só bytes var extraídos");
    assert_ne!(want196, want32, "sub-padrões distintos por comprimento");
}

/// hex_decode: aceita caixa alta/baixa, rejeita ímpar e não-hex.
#[test]
fn hex_decode_strict() {
    assert_eq!(hex_decode("0Abf").unwrap(), vec![0x0A, 0xBF]);
    assert!(hex_decode("abc").is_err()); // ímpar
    assert!(hex_decode("zz").is_err()); // não-hex
}
