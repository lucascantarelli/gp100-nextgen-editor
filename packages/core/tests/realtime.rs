//! Contratos de TEMPO REAL do MockDevice — latência com jitter + USB intermitente.
//!
//! O mock SEMPRE devolveu `RecvTimeout` instantâneo com a fila vazia: o falso
//! positivo de tempo era código testado contra ele que assumia resposta
//! imediata e sobrevivia ao aparelho real, que tem latência de fio e janela de
//! 3s (ADR-3). Estes contratos fecham o falso positivo sem quebrar a suíte:
//!
//!   1. latência de ida E de volta medida no relógio (piso E teto da faixa);
//!   2. `recv` VAZIO com latência optada espelha o `real.rs` — espera a
//!      janela inteira antes de acusar silêncio;
//!   3. sem o opt-in nenhum sleep acontece (default dos 500+ testes intacto);
//!   4. USB instável: `SendFailed` transitório na n-ésima tentativa, device
//!      vivo, e a tentativa seguinte passa;
//!   5. resposta perdida no fio: a mensagem some da fila (o host vê o
//!      silêncio) e a seguinte chega — recuperação sem reabrir nada.
//!
//! Todos os planos são contra-contados (sem aleatoriedade): a única
//! aleatoriedade é o jitter, e ele vem de um LCG semeado — determinismo
//! provado no `mod tests` do próprio `mock.rs`.

use gp100_core::transport::mock::{MockDevice, MockFault};
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};
use gp100_core::{SYSEX_EOX, SYSEX_HEADER};
use std::time::{Duration, Instant};

/// Frame de SELECT válido no formato do envelope do GP-100
/// (`F0 21 25 7F 47 50 2D 64 | FUNC | ADDR | payload | F7`): `11/13010000`
/// com o pp em BE — o mesmo que a `Session` manda e que o mock reconhece
/// (ingest de `(0x11, [13,01,00,00])` com payload de 2B).
fn frame_select(pp: u16) -> Vec<u8> {
    let mut m = Vec::with_capacity(SYSEX_HEADER.len() + 1 + 4 + 2 + 1);
    m.extend_from_slice(&SYSEX_HEADER);
    m.push(0x11);
    m.extend_from_slice(&[0x13, 0x01, 0x00, 0x00]);
    m.extend_from_slice(&pp.to_be_bytes());
    m.push(SYSEX_EOX);
    m
}

/// A latência de IDA (send) e a de VOLTA (recv) são DUAS dormidas na faixa:
/// o relógio enxerga o piso em cada lado, e o teto não estoura (folga de CI
/// para o scheduler — o relógio do teste não é o do runner).
#[test]
fn latencia_de_ida_e_de_volta_medidas_no_relogio() {
    let min = Duration::from_millis(30);
    let max = Duration::from_millis(60);
    let folga = Duration::from_millis(500);
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_latency(min, max);
    dev.open().expect("open com device presente");

    // IDA: o device recebe os bytes depois do jitter…
    let t0 = Instant::now();
    dev.send_raw(&frame_select(3), WireKind::Read)
        .expect("select válido passa (só demora, não falha)");
    let ida = t0.elapsed();
    assert!(ida >= min, "piso da ida: {ida:?} < {min:?}");
    assert!(ida <= max + folga, "teto da ida: {ida:?} > {max:?} + folga");

    // …VOLTA: a resposta (meta6) chega ao host depois do outro jitter.
    let t0 = Instant::now();
    dev.recv_raw(Duration::from_secs(5))
        .expect("a resposta do select está na fila");
    let volta = t0.elapsed();
    assert!(volta >= min, "piso da volta: {volta:?} < {min:?}");
    assert!(
        volta <= max + folga,
        "teto da volta: {volta:?} > {max:?} + folga"
    );
}

/// Com latência optada, a fila vazia espelha o `real.rs`: o silêncio só é
/// acusado DEPOIS da janela da chamada — é o coração do fechamento do falso
/// positivo (antes: 3s de janela resolvidos em microssegundos).
#[test]
fn recv_vazio_com_latencia_respeita_a_janela_inteira() {
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_latency(Duration::from_millis(10), Duration::from_millis(20));
    dev.open().expect("open");

    let janela = Duration::from_millis(80);
    let t0 = Instant::now();
    let err = dev
        .recv_raw(janela)
        .expect_err("fila vazia => silêncio, não invenção de mensagem");
    let dt = t0.elapsed();

    assert!(
        matches!(err, TransportError::RecvTimeout { .. }),
        "silêncio tipado: {err:?}"
    );
    assert!(dt >= janela, "janela respeitada: {dt:?} < {janela:?}");
}

/// A prova do DEFAULT: sem `with_latency` não existe um único sleep no mock.
/// Quem rodar `recv` com janela de 5s numa fila vazia volta na hora — é o
/// comportamento do qual os 500+ testes existentes dependem.
#[test]
fn sem_opt_in_o_mock_continua_instantaneo() {
    let mut dev = MockDevice::new().expect("mock montado");
    dev.open().expect("open");

    let t0 = Instant::now();
    let err = dev
        .recv_raw(Duration::from_secs(5))
        .expect_err("fila vazia => RecvTimeout");
    let dt = t0.elapsed();

    assert!(matches!(err, TransportError::RecvTimeout { .. }), "{err:?}");
    assert!(
        dt < Duration::from_millis(250),
        "sem latência optada o mock NÃO dorme: {dt:?}"
    );
}

/// USB instável na camada de envio: o n-ésimo `send_raw` falha UMA vez com
/// `SendFailed` transitório, o device segue vivo (`transient_failures`
/// contabiliza) e a tentativa seguinte passa — exatamente o ciclo
/// falha→retry que a ADR-4 prometeu distinguir de `DeviceGone`.
#[test]
fn usb_flaky_falha_transitoria_na_nvez_e_a_seguinte_passa() {
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_fault(MockFault::UsbFlaky {
            send_every: 3,
            drop_every: 0,
        });
    dev.open().expect("open");

    dev.send_raw(&frame_select(3), WireKind::Read)
        .expect("envio 1 passa");
    dev.send_raw(&frame_select(4), WireKind::Read)
        .expect("envio 2 passa");

    let err = dev
        .send_raw(&frame_select(5), WireKind::Read)
        .expect_err("envio 3 é múltiplo de 3 — o SO recusa o buffer");
    match err {
        TransportError::SendFailed { ref why } => {
            assert!(why.contains("USB instável"), "why nomeia a falha: {why}");
            assert!(
                why.contains("MockFault::UsbFlaky"),
                "why cita o plano: {why}"
            );
        }
        outro => panic!("SendFailed transitório esperado, veio {outro:?}"),
    }
    assert_eq!(
        dev.transient_failures(),
        1,
        "exatamente UMA falha transitória contabilizada"
    );

    // device VIVO: a tentativa seguinte passa e o recv segue entregando as
    // respostas dos envios que chegaram (1 e 2 enfileiraram meta6).
    dev.send_raw(&frame_select(6), WireKind::Read)
        .expect("envio 4 sai do múltiplo e passa");
    dev.recv_raw(Duration::from_millis(50))
        .expect("resposta do envio 1 chega — device segue servindo");
}

/// USB instável na camada de recepção: a n-ésima resposta é PERDIDA no fio.
/// A prova é a fila encurtar — a mensagem B some (o host vê o silêncio) e C
/// chega em seguida; nada de enfileirar de novo ou reenviar (D6: retry é
/// política de camada acima).
#[test]
fn usb_flaky_perde_resposta_no_fio_e_a_seguinte_chega() {
    let mut dev = MockDevice::new()
        .expect("mock montado")
        .with_fault(MockFault::UsbFlaky {
            send_every: 0,
            drop_every: 2,
        });
    dev.open().expect("open");

    // Três pushes distintos: A entrega, B some, C entrega.
    dev.queue_push(0x12, [0x00, 0x00, 0x00, 0x01], &[0xAA]);
    dev.queue_push(0x12, [0x00, 0x00, 0x00, 0x01], &[0xBB]);
    dev.queue_push(0x12, [0x00, 0x00, 0x00, 0x01], &[0xCC]);

    let a = dev
        .recv_raw(Duration::from_millis(50))
        .expect("1ª chega (entrega 1)");
    assert_eq!(a[a.len() - 2], 0xAA, "payload A no fim do envelope");

    let err = dev
        .recv_raw(Duration::from_millis(50))
        .expect_err("2ª é PERDIDA no fio — o host vê RecvTimeout");
    assert!(
        matches!(err, TransportError::RecvTimeout { .. }),
        "silêncio tipado: {err:?}"
    );

    let c = dev
        .recv_raw(Duration::from_millis(50))
        .expect("3ª chega — a perda não derruba o device nem a fila");
    assert_eq!(c[c.len() - 2], 0xCC, "payload C — B não reapareceu");

    // A fila foi consumida de verdade (B saiu, não ficou pendurada).
    dev.recv_raw(Duration::from_millis(50))
        .expect_err("fila vazia depois das três");
}
