//! boot + progresso sobre o MOCK REAL: até aqui o `boot()` completo só tinha sido provado
//! sobre o transporte de REPLAY (fixtures da S1); o mock exigia rotas
//! novas (meta6 da sonda @13020001 — t12; PG 8 da sonda @13020005 — t16).
//! Estes testes travam o boot END-TO-END contra o `MockDevice` e o hook
//! observacional de progresso (`boot_with_progress`).

use gp100_core::session::{BootProgress, BootStage, Session};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport;

/// Boot completo sobre o MOCK (travessia do "device" simulado): com o
/// inventário DEFAULT do mock — `0..198`, o espaço do DOCUMENTO (#132: só o
/// aparelho varre o banco/slot) — o script fecha em **2297** transações,
/// sem Timeout/InvalidShape, e o scan deixa o pp no último do inventário
/// (`0x00C5` = 197).
///
/// **Dois scripts, duas provas.** O APARELHO fecha em **2299** (banco
/// `0x01xx` + o quirk §13.4 do corrente `0x0100` duplicado + keepalive ×2),
/// e quem o prova é `tests/pp_gate.rs::o_boot_do_aparelho_varre_os_198_pps_da_captura`,
/// pela contagem do replay da S1. Aqui o número é o do MOCK: `0..198` não
/// contém o `0x0100`, então não há quirk — e o número não é decorativo:
/// `40 + 198×11 + 11 + 5 + 61 + 2`.
#[test]
fn boot_completo_sobre_o_mock() {
    let mut mock = MockDevice::new().expect("mock montado (R4 travado no build)");
    mock.open().expect("open do chamador (ADR-4)");
    let mut session = Session::new(&mut mock);
    let report = session.boot().expect("boot completo contra o mock");
    assert_eq!(report.transactions, 2297);
    assert_eq!(
        session.current_pp(),
        0x00C5,
        "scan termina no último pp do inventário do mock (197 = 0x00C5)"
    );
}

/// O hook de progresso é OBSERVACIONAL: mesmo total do boot canônico,
/// 1 beat por transação, monotônico sem buracos, stage na ordem §13.10
/// e o último beat em Keepalive com done == total.
///
/// O número é o do MOCK (2297 — ver `boot_completo_sobre_o_mock`); o
/// APARELHO é 2299 e quem o prova é o `pp_gate.rs`.
#[test]
fn boot_com_progresso_beats_por_transacao() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("open");
    let mut session = Session::new(&mut mock);

    let mut beats: Vec<(BootStage, usize, usize)> = Vec::new();
    let report = session
        .boot_with_progress(Some(&mut |p: BootProgress| {
            beats.push((p.stage, p.done, p.total))
        }))
        .expect("boot com progresso");

    assert_eq!(report.transactions, 2297);
    assert_eq!(beats.len(), 2297, "1 beat por transação");
    for (i, (_, done, total)) in beats.iter().enumerate() {
        assert_eq!(*total, 2297);
        assert_eq!(*done, i + 1, "done cresce 1 a 1 sem buracos");
    }
    // stages só AVANÇAM na ordem do script (Tables → … → Keepalive)
    let order = [
        BootStage::Tables,
        BootStage::Scan,
        BootStage::Probe,
        BootStage::Setlist,
        BootStage::Names,
        BootStage::Keepalive,
    ];
    let mut last: Option<BootStage> = None;
    for (stage, _, _) in &beats {
        if Some(*stage) != last {
            if let Some(prev) = last {
                let a = order.iter().position(|s| s == &prev).unwrap();
                let b = order.iter().position(|s| s == stage).unwrap();
                assert!(b > a, "stage voltou: {prev:?} -> {stage:?}");
            }
            last = Some(*stage);
        }
    }
    assert_eq!(beats.last().map(|(s, _, _)| *s), Some(BootStage::Keepalive));
}

/// `boot()` (canônico, ADR-6) e `boot_with_progress(None)` são o MESMO
/// script — o hook é só observação.
#[test]
fn boot_com_progresso_none_equivalente() {
    let mut txs: Vec<usize> = Vec::new();
    for use_progress in [false, true] {
        let mut mock = MockDevice::new().expect("mock");
        mock.open().expect("open");
        let mut session = Session::new(&mut mock);
        let r = if use_progress {
            session.boot_with_progress(None).expect("boot sem callback")
        } else {
            session.boot().expect("boot canônico")
        };
        txs.push(r.transactions);
    }
    assert_eq!(txs[0], txs[1], "hook None não altera o script");
    assert_eq!(txs[0], 2297);
}

/// O pp corrente no progresso acompanha o scan (a UI mostra o preset
/// sendo levantado): na sonda e nas etapas finais o pp é o último do
/// inventário — no MOCK, `0x00C5` (197); no aparelho seria `0x0162`, o
/// mesmo fato provado no `pp_gate.rs`.
#[test]
fn progresso_carrega_pp_corrente() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut session = Session::new(&mut mock);

    let mut seen_probe: Option<BootProgress> = None;
    let mut seen_names: Option<BootProgress> = None;
    let report = session
        .boot_with_progress(Some(&mut |p: BootProgress| {
            if p.stage == BootStage::Probe && seen_probe.is_none() {
                seen_probe = Some(p);
            }
            if p.stage == BootStage::Names && seen_names.is_none() {
                seen_names = Some(p);
            }
        }))
        .expect("boot com progresso");
    assert_eq!(report.transactions, 2297);

    let probe = seen_probe.expect("há beats da sonda (T6)");
    assert_eq!(probe.current_pp, 0x00C5);
    let names = seen_names.expect("há beats de nomes (T3)");
    assert_eq!(names.current_pp, 0x00C5);
}

/// **Ponta a ponta pelo caminho REAL do boot: fio → cache → decode.**
///
/// O `boot()` tem de deixar as 9 páginas de cada pp no cache no SHAPE do
/// fio (196B nas 8 primeiras, 32B na 8ª) e no ÍNDICE certo (`raw[3]` =
/// 0..8), de modo que `preset_pages::decode` aceite as 198. É o contrato
/// que a UI consome (`device_board`, `device_preset_library`, nome do pp
/// corrente) — o teste `nome_bate_com_o_all_prst_em_198_de_198` prova o
/// CONTEÚDO sobre a captura real; aqui prova-se o caminho.
///
/// Por que o conteúdo não é checado aqui: o `MockDevice` é simulador de
/// SHAPE e não de conteúdo — o corpo do `open` é preenchido com zeros por
/// design (`mock.rs`: "vars de página não observadas no boot capturado —
/// zeros; o conteúdo não é evidência"). Checar o nome contra o `all.prst`
/// sobre o mock seria esperar evidência que ele nem tenta servir.
#[test]
fn scan_preenche_cache_no_shape_do_fio_ponta_a_ponta() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut session = Session::new(&mut mock);
    session.boot().expect("boot completo");

    let pps = session.cached_pps();
    assert_eq!(pps.len(), 198, "as 198 páginas de cada pp do inventário");

    let mut ok = 0usize;
    for pp in &pps {
        let pags = session
            .preset_state(*pp)
            .unwrap_or_else(|| panic!("pp {pp:#06x} sem cache após o boot"));

        // SHAPE do fio: 8 páginas de 196B (4B header + 192B) e a 8ª de
        // 32B (4B + 28B) — é o comprimento da captura real, §13.10.
        for (i, p) in pags.iter().enumerate().take(8) {
            assert_eq!(p.raw.len(), 196, "pp {pp:#06x} pg {i}");
        }
        assert_eq!(pags[8].raw.len(), 32, "pp {pp:#06x} pg 8 (32B)");

        // ÍNDICE pelo payload: `raw[3]` é o número da página (§13.10),
        // 0..8 — não a posição do pedido (o `open` entrega a 0).
        for (i, p) in pags.iter().enumerate() {
            assert_eq!(
                p.raw[3], i as u8,
                "pp {pp:#06x} slot {i}: raw[3] deve ser {i} (veio {})",
                p.raw[3]
            );
        }

        let dec =
            gp100_core::preset_pages::decode(pags).unwrap_or_else(|e| panic!("pp {pp:#06x}: {e}"));
        for i in 0..8usize {
            assert_eq!(dec.corpo(i).len(), 96, "pp {pp:#06x} pg {i} decodificado");
        }
        assert_eq!(dec.corpo(8).len(), 14, "pp {pp:#06x} pg 8 decodificado");
        ok += 1;
    }
    assert_eq!(ok, 198, "198/198 pelo caminho do boot");
}

/// **#161 — o relatório carrega o inventário e os nomes lidos.** O gate
/// do front só monta a casca com o aparelho LIDO ("198/198" do catálogo
/// atual); aqui se prova que os números chegam VERDADEIROS pelo mesmo
/// caminho do fio (scan → cache → decode da pg0), não que são esperados —
/// a interpretação ("coerente com o catálogo") é do gate.
#[test]
fn relatorio_carrega_inventario_e_nomes_do_scan() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut session = Session::new(&mut mock);
    let rel = session.boot().expect("boot completo");
    assert_eq!(rel.presets, 198, "inventário lido pelo scan (0..198, #132)");
    assert_eq!(
        rel.names, 198,
        "198/198: toda pg0 do cache decodifica com nome pelo caminho do boot"
    );
    assert!(rel.transactions >= 2295, "transações do script de boot");
}
