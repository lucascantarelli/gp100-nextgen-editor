//! `value_gate.rs` — a trava de CONTEÚD do ADR-10 se prova por MUTAÇÃO
//! (#110).
//!
//! **O perigo que esta pasta fecha.** Em 05/10/2026, no gate H2, o runbook
//! mandava `set-param 3 0x0700006e 0 99.5`. O `99.5` era um número
//! inventado. O firmware V2.1 tem, em `Drivers/audio/audio.c:1828`, o
//! assert `para <= GetParaMaxVal(` e, ao assertar, **para de responder a
//! toda transação** — inclusive às leituras. O device continua enumerado e
//! `OK` no Windows, o que elimina cabo/driver/porta; a recuperação é um
//! **power-cycle físico**. E como `set-param` é fire-and-forget (§13.11, D4),
//! **o fio não dá nenhum aviso**: o operador descobre que quebrou o pedal
//! quando a leitura seguinte toma timeout.
//!
//! Por isso "o codec devolveu `Err`" **não** basta como prova. Um gate que
//! só validasse o valor de retorno passaria mesmo com a validação happening
//! *depois* do envio. Estes testes usam um transporte que **pune qualquer
//! byte**, então a única forma de passarem é a recusa ter acontecido antes
//! do `send_raw`.
//!
//! **O que também é testado:** que a trava não barra o que a Suite de fato
//! manda. Os 92 frames reais de `analysis/fixtures/knobs.jsonl` têm de
//! passar; um gate que recusa valor legítimo é tão quebrado quanto um que
//! não recusa valor perigoso.

use std::time::Duration;

use gp100_core::codec::{set_param, set_param_payload};
use gp100_core::session::Session;
use gp100_core::transport::{DeviceTransport, MockDevice, TransportError, WireKind};

/// Transporte que FALHA se qualquer byte sair. É o instrumento do teste:
/// um `Err` aqui é "um byte escapou", que é exatamente o que a trava
/// precisa impedir.
struct ByteGuard {
    /// Frames que saíram (deve ficar VAZIO no caminho da recusa).
    escapados: Vec<Vec<u8>>,
}

impl ByteGuard {
    fn new() -> Self {
        Self {
            escapados: Vec::new(),
        }
    }
}

impl DeviceTransport for ByteGuard {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        self.escapados.push(data.to_vec());
        Ok(())
    }
    fn recv_raw(&mut self, _timeout: Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::RecvTimeout { timeout_ms: 0 })
    }
}

/// **O teste que a #110 pede, literalmente:** valor fora da faixa é
/// recusado e NENHUM byte sai do transporte.
///
/// O `99.5` é o valor exato do runbook que derrubou o aparelho.
#[test]
fn valor_fora_da_faixa_nao_sai_um_byte() {
    let mut guard = ByteGuard::new();
    guard.open().expect("abre");
    let mut s = Session::new(guard);

    let e = s
        .set_param(3, 0x0700_006e, 0, 99.5)
        .expect_err("99.5 acima do teto 99 do dicionario");
    let msg = e.to_string();

    let guard = s.into_transport();
    assert!(
        guard.escapados.is_empty(),
        "ZERO bytes podem sair: escaparam {}",
        guard.escapados.len()
    );
    assert!(msg.contains("10030002"), "a mensagem diz o endereco: {msg}");
    assert!(
        msg.contains("0x0700006e/0"),
        "a mensagem nomeia o knob: {msg}"
    );
    assert!(
        msg.contains("faixa 0 .. 99"),
        "a mensagem diz o teto: {msg}"
    );
    assert!(
        msg.contains("99.5"),
        "a mensagem repete o valor pedido: {msg}"
    );
}

/// O valor REAL da mesma captura (15.0, vetor do `knobs.jsonl` linha 1)
/// sai — a trava não pode barrar o que o aparelho de fato recebeu.
#[test]
fn valor_real_da_captura_sai() {
    let mut guard = ByteGuard::new();
    guard.open().expect("abre");
    let mut s = Session::new(guard);

    s.set_param(3, 0x0700_006e, 0, 15.0)
        .expect("15.0 = vetor real");

    let guard = s.into_transport();
    assert_eq!(guard.escapados.len(), 1, "um frame saiu");
    let frame = &guard.escapados[0];
    assert_eq!(frame[0], 0xF0, "SysEx Start");
    assert_eq!(frame[frame.len() - 1], 0xF7, "SysEx EOX");
    let (code, ctrl, value) =
        gp100_core::codec::set_param_parse(&frame[13..frame.len() - 1]).expect("payload de volta");
    assert_eq!((code, ctrl), (0x0700_006e, 0));
    assert_eq!(value, 15.0);
}

/// `NaN`/`inf` recusados ANTES do transporte, com ou sem regra no
/// dicionário: o bit-pattern de `f32` para eles é lixo que nenhum firmware
/// aceitaria, e mandá-lo é indistinguível de corrupção.
#[test]
fn nao_finito_nao_sai_um_byte() {
    let mut guard = ByteGuard::new();
    guard.open().expect("abre");
    let mut s = Session::new(guard);

    for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        s.set_param(3, 0x0700_006e, 0, v).expect_err("nao-finito");
        // Par SEM regra: mesmo assim recusado.
        s.set_param(5, 0x0a00_002c, 1, v)
            .expect_err("nao-finito sem regra");
    }
    let guard = s.into_transport();
    assert!(
        guard.escapados.is_empty(),
        "ZERO bytes podem sair: escaparam {}",
        guard.escapados.len()
    );
}

/// O par SEM regra (o knob do CAB `0x0a00002c`/`ctrl 1`, que o aparelho
/// varreu de 0 a 99 e o dicionário não descreve) **passa** — recusar
/// quebraria um knob que comprovadamente funciona.
#[test]
fn par_sem_regra_no_dicionario_passa() {
    let mut guard = ByteGuard::new();
    guard.open().expect("abre");
    let mut s = Session::new(guard);

    s.set_param(5, 0x0a00_002c, 1, 0.0).expect("piso observado");
    s.set_param(5, 0x0a00_002c, 1, 99.0)
        .expect("teto observado");

    let guard = s.into_transport();
    assert_eq!(guard.escapados.len(), 2, "os dois frames saem");
}

/// Slot fora de 1..=9 continua recusado como `InvalidShape` — a trava de
/// valor não engoliu a de shape (as duas são pré-`send_raw`, mas por
/// motivos e mensagens diferentes).
#[test]
fn slot_invalido_continua_sendo_shape() {
    let mut guard = ByteGuard::new();
    guard.open().expect("abre");
    let mut s = Session::new(guard);

    let e = s
        .set_param(10, 0x0700_006e, 0, 15.0)
        .expect_err("slot 10 nao existe");
    assert!(e.to_string().contains("1..=9"), "{e}");
    let guard = s.into_transport();
    assert!(guard.escapados.is_empty(), "nenhum byte");
}

/// O mock aceita o valor dentro da faixa e guarda no estado: a trava é de
/// **conteúdo**, não de política — ela vale igual no mock e no aparelho
/// real, e é por isso que o H1 (read-only) continua seguro por construção.
#[test]
fn o_mock_aceita_e_guarda_o_valor_dentro_da_faixa() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("abre");
    let mut s = Session::new(mock);

    s.set_param(3, 0x0700_006e, 0, 15.0)
        .expect("dentro da faixa");
    let mock = s.into_transport();
    // Chave do estado do mock = `(nibble, ctrl)` -> `(code, value)`.
    assert_eq!(
        mock.state().set_params.get(&(0x07, 0)),
        Some(&(0x0700_006e, 15.0))
    );

    // Recusa o segundo valor: o mock precisa ficar com o 15.0, porque a
    // trava acontece ANTES do transporte — se devolvesse, o estado do mock
    // mostraria o 99.5 e o teste de "nenhum byte saiu" seria mentira.
    let mut s = Session::new(mock);
    s.set_param(3, 0x0700_006e, 0, 99.5)
        .expect_err("fora da faixa");
    let mock = s.into_transport();
    assert_eq!(
        mock.state().set_params.get(&(0x07, 0)),
        Some(&(0x0700_006e, 15.0)),
        "o valor recusado NAO pode ter substituido o bom"
    );
}

/// `set_param_payload` — a função pura do codec — recusa pelo mesmo
/// caminho, o que prova que a trava não está só no `Session`.
#[test]
fn o_codec_puro_tambem_recusa() {
    assert!(set_param_payload(3, 0x0700_006e, 0, 99.5).is_err());
    assert!(set_param_payload(3, 0x0700_006e, 0, 15.0).is_ok());
    assert!(set_param(3, 0x0700_006e, 0, 99.5).is_err());
}
