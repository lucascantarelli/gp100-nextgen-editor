//! CONTRATO do upload de SnapTone/NAM (§5) sobre a FSM de sessão.
//!
//! **O que este arquivo prova, e por que precisa de um transporte próprio.**
//! O caminho do SnapTone tem ACK de 16B por bloco — o único fluxo de escrita
//! do projeto que o mock não dá para carregar só com `queue_push`, porque o
//! número de ACKs depende do número de blocos do modelo. Por isso há aqui um
//! transporte roteirizado ([`Roteiro`]): ele grava o que a FSM mandou e
//! devolve o que o teste mandar, na ordem. É ele que dá para provar (a) que a
//! FSM manda o stream **exato** que §2 especifica, (b) que ela espera um ACK
//! por bloco antes do próximo, (c) que um ACK de tamanho errado e um ACK
//! ausente viram erro TIPADO, e (d) que um push da família GP-100 no meio do
//! stream não é confundido com o ACK (D7).
//!
//! O `MockDevice` entra no complemento: ele remonta o modelo, o que prova a
//! entrega de ponta a ponta dos ~2,7 KB sem hardware.

mod common;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use gp100_core::session::{Session, SNAP_TONE_SETTLE_MS};
use gp100_core::snap_tone;
use gp100_core::transport::{DeviceTransport, MockDevice, TransportError, WireKind};
use gp100_core::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// Resposta ACK no framing da família com N bytes de corpo (N ≠ 16 = errado
/// por construção, que é o que o teste quer).
fn ack_de(n: usize) -> Vec<u8> {
    let mut m = vec![0xF0];
    m.extend(gp100_core::codec::nibble_expand(&vec![0xAAu8; n]));
    m.push(SYSEX_EOX);
    m
}

/// Resposta ACK com os 16 bytes CRU (sem framing) — o segundo caminho de
/// [`snap_tone::ack_body`].
fn ack_cru() -> Vec<u8> {
    vec![0x55u8; snap_tone::ACK_LEN]
}

/// Um envelope qualquer do GP-100 (vai para o backlog, D7).
fn push_gp100() -> Vec<u8> {
    let mut m = Vec::from(SYSEX_HEADER);
    m.push(0x12);
    m.extend_from_slice(&[0x12, 0x00, 0x00, 0x01]);
    m.extend_from_slice(&[0x01, 0x00, 0x00]);
    m.push(SYSEX_EOX);
    m
}

/// Transporte roteirizado: guarda o que a FSM enviou e devolve as respostas
/// do roteiro, uma por `recv_raw`. Roteiro vazio = nada responde (o
/// `recv_raw` devolve `RecvTimeout`, como um device mudo).
struct Roteiro {
    enviados: Vec<Vec<u8>>,
    respostas: VecDeque<Vec<u8>>,
    recv_calls: usize,
}

impl Roteiro {
    fn com(respostas: Vec<Vec<u8>>) -> Self {
        Self {
            enviados: Vec::new(),
            respostas: respostas.into(),
            recv_calls: 0,
        }
    }

    /// Roteiro que responde um ACK de família para todo bloco pedido.
    fn ack_por_bloco(n: usize) -> Self {
        Self::com((0..n).map(|_| ack_de(snap_tone::ACK_LEN)).collect())
    }

    fn sessao(&mut self) -> Session<&mut Self> {
        Session::new(self)
    }
}

impl DeviceTransport for Roteiro {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        self.enviados.push(data.to_vec());
        Ok(())
    }
    fn recv_raw(&mut self, _timeout: Duration) -> Result<Vec<u8>, TransportError> {
        self.recv_calls += 1;
        self.respostas
            .pop_front()
            .ok_or(TransportError::RecvTimeout { timeout_ms: 3000 })
    }
}

/// Modelo determinístico de N bytes (o conteúdo é opaco para o framing).
fn modelo(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + 3) as u8).collect()
}

// ─────────────────────────────── o contrato do fio ────────────────────────

/// O stream vai para o fio **exatamente** como §2 especifica: bloco 0 cheio
/// (48B), bloco 1 cheio (48B) e a cauda curta. Este é o teste que amarra a
/// FSM ao `wire_block` — sem ele, uma FSM que mandasse o envelope do GP-100
/// por cima do framing da família passaria em todo o resto.
#[test]
fn o_stream_vai_para_o_fio_exatamente_como_o_protocolo_diz() {
    // 40 bytes = 2 blocos de 19 + cauda de 2.
    let m = modelo(40);
    let mut r = Roteiro::ack_por_bloco(3);
    let rel = r
        .sessao()
        .upload_snap_tone_settled(1, &m, Duration::ZERO)
        .expect("upload com ACK em todo bloco");
    assert_eq!(rel.blocks, 3);
    assert_eq!(rel.acks, 3);
    assert_eq!(rel.bytes, 40);
    assert_eq!(rel.slot, 1);

    assert_eq!(r.enviados.len(), 3, "um frame por bloco, sem sobra");
    let esperado: Vec<Vec<u8>> = m
        .chunks(snap_tone::PAYLOAD_MAX)
        .enumerate()
        .map(|(i, c)| snap_tone::wire_block(i as u8, c).expect("bloco"))
        .collect();
    assert_eq!(r.enviados, esperado, "frame a frame, igual ao §2");
    // E os tamanhos são os do §2: 19B de payload = 48B no fio; a cauda de 2B
    // = 14B. Um payload de 18 passaria em tudo e mandaria 44B — o tamanho é
    // que denuncia.
    assert_eq!(
        r.enviados.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![48, 48, 14]
    );
}

/// Um `recv_raw` por bloco: a FSM ESPERA o ACK antes de mandar o próximo.
///
/// Se ela disparasse o stream inteiro e collected os ACKs depois, o device
/// receberia 143 blocos de uma vez e o settle (§4) deixaria de proteger
/// qualquer coisa — e o teste passaria, porque o total de ACKs seria o mesmo.
#[test]
fn um_ack_por_bloco_antes_do_proximo() {
    let mut r = Roteiro::ack_por_bloco(3);
    r.sessao()
        .upload_snap_tone_settled(1, &modelo(40), Duration::ZERO)
        .expect("upload");
    assert_eq!(r.recv_calls, 3, "3 blocos = 3 esperas, uma por bloco");
}

/// O ACK no caminho CRU (16 bytes sem framing) é aceito — o caminho da
/// família é o primário, mas um device que responde fora do envelope não pode
/// fazer o upload falhar (ver `snap_tone::ack_body`).
#[test]
fn ack_cru_de_16_bytes_tambem_e_aceito() {
    let mut r = Roteiro::com(vec![ack_cru(), ack_cru()]);
    let rel = r
        .sessao()
        .upload_snap_tone_settled(3, &modelo(20), Duration::ZERO)
        .expect("ACK cru aceito");
    assert_eq!(rel.acks, 2);
}

/// ACK de tamanho errado = erro TIPADO e o stream PARA ali (não segue
/// mandando blocos para o device depois de uma recusa).
#[test]
fn ack_de_tamanho_errado_aborta_o_upload() {
    let mut r = Roteiro::com(vec![ack_de(15), ack_de(snap_tone::ACK_LEN)]);
    let e = r
        .sessao()
        .upload_snap_tone_settled(1, &modelo(40), Duration::ZERO)
        .expect_err("ACK de 15B é recusa");
    assert!(
        matches!(e, ProtocolError::InvalidShape { .. }),
        "veio {e:?}"
    );
    assert_eq!(r.enviados.len(), 1, "o segundo bloco NÃO foi para o fio");
}

/// Device que não responde = `Timeout` (D6), não panic e não `InvalidShape`
/// genérico: a UI precisa distinguir "não respondeu" de "respondeu errado".
#[test]
fn ack_ausente_e_timeout_e_nao_panic() {
    let mut r = Roteiro::com(vec![]);
    let e = r
        .sessao()
        .upload_snap_tone_settled(1, &modelo(19), Duration::ZERO)
        .expect_err("sem ACK");
    assert!(
        matches!(&e, ProtocolError::Timeout { addr, .. } if addr == "snapTone/ack"),
        "veio {e:?}"
    );
}

/// D7 no caminho do SnapTone: um push do GP-100 intercalado **não** é o ACK.
/// Ele vai para o backlog e o upload continua — se a FSM aceitasse qualquer
/// mensagem como ACK, o primeiro bloco pararia esperando um push que não é
/// resposta dele, e o relatório mentiria sobre o device.
#[test]
fn push_gp100_intercalado_vai_para_o_backlog_e_nao_e_ack() {
    let mut r = Roteiro::com(vec![
        push_gp100(),
        ack_de(snap_tone::ACK_LEN),
        push_gp100(),
        ack_de(snap_tone::ACK_LEN),
        ack_de(snap_tone::ACK_LEN),
    ]);
    let mut s = r.sessao();
    let rel = s
        .upload_snap_tone_settled(1, &modelo(40), Duration::ZERO)
        .expect("push não é ACK");
    assert_eq!(rel.acks, 3, "só os ACKs contam como ACK");
    let backlog = s.pending_pushes().expect("backlog legível");
    assert_eq!(backlog.len(), 2, "os 2 pushes foram preservados (D7)");
    assert_eq!(backlog, vec![push_gp100(), push_gp100()]);
}

// ─────────────────────── pré-condições: nada vai ao fio ───────────────────

/// Slot fora de 1..=5 é erro e NADA vai para o fio. `SnapTone1..5` é o que as
/// strings do firmware dizem; aceitar o 0 (ou o 6) seria escrever num slot que
/// não existe, e o device responderia um erro que ninguém sabe ler.
#[test]
fn slot_fora_de_1_a_5_e_erro_antes_de_qualquer_byte() {
    assert_eq!(snap_tone::SLOTS, 5, "§5: SnapTone1..5 no firmware");
    for slot in [0u8, 6, 255] {
        let mut r = Roteiro::ack_por_bloco(1);
        let e = r
            .sessao()
            .upload_snap_tone_settled(slot, &modelo(19), Duration::ZERO)
            .expect_err("slot invalido");
        assert!(
            matches!(e, ProtocolError::InvalidShape { .. }),
            "slot {slot}: veio {e:?}"
        );
        assert!(
            r.enviados.is_empty(),
            "slot {slot}: nenhum byte foi enviado"
        );
    }
}

/// Os 5 slots são todos aceitos (o limite é 5, não 2 nem 4).
#[test]
fn os_cinco_slots_passam() {
    for slot in 1..=snap_tone::SLOTS {
        let mut r = Roteiro::ack_por_bloco(1);
        let rel = r
            .sessao()
            .upload_snap_tone_settled(slot, &modelo(19), Duration::ZERO)
            .expect("slot valido");
        assert_eq!(rel.slot, slot);
    }
}

/// Modelo vazio não é stream (`blocks` recusa) e nada vai ao fio.
#[test]
fn modelo_vazio_aborta_antes_do_fio() {
    let mut r = Roteiro::ack_por_bloco(1);
    let e = r
        .sessao()
        .upload_snap_tone_settled(1, &[], Duration::ZERO)
        .expect_err("modelo vazio");
    assert!(
        matches!(e, ProtocolError::InvalidShape { .. }),
        "veio {e:?}"
    );
    assert!(r.enviados.is_empty());
}

// ─────────────────── o mock: entrega de ponta a ponta ──────────────────────

/// Os ~2,7 KB do §5 atravessam o mock e voltam IGUAIS — 143 blocos, 143 ACKs,
/// zero divergência. Este é o teste que dispensa hardware para a maior parte
/// do risco: se a ordem dos blocos ou o tamanho do payload estivessem errados,
/// o modelo remontado não seria o modelo.
#[test]
fn o_modelo_de_2700_bytes_atravessa_o_mock_igual() {
    let m = modelo(2700);
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let rel = {
        let mut s = Session::new(&mut dev);
        s.upload_snap_tone_settled(2, &m, Duration::ZERO)
            .expect("upload dos 143 blocos")
    };
    assert_eq!(rel.blocks, 143, "142 cheios + 1 de 2 bytes (§5)");
    assert_eq!(rel.acks, 143, "ACK de 16B por bloco (§5)");
    assert_eq!(rel.slot, 2);
    let (remontado, transferencias) = dev.snap_tone();
    assert_eq!(remontado, m.as_slice(), "o modelo chega inteiro e em ordem");
    assert_eq!(transferencias, 1, "uma transferência só");
    assert_eq!(dev.state().snap_tone_acks, 143);
    assert_eq!(
        dev.state().rejected,
        0,
        "nenhum bloco recusado (CRC confere)"
    );
}

/// Duas transferências seguidas (dois slots): o índice 0 reabre o stream, e o
/// mock guarda a ÚLTIMA. Sem isso, o teste anterior passaria também num
/// upload que acumulasse tudo num stream só.
#[test]
fn duas_transferencias_seguidas_nao_se_misturam() {
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let (a, b) = (modelo(100), modelo(37));
    for (slot, m) in [(1u8, &a), (4u8, &b)] {
        let mut s = Session::new(&mut dev);
        s.upload_snap_tone_settled(slot, m, Duration::ZERO)
            .expect("upload");
    }
    let (remontado, transferencias) = dev.snap_tone();
    assert_eq!(remontado, b.as_slice(), "o stream corrente é o último");
    assert_eq!(transferencias, 2);
}

/// O mock recusa bloco com CRC errado (D5). Isto amarra o wire_block ao que o
/// device valida: se o CRC do BUF saísse errado, o upload inteiro falharia no
/// primeiro bloco — e o teste do modelo de 2700 bytes acusaria.
#[test]
fn bloco_com_crc_errado_e_recusado() {
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let mut w = snap_tone::wire_block(0, &[1, 2, 3]).expect("bloco");
    // Vira um nibble do payload: o CRC gravado deixa de valer.
    w[10] ^= 0x01;
    let e = dev
        .send_raw(&w, WireKind::Write)
        .expect_err("CRC invalido e recusa");
    assert!(matches!(e, TransportError::SendFailed { .. }), "veio {e:?}");
    assert_eq!(dev.state().rejected, 1);
    assert_eq!(dev.state().snap_tone_acks, 0, "nenhum ACK para bloco ruim");
}

/// O `length` do BUF é o que o device lê: um `length` mentiroso faz o mock
/// recusar (e o device real leria o bloco errado) — a checagem é o que impede
/// o truncamento silencioso de passar.
#[test]
fn length_mentiroso_e_recusado() {
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    // BUF à mão: [crc, 0x92, 0, length=9] + 3 bytes de payload.
    let mut b = vec![0u8, snap_tone::CMD_SNAP_TONE, 0, 9, 1, 2, 3];
    let crc = snap_tone::crc8(&b);
    b[0] = crc;
    let mut w = vec![0xF0];
    w.extend(gp100_core::codec::nibble_expand(&b));
    w.push(SYSEX_EOX);
    let e = dev
        .send_raw(&w, WireKind::Write)
        .expect_err("length != payload");
    assert!(matches!(e, TransportError::SendFailed { .. }), "veio {e:?}");
    assert!(e.to_string().contains("length 9 != payload de 3"));
}

/// O relatório de envio é o que volta pela ponte IPC, e o contrato do fio
/// (camelCase/plain, sem wrapper) é testado AQUI — no crate que compila nas
/// três plataformas — em vez de no crate do Tauri, que só a CI do Windows
/// constrói. Um campo renomeado aqui passaria pelo typecheck do Rust e
/// quebraria no `invoke` da tela.
#[test]
fn o_relatorio_de_envio_serializa_com_o_contrato_do_fio() {
    let rel = gp100_core::session::SnapToneUploadReport {
        slot: 3,
        blocks: 143,
        acks: 143,
        bytes: 2700,
    };
    let json = serde_json::to_value(&rel).expect("serializável");
    assert_eq!(json["slot"], 3);
    assert_eq!(json["blocks"], 143);
    assert_eq!(json["acks"], 143);
    assert_eq!(json["bytes"], 2700);
    // E o relatório de um modelo de 2,7 KB bate com o §5: um ACK por bloco.
    assert_eq!(rel.acks, rel.blocks, "um ACK por bloco (§5)");
}

// ─────────────────────────────── o settle de §4 ───────────────────────────

/// O settle de §4 EXISTE no caminho de produção: entre dois blocos há pelo
/// menos `settle` de relógio. Medir o tempo é o jeito de provar um sleep —
/// um contador de ACKs não o provaria, porque o número de ACKs é o mesmo
/// com ou sem espera.
///
/// 5 blocos => 4 settles: o último não espera porque não há operação
/// seguinte (e um teste que pagasse 250 ms à toa no fim seria lento sem
/// provar nada).
#[test]
fn o_settle_entre_blocos_e_real() {
    let settle = Duration::from_millis(40);
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let m = modelo(5 * snap_tone::PAYLOAD_MAX); // 5 blocos cheios
    let ini = Instant::now();
    {
        let mut s = Session::new(&mut dev);
        s.upload_snap_tone_settled(1, &m, settle).expect("upload");
    }
    let decorrido = ini.elapsed();
    let minimo = settle * 4;
    assert!(
        decorrido >= minimo,
        "5 blocos deviam custar ao menos {minimo:?} de settle, custou {decorrido:?}"
    );
}

/// O piso de produção são 250 ms (`SNAP_TONE_SETTLE_MS`) e o upload de um
/// bloco só não paga settle nenhum — o que mantém o caminho de uso comum
/// (substituir o tom de um preset) instantâneo.
#[test]
fn o_settle_de_producao_e_o_piso_de_250ms_e_um_bloco_nao_paga_settle() {
    assert_eq!(
        SNAP_TONE_SETTLE_MS, 250,
        "§4 regra 1: 0,15s corrompe o stream e loop tight trava a pedaleira"
    );
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let ini = Instant::now();
    let rel = {
        let mut s = Session::new(&mut dev);
        s.upload_snap_tone(1, &modelo(4))
            .expect("upload de 1 bloco")
    };
    assert_eq!(rel.blocks, 1);
    assert!(
        ini.elapsed() < Duration::from_millis(SNAP_TONE_SETTLE_MS * 2),
        "1 bloco não espera 250ms: levou {:?}",
        ini.elapsed()
    );
}
