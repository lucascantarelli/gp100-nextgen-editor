//! boot + progresso sobre o MOCK REAL: até aqui o `boot()` completo só tinha sido provado
//! sobre o transporte de REPLAY (fixtures da S1); o mock exigia rotas
//! novas (meta6 da sonda @13020001 — t12; PG 8 da sonda @13020005 — t16).
//! Estes testes travam o boot END-TO-END contra o `MockDevice` e o hook
//! observacional de progresso (`boot_with_progress`).

use gp100_core::session::{BootProgress, BootStage, Session};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport;

/// Boot completo sobre o MOCK (travessia do "device" simulado): com o
/// inventário DA CAPTURA (198 pps começando no corrente 0x0100 — quirk
/// §13.4 duplica select/open dele), o script fecha em 2299 transações
/// (= prova C do replay), sem Timeout/InvalidShape, e o scan deixa o pp
/// no último do inventário.
#[test]
fn boot_completo_sobre_o_mock() {
    let mut mock = MockDevice::new().expect("mock montado (R4 travado no build)");
    mock.open().expect("open do chamador (ADR-4)");
    let mut session = Session::new(&mut mock);
    // Inventário da S1: o pp corrente PRIMEIRO + os demais 0..197.
    let mut pps: Vec<u16> = vec![0x0100];
    pps.extend(0u16..197);
    session.set_inventory(pps);

    let report = session.boot().expect("boot completo contra o mock");
    assert_eq!(report.transactions, 2299);
    assert_eq!(
        session.current_pp(),
        196,
        "scan termina no último pp (0..197)"
    );
}

/// O hook de progresso é OBSERVACIONAL: mesmo total do boot canônico,
/// 1 beat por transação, monotônico sem buracos, stage na ordem §13.10
/// e o último beat em Keepalive com done == total.
#[test]
fn boot_com_progresso_beats_por_transacao() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("open");
    let mut session = Session::new(&mut mock);
    let mut pps: Vec<u16> = vec![0x0100];
    pps.extend(0u16..197);
    session.set_inventory(pps);

    let mut beats: Vec<(BootStage, usize, usize)> = Vec::new();
    let report = session
        .boot_with_progress(Some(&mut |p: BootProgress| {
            beats.push((p.stage, p.done, p.total))
        }))
        .expect("boot com progresso");

    assert_eq!(report.transactions, 2299);
    assert_eq!(beats.len(), 2299, "1 beat por transação");
    for (i, (_, done, total)) in beats.iter().enumerate() {
        assert_eq!(*total, 2299);
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
        let mut pps: Vec<u16> = vec![0x0100];
        pps.extend(0u16..197);
        session.set_inventory(pps);
        let r = if use_progress {
            session.boot_with_progress(None).expect("boot sem callback")
        } else {
            session.boot().expect("boot canônico")
        };
        txs.push(r.transactions);
    }
    assert_eq!(txs[0], txs[1], "hook None não altera o script");
    assert_eq!(txs[0], 2299);
}

/// O pp corrente no progresso acompanha o scan (a UI mostra o preset
/// sendo levantado): na sonda e nas etapas finais o pp é o último do
/// inventário (197), já que o scan acabou de percorrer 0..198.
#[test]
fn progresso_carrega_pp_corrente() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut session = Session::new(&mut mock);
    let mut pps: Vec<u16> = vec![0x0100];
    pps.extend(0u16..197);
    session.set_inventory(pps);

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
    assert_eq!(report.transactions, 2299);

    let probe = seen_probe.expect("há beats da sonda (T6)");
    assert_eq!(probe.current_pp, 196);
    let names = seen_names.expect("há beats de nomes (T3)");
    assert_eq!(names.current_pp, 196);
}
