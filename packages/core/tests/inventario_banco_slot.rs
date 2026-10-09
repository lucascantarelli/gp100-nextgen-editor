//! O espaço de presets no fio é BANCO/SLOT, não um intervalo linear (#148).
//!
//! Campo 07/10: o scan do boot mandava selects com pp `0x0063..0x00C5`
//! (inventário linear `0..198`) — presets que NÃO existem no aparelho — e o
//! firmware V2.1 morria no assert `PresetNum < TOTAL_PA` (audio.c:912),
//! voltando só com power-cycle. As capturas (S1–S4: 796 selects, 198
//! payloads distintos) mostram o espaço real: `0000..=0062` + `0100..=0162`.
//! Estes testes travam o inventário default e a guarda de select ANTES do
//! fio — o mesmo padrão da #110 para valores de knob (ADR-10).
//!
//! **A guarda é do APARELHO (#132/ADR-12).** O espaço do MOCK continua sendo
//! o do documento (`0..198`, e o scan dele varre os 198 — ver
//! `boot_mock.rs`); por isso o teste de recusa usa o embrulho `Aparelho`, o
//! MESMO que o build de campo (#130) e o `pp_gate.rs` usam, e prova a borda
//! do mock ao lado, no mesmo teste.

use std::time::Duration;

use gp100_core::session::{inventario_default, pp_e_valido, Session};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};

/// O inventário default É a lista capturada: 198 pps nos dois bancos, sem
/// NENHUM pp do intervalo linear inexistente (0x0063..0x00C5).
#[test]
fn inventario_default_e_o_espaco_banco_slot_da_captura() {
    let pps = inventario_default();
    assert_eq!(pps.len(), 198, "198 presets = 2 bancos de 99");
    assert_eq!(pps.first(), Some(&0x0000));
    assert_eq!(pps[98], 0x0062, "fim do banco 00");
    assert_eq!(pps[99], 0x0100, "início do banco 01");
    assert_eq!(pps.last(), Some(&0x0162));
    // os pps lineares 0x0063..0x00C5 não podem aparecer NUNCA:
    for pp in 0x0063u16..=0x00C5 {
        assert!(
            !pps.contains(&pp),
            "pp inexistente no inventário: {pp:#06x}"
        );
    }
}

/// A guarda aceita SOMENTE o espaço capturado — inclusive rejeitando os
/// valores "plausíveis" que o inventário linear antigo gerava.
#[test]
fn pp_e_valido_aceita_somente_o_espaco_capturado() {
    for pp in 0x0000u16..=0x0062 {
        assert!(pp_e_valido(pp), "banco 00, slot {pp} é válido");
    }
    for pp in 0x0100u16..=0x0162 {
        assert!(pp_e_valido(pp), "banco 01, slot {pp:x} é válido");
    }
    for pp in [0x0063u16, 0x00C5, 0x0163, 0x0200, 0xFFFF] {
        assert!(!pp_e_valido(pp), "pp fora do espaço aceito: {pp:#06x}");
    }
}

/// Um transporte que se DECLARA aparelho por cima do mock — o mesmo embrulho
/// do `LoggingTransport` no build de campo (#130), e o caminho pelo qual a
/// identidade `e_aparelho` chega à `Session` sem hardware (o `RealDevice` de
/// verdade só existe atrás da feature).
struct Aparelho<T>(T);

impl<T: DeviceTransport> DeviceTransport for Aparelho<T> {
    fn open(&mut self) -> Result<(), TransportError> {
        self.0.open()
    }
    fn close(&mut self) -> Result<(), TransportError> {
        self.0.close()
    }
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        self.0.send_raw(data, kind)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        self.0.recv_raw(timeout)
    }
    fn permite_escrita(&self) -> bool {
        self.0.permite_escrita()
    }
    fn e_aparelho(&self) -> bool {
        true
    }
}

/// A sessão "de campo" do teste: mock por baixo, aparelho por cima.
fn sessao_aparelho() -> Session<Aparelho<MockDevice>> {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("open do chamador (ADR-4)");
    Session::new(Aparelho(mock))
}

/// select de pp inexistente no APARELHO é RECUSADO antes de qualquer byte no
/// fio (#148), e a recusa fala do espaço do aparelho — não é Timeout nem erro
/// de transporte. No MOCK o MESMO pp passa: a guarda é do aparelho
/// (#132/ADR-12), e é essa borda que o teste também trava.
#[test]
fn select_de_pp_inexistente_e_recusado_antes_do_fio() {
    let mut session = sessao_aparelho();

    let err = session
        .select_preset(0x0063)
        .expect_err("pp inexistente tem de ser recusado");
    assert!(
        err.to_string().contains("0x0000..0x0062") && err.to_string().contains("0x0100..0x0162"),
        "o erro fala do espaço do aparelho: {err}"
    );
    assert_eq!(
        session.into_transport().0.transactions(),
        0,
        "a recusa acontece ANTES do fio — zero transações"
    );

    // pp válido passa (o device segue com o pp selecionado):
    let mut session = sessao_aparelho();
    session.select_preset(0x0000).expect("select válido passa");
    assert_eq!(session.current_pp(), 0x0000);

    // E o MOCK puro aceita o pp que o aparelho recusa (#132).
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("open do chamador (ADR-4)");
    let mut session = Session::new(&mut mock);
    session.select_preset(0x0063).expect("mock sem trava");
}
