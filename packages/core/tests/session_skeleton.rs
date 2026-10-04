//! Testes de CONTRATO da session (ADR-6 rev.3): a FSM constrói sobre
//! um transporte EXTERNO (ciclo de vida do chamador) e os 7 métodos da FSM
//! estão IMPLEMENTADOS (nenhum `todo!` sobreviveu — era o contrato do
//! esqueleto; a implementação é provada pelo replay em
//! `tests/replay_fixtures.rs`).

use std::time::Duration;

use gp100_core::session::Session;
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};

/// Transporte externo mínimo: prova que `Session` aceita QUALQUER
/// implementador da trait (ADR-4) sem conhecer internals.
struct NullTransport;

impl DeviceTransport for NullTransport {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn send_raw(&mut self, _data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        Ok(())
    }
    fn recv_raw(&mut self, _timeout: Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::RecvTimeout { timeout_ms: 0 })
    }
}

/// `Session::new` NÃO abre o transporte nem consome I/O (ciclo de vida do
/// chamador), e `into_transport` devolve o MESMO transporte.
#[test]
fn session_nao_dona_do_ciclo_de_vida() {
    let mut t = NullTransport;
    t.open()
        .expect("o chamador abre ANTES de construir a Session");
    let session = Session::new(t);
    let mut back = session.into_transport();
    // o transporte devolvido segue funcional (mesmo objeto)
    assert!(back.send_raw(&[0xF0, 0xF7], WireKind::Write).is_ok());
    back.close().expect("o chamador fecha DEPOIS");
}

/// FSM completa: os 7 métodos da FSM não são mais placeholders — cada um
/// executa de verdade (aqui: fire-and-forget no NullTransport = Ok; os
/// que esperam resposta estouram Timeout tipado do ADR-3, não panic).
#[test]
fn metodos_implementados_sem_todo() {
    let mut s = Session::new(NullTransport);
    s.set_param(1, 0x0300_0001, 0, 42.0)
        .expect("set_param (D4)");
    s.save_preset(0, 4, "It's GP100").expect("save (D3)");
    // upload_ir executa begin+chunk e estoura Timeout esperando o ACK (D1/D6)
    assert!(matches!(
        s.upload_ir(0, &[0x5Au8; 15]),
        Err(gp100_core::ProtocolError::Timeout { .. })
    ));
    assert!(s.pending_pushes().expect("backlog vazio").is_empty());
    // transações com resposta estouram Timeout do ADR-3 no NullTransport
    // (D6) — não panic nem erro de transporte cru
    assert!(matches!(
        s.boot(),
        Err(gp100_core::ProtocolError::Timeout { .. })
    ));
    assert!(matches!(
        s.scan_state(),
        Err(gp100_core::ProtocolError::Timeout { .. })
    ));
    assert!(matches!(
        s.list_user_irs(),
        Err(gp100_core::ProtocolError::Timeout { .. })
    ));
}
