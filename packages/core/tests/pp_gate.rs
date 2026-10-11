//! Trava de faixa do `pp` no aparelho (#132/ADR-12) — a irmã da ADR-10.
//!
//! Três coisas são provadas aqui, na ordem em que a issue as pede:
//!
//! 1. **O inventário do aparelho É o da captura** —
//!    [`inventario_do_aparelho`] tem de ser, `u16` por `u16`, a sequência que
//!    o Suite varre em `analysis/fixtures/boot.jsonl` (198 pps: banco
//!    `0x01xx` e banco `0x00xx`). Nenhum número aqui é de parede.
//! 2. **A recusa acontece ANTES do frame** — um `select` fora do inventário
//!    não pode nem chegar no `send_raw` (contagem de transações do mock = 0);
//!    dentro do inventário, sai e o aparelho responde.
//! 3. **O mock não muda** — ele continua aceitando qualquer `pp` (o espaço
//!    dele é o do documento, e os testes varrem pps arbitrários).
//!
//! O duplo `Aparelho` é um `MockDevice` que se DECLARA aparelho: é o mesmo
//! embrulho que o `LoggingTransport` faz no build de campo (#130), e é por
//! ele que a identidade `e_aparelho` chega à `Session` sem depender de
//! hardware (o `RealDevice` de verdade só existe atrás da feature).

use std::time::Duration;

use gp100_core::session::{inventario_do_aparelho, Session};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};
use gp100_core::ProtocolError;

mod common;
use common::{fixture_rows, fixture_tuple};

/// Um transporte que É aparelho por cima de um mock — a mesma composição do
/// `LoggingTransport` no build de campo, com `e_aparelho` ligado.
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

/// Os pps do `11/13010000` da captura, na ordem e SEM repetir (o 0x0100
/// aparece duas vezes — é o quirk do preset corrente, §13.4).
fn pps_da_captura() -> Vec<u16> {
    let mut pps: Vec<u16> = Vec::new();
    for (_, dir, _f, addr, data) in fixture_rows("boot.jsonl").into_iter().map(fixture_tuple) {
        if dir == "out" && addr == "13010000" {
            let b = gp100_core::golden::hex_decode(&data).expect("pp hex");
            let pp = u16::from_be_bytes([b[0], b[1]]);
            if !pps.contains(&pp) {
                pps.push(pp);
            }
        }
    }
    pps
}

/// **Critério de aceite 1 (#132): o espaço único vem da captura.** Se alguém
/// mudar um dos dois lados, este teste acusa — é ele que impede que a faixa
/// vire "número de parede" (a regra que o ADR-10 existe para evitar).
#[test]
fn o_inventario_do_aparelho_e_o_da_captura_s1() {
    let da_captura = pps_da_captura();
    assert_eq!(da_captura.len(), 198, "a S1 varre 198 pps");
    assert_eq!(
        inventario_do_aparelho(),
        da_captura,
        "o default do aparelho tem de ser, byte a byte, o que o Suite varre"
    );
}

/// **Critério de aceite 2: a recusa é ANTES do frame.** Um `pp` fora do
/// inventário não chega no `send_raw` — o mock nem conta a transação — e o
/// erro nomeia o endereço, o campo e a faixa, que é o que o runbook de campo
/// precisa ler.
#[test]
fn select_fora_do_inventario_e_recusado_antes_do_frame() {
    let mut s = sessao_aparelho();

    for pp in [0x0063u16, 0x00c5, 0x00ff, 0x0163, 0xffff] {
        let e = s
            .select_preset(pp)
            .expect_err("pp fora do inventário tem de ser recusado");
        match e {
            ProtocolError::ValueOutOfRange {
                ref addr,
                ref param,
                ref got,
                ref allowed,
            } => {
                assert_eq!(addr, "11/13010000", "o endereço do select");
                assert_eq!(param, "pp", "o campo recusado");
                assert_eq!(got, &format!("{pp:#06x}"), "o pp pedido, em hex");
                assert_eq!(
                    allowed, "0x0000..0x0062, 0x0100..0x0162",
                    "a faixa do aparelho, legível"
                );
            }
            outro => panic!("esperado ValueOutOfRange, veio {outro:?}"),
        }
    }

    // ZERO frames: a recusa aconteceu antes de qualquer byte existir.
    let enviadas = s.into_transport().0.transactions();
    assert_eq!(enviadas, 0, "nenhum select fora da faixa pode sair");
}

/// Dentro do inventário o select passa, muda o estado e o aparelho responde
/// (o mock ecoa o meta6 — o mesmo pareamento da captura).
#[test]
fn select_dentro_do_inventario_passa_e_o_aparelho_responde() {
    let mut s = sessao_aparelho();
    for pp in [0x0000u16, 0x0062, 0x0100, 0x0162] {
        s.select_preset(pp).expect("pp provado pela captura");
        assert_eq!(s.current_pp(), pp, "o pp corrente acompanha o select");
    }
    assert_eq!(s.into_transport().0.transactions(), 4, "4 selects no fio");
}

/// Os LIMITES da faixa: o último de cada banco entra, o primeiro depois
/// dele sai. É o `PresetNum < TOTAL_PA` virando regra de código.
#[test]
fn os_limites_dos_dois_bancos_sao_exatos() {
    let mut s = sessao_aparelho();
    s.select_preset(0x0062).expect("último do banco 0");
    assert!(
        s.select_preset(0x0063).is_err(),
        "primeiro inexistente do banco 0"
    );
    s.select_preset(0x0100).expect("primeiro do banco 1");
    s.select_preset(0x0162).expect("último do banco 1");
    assert!(
        s.select_preset(0x0163).is_err(),
        "primeiro inexistente do banco 1"
    );
}

/// **Critério 3 do lado do core: o mock não muda.** Sem `e_aparelho`, a
/// trava nem existe — o mesmo `pp` que o aparelho recusa passa aqui.
#[test]
fn o_mock_continua_aceitando_qualquer_pp() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("open");
    let mut s = Session::new(mock);
    s.select_preset(0x0063).expect("mock sem trava");
    s.select_preset(0xffff).expect("mock sem trava");
    assert_eq!(s.into_transport().transactions(), 2, "os dois saíram");
}

/// `set_inventory` manda na trava: a recusa acompanha o inventário que o
/// boot varre (é o caminho do "o aparelho descobriu pps novos" — R3 do
/// H1_CHECKLIST, sem código novo: quem define o scan define a faixa).
#[test]
fn a_trava_usa_o_inventario_fixado_nao_um_assumo_proprio() {
    let mut s = sessao_aparelho();
    s.set_inventory(vec![7]);
    s.select_preset(7).expect("no inventário fixado");
    let e = s.select_preset(8).expect_err("fora do inventário fixado");
    assert!(
        matches!(e, ProtocolError::ValueOutOfRange { .. }),
        "esperado ValueOutOfRange, veio {e:?}"
    );
}

/// **O boot do aparelho varre os 198 pps provados.** 2299 = a mesma conta
/// do replay da captura (40 tabelas + 198×11 + 2 do 0x0100 duplicado +
/// sonda 11 + estado5 5 + nomes 61 + keepalive 2) — ou seja, o aparelho
/// recebe a sequência do Suite, e não o `0..198` que mandava 99 selects
/// fora do aparelho.
#[test]
fn o_boot_do_aparelho_varre_os_198_pps_da_captura() {
    let mut s = sessao_aparelho();
    let r = s.boot().expect("boot completo contra o mock");
    assert_eq!(r.transactions, 2299, "a mesma contagem do replay da S1");
    assert_eq!(
        s.current_pp(),
        0x0062,
        "o scan termina no último pp do banco 0 (ordem da captura)"
    );
}
