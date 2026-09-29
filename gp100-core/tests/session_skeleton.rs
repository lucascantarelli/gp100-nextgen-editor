//! Testes de CONTRATO do esqueleto do session (M0.6, ADR-6 rev.3): a FSM
//! deve ser CONSTRUÍVEL sobre um transporte EXTERNO (ciclo de vida do
//! chamador) e os métodos ainda-não-implementados devem ser placeholders
//! EXPLÍCITOS (`todo!("M0.6: …")` — nunca `unimplemented!` genérico nem
//! corpo silencioso).

use std::time::Duration;

use gp100_core::session::Session;
use gp100_core::transport::{DeviceTransport, TransportError};

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
    fn send_raw(&mut self, _data: &[u8]) -> Result<(), TransportError> {
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
    assert!(back.send_raw(&[0xF0, 0xF7]).is_ok());
    back.close().expect("o chamador fecha DEPOIS");
}

/// Os métodos da FSM ainda não implementados devem PANICAR com a mensagem
/// explícita "M0.6" (contrato do esqueleto: nada de corpo silencioso que
/// fingiria sucesso — a issue M0.6 torna cada um verde no replay).
#[test]
fn metodos_sao_placeholders_explicitos_m06() {
    let mut session = Session::new(NullTransport);

    let msgs: Vec<String> = [
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.boot())).err(),
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.scan_state())).err(),
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            session.set_param(1, 0x0700_006e, 0, 15.0)
        }))
        .err(),
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            session.save_preset(0, 4, "It's GP100")
        }))
        .err(),
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            session.upload_ir(0, &[0u8; 15])
        }))
        .err(),
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.list_user_irs())).err(),
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.pending_pushes())).err(),
    ]
    .into_iter()
    .map(|e| {
        let e = e.expect("todo! panica");
        e.downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .expect("mensagem de panic é String/&str")
    })
    .collect();

    assert_eq!(msgs.len(), 7, "os 7 métodos da FSM estão em todo!");
    for m in &msgs {
        // todo!() prefixa com "not yet implemented: " — o que importa é a
        // mensagem apontar a issue M0.6 (nunca placeholder genérico)
        assert!(m.contains("M0.6"), "placeholder deve apontar a issue: {m}");
    }
}
