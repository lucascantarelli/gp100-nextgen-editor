//! Contrato do device que MORRE no meio da sessão (issue #48).
//!
//! O `TransportError::DeviceGone` nasceu no M0.5 (ADR-4) com a promessa
//! escrita de que "a FSM/H1 decide diferente" dele para um `SendFailed`
//! transitório. Estes contratos tornam a promessa verificável:
//!
//!   1. o mock (test double declarado) sabe CAIR — `MockFault::DieAfter(n)`;
//!   2. o transporte reporta `DeviceGone` com tipo próprio (não `SendFailed`);
//!   3. a `Session` PROPAGA o tipo (`ProtocolError::DeviceGone`), sem achatá-lo
//!      em `InvalidShape` — é o que permite a UI parar de tentar e mostrar
//!      recuperação em vez de retentar um device que já não está no fio;
//!   4. nada disso trava: a falha é imediata e o objeto segue respondendo
//!      diagnóstico (estado/`transactions`).

use gp100_core::session::Session;
use gp100_core::transport::mock::{MockDevice, MockFault};
use gp100_core::transport::{DeviceTransport, TransportError};
use gp100_core::ProtocolError;
use std::time::{Duration, Instant};

/// O mock cai exatamente depois de `n` transmissões — e a morte precede o
/// parse do frame (não há ninguém no fio para receber bytes).
#[test]
fn mock_cai_depois_de_n_transmissoes() {
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_fault(MockFault::DieAfter(0));
    dev.open().expect("open enquanto o device está presente");

    let err = dev
        .send_raw(b"nao-e-sysex")
        .expect_err("o 1º send já encontra o device ausente");
    assert!(
        matches!(err, TransportError::DeviceGone { .. }),
        "DeviceGone precede o shape do frame: {err:?}"
    );
    assert_eq!(dev.transactions(), 1, "a tentativa conta mesmo morrendo");
    assert!(
        matches!(
            dev.recv_raw(Duration::from_millis(1)),
            Err(TransportError::DeviceGone { .. })
        ),
        "recv também reporta o device ausente"
    );
}

/// A `Session` NÃO achata o device ausente em `InvalidShape`: o tipo próprio
/// é o contrato que a UI consome (LED off + retry explícito).
#[test]
fn sessao_propaga_device_gone_sem_achatar_em_shape() {
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_fault(MockFault::DieAfter(1));
    dev.open().expect("open antes da morte");
    let mut session = Session::new(dev);

    // transação 1: device vivo (o select §13.10 passa e muda o pp)
    session.select_preset(3).expect("select com o device vivo");

    // transação 2: o device SUMIU — aborta com o tipo próprio
    let start = Instant::now();
    let err = session
        .select_preset(4)
        .expect_err("o device caiu no meio da sessão");
    assert!(
        matches!(err, ProtocolError::DeviceGone { .. }),
        "esperado ProtocolError::DeviceGone, veio {err:?}"
    );
    assert!(
        err.to_string().contains("device sumiu no meio da sessão"),
        "mensagem humana preservada: {err}"
    );
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "device ausente falha IMEDIATO (sem esperar a janela de 3s)"
    );
}

/// O boot pode morrer no meio: o erro sobe tipado (é o cenário do smoke
/// Tauri, que arma o mesmo plano no shell).
#[test]
fn boot_que_perde_o_device_no_meio_aborta_tipado() {
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_fault(MockFault::DieAfter(60));
    dev.open().expect("open antes da morte");
    let mut session = Session::new(dev);

    let err = session
        .boot_with_progress(None)
        .expect_err("o boot não termina com o device caindo no meio");
    assert!(
        matches!(err, ProtocolError::DeviceGone { .. }),
        "esperado ProtocolError::DeviceGone, veio {err:?}"
    );
}
