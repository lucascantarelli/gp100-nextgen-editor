//! Testes de CONTRATO da trait de transporte (ADR-4): a trait
//! `DeviceTransport` deve ser IMPLEMENTÁVEL por quem está FORA do crate
//! (é assim que o `MockDevice` e o `RealDevice` a
//! consomem) e utilizável como objeto de trait (`dyn`), sem infra extra.

use std::time::Duration;

use gp100_core::transport::{DeviceTransport, TransportError, WireKind};
use gp100_core::{SYSEX_EOX, SYSEX_HEADER};

/// Implementador EXTERNO mínimo (loopback): prova que a trait é pública,
/// object-safe e suficiente — nada de internals do crate é necessário.
/// O `plugged` simula o device físico (desconexão no meio da sessão).
struct LoopbackTransport {
    opened: bool,
    plugged: bool,
    inbox: Vec<Vec<u8>>,
}

impl LoopbackTransport {
    fn new(pre: Vec<Vec<u8>>) -> Self {
        Self {
            opened: false,
            plugged: true,
            inbox: pre,
        }
    }

    /// Simula desconexão física (remoção do USB) SEM fechar o transporte.
    fn unplug(&mut self) {
        self.plugged = false;
    }

    /// Simula o retorno do device físico.
    fn plug(&mut self) {
        self.plugged = true;
    }
}

impl DeviceTransport for LoopbackTransport {
    fn open(&mut self) -> Result<(), TransportError> {
        if !self.plugged {
            return Err(TransportError::OpenFailed {
                why: "device ausente".into(),
            });
        }
        self.opened = true;
        Ok(())
    }

    fn close(&mut self) -> Result<(), TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        self.opened = false;
        Ok(())
    }

    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        if !self.plugged {
            return Err(TransportError::DeviceGone {
                why: "midiOutLongMsg: device removido".into(),
            });
        }
        // loopback: ecoa de volta (o mock real responderia via golden)
        self.inbox.push(data.to_vec());
        Ok(())
    }

    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        if !self.plugged {
            return Err(TransportError::DeviceGone {
                why: "midiIn sem device".into(),
            });
        }
        if self.inbox.is_empty() {
            return Err(TransportError::RecvTimeout {
                timeout_ms: timeout.as_millis() as u64,
            });
        }
        Ok(self.inbox.remove(0))
    }
}

/// Um envelope mínimo para exercitar o loopback (header + F7, §13.1).
fn envelope() -> Vec<u8> {
    let mut m = Vec::from(SYSEX_HEADER);
    m.push(SYSEX_EOX);
    m
}

#[test]
fn ciclo_de_vida_e_eco_via_trait() {
    let mut t = LoopbackTransport::new(vec![]);
    t.open().expect("abre");
    t.send_raw(&envelope(), WireKind::Write).expect("envia");
    let got = t
        .recv_raw(Duration::from_millis(100))
        .expect("eco na janela");
    assert_eq!(got, envelope());
    t.close().expect("fecha");
    // close idempotente na prática do implementador: 2º close é erro tipado
    assert!(matches!(t.close(), Err(TransportError::Closed)));
}

#[test]
fn recv_sem_mensagem_estoura_timeout_tipado() {
    let mut t = LoopbackTransport::new(vec![]);
    t.open().expect("abre");
    match t.recv_raw(Duration::from_millis(5)) {
        Err(TransportError::RecvTimeout { timeout_ms }) => assert_eq!(timeout_ms, 5),
        other => panic!("esperava RecvTimeout, veio {other:?}"),
    }
}

#[test]
fn send_fechado_e_erro_tipado_sem_panic() {
    let mut t = LoopbackTransport::new(vec![]);
    assert!(matches!(
        t.send_raw(&envelope(), WireKind::Write),
        Err(TransportError::Closed)
    ));
    assert!(matches!(
        t.recv_raw(Duration::from_millis(1)),
        Err(TransportError::Closed)
    ));
}

#[test]
fn trait_e_object_safe_para_dyn() {
    // MockDevice/RealDevice serão carregados como dyn se a
    // FSM optar por injeção dinâmica — a trait precisa suportar desde já.
    let mut t: Box<dyn DeviceTransport> = Box::new(LoopbackTransport::new(vec![envelope()]));
    t.open().expect("abre via dyn");
    assert!(t.send_raw(&envelope(), WireKind::Write).is_ok());
    assert!(t.recv_raw(Duration::from_millis(100)).is_ok());
}

/// Desconexão física NO MEIO da sessão = `DeviceGone` (não `Closed`, não
/// `SendFailed`); o mesmo objeto RECONECTA com `open()` (decisão de projeto).
#[test]
fn device_gone_durante_sessao_e_reconexao_no_mesmo_objeto() {
    let mut t = LoopbackTransport::new(vec![]);
    t.open().expect("abre");
    t.send_raw(&envelope(), WireKind::Write)
        .expect("envia com device presente");

    // usuário desconecta a pedal: próxima operação detecta DeviceGone
    t.unplug();
    assert!(matches!(
        t.send_raw(&envelope(), WireKind::Write),
        Err(TransportError::DeviceGone { .. })
    ));
    assert!(matches!(
        t.recv_raw(Duration::from_millis(10)),
        Err(TransportError::DeviceGone { .. })
    ));

    // SEM fechar/sem recriar: device volta, open() reconecta
    t.plug();
    t.open().expect("reconecta no MESMO objeto");
    t.send_raw(&envelope(), WireKind::Write)
        .expect("envia pós-reconexão");
    assert_eq!(
        t.recv_raw(Duration::from_millis(100)).expect("eco"),
        envelope()
    );
}

/// `open()` com o device fisicamente AUSENTE é `OpenFailed` (não
/// `DeviceGone` — que só ocorre no MEIO de uma sessão aberta).
#[test]
fn open_sem_device_e_open_failed() {
    let mut t = LoopbackTransport::new(vec![]);
    t.unplug();
    match t.open() {
        Err(TransportError::OpenFailed { why }) => assert!(why.contains("ausente")),
        other => panic!("esperava OpenFailed, veio {other:?}"),
    }
}
