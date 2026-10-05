//! CONTRATO do laboratório de IRs (§13.7 upload + §13.12 tabela) sobre a FSM.
//!
//! O `upload_ir` já existia e já era coberto no nível do TRANSPORTE
//! (`mock_device.rs`) e do replay de campo (`replay_fixtures.rs`). O que
//! falta — e é o que este arquivo prova — é o contrato do ponto de vista de
//! quem vai **usar** a FSM: o que a tela pode mandar, o que ela recebe de
//! volta e o que nunca pode chegar ao fio.
//!
//! **Por que as pre-condições são o centro deste arquivo.** O dono escolhe o
//! arquivo no disco (`<input type="file">`), e o que chega ali é o que é: um
//! `.ir` truncado, um WAV que ele renomeou, um arquivo de 0 bytes. Um
//! `chunks[chunks.len() - 1]` com lista vazia não dá erro — dá PANIC, e o
//! panic do lado do Tauri derruba o app inteiro sem mensagem. O mesmo vale
//! para o slot: escrever no slot 20 (que não existe) faria o aparelho
//! responder um erro que ninguém sabe ler.
//!
//! **Por que o transporte roteirizado aparece aqui também.** O caminho do IR
//! tem ACK por chunk cujo *conteúdo* depende do chunk (slot + idx), então um
//! ACK genérico passaria num upload que mandasse os índices errados. Com um
//! roteiro, o teste afirma o idx exato de cada frame.

mod common;

use std::collections::VecDeque;
use std::time::Duration;

use gp100_core::codec::{ir_begin, ir_chunk, ir_chunk_ack_payload};
use gp100_core::session::{IrUploadReport, Session};
use gp100_core::transport::{DeviceTransport, MockDevice, TransportError, WireKind};
use gp100_core::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

/// Os 20 slots de User IR do firmware (`<ppIRInfo0..19>`, §13.12).
const SLOTS: u8 = 20;

/// Bytes por chunk no fio (§13.7): 15 bytes reais = 30 nibbles.
const CHUNK: usize = 15;

/// Transporte roteirizado: grava o que a FSM mandou e devolve o roteiro.
/// Roteiro vazio = device mudo (`RecvTimeout`).
struct Roteiro {
    enviados: Vec<Vec<u8>>,
    respostas: VecDeque<Vec<u8>>,
}

impl Roteiro {
    fn com(respostas: Vec<Vec<u8>>) -> Self {
        Self {
            enviados: Vec::new(),
            respostas: respostas.into(),
        }
    }

    /// Roteiro que responde o ACK CORRETO de cada chunk de um blob: o
    /// `idx` é o que a FSM mandou, e a resposta é montada a partir dele — um
    /// ACK genérico deixaria um índice errado passar.
    fn ack_por_chunk(slot: u8, n_chunks: usize) -> Self {
        let idx_de = |i: usize| -> u16 { ((i / 128) as u16) * 256 + (i % 128) as u16 };
        // n_chunks ACKs de dados + 1 do marcador (o último duplicado)
        let mut respostas: Vec<Vec<u8>> = (0..n_chunks)
            .map(|i| envelope(ir_chunk_ack_payload(slot, idx_de(i))))
            .collect();
        respostas.push(envelope(ir_chunk_ack_payload(slot, idx_de(n_chunks - 1))));
        Self::com(respostas)
    }

    fn sessao(&mut self) -> Session<&mut Self> {
        Session::new(self)
    }
}

/// Envelope do GP-100 (`F0` … `F7`) com um payload.
fn envelope(payload: impl AsRef<[u8]>) -> Vec<u8> {
    let mut m = Vec::from(SYSEX_HEADER);
    m.push(0x12);
    m.extend_from_slice(&[0x12, 0x00, 0x10, 0x02]);
    m.extend_from_slice(payload.as_ref());
    m.push(SYSEX_EOX);
    m
}

/// Um push qualquer do GP-100 (vai para o backlog, D7).
fn push_gp100() -> Vec<u8> {
    let mut m = Vec::from(SYSEX_HEADER);
    m.push(0x12);
    m.extend_from_slice(&[0x12, 0x00, 0x00, 0x01]);
    m.extend_from_slice(&[0x01, 0x00, 0x00]);
    m.push(SYSEX_EOX);
    m
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
        self.respostas
            .pop_front()
            .ok_or(TransportError::RecvTimeout { timeout_ms: 3000 })
    }
}

/// Blob determinístico de N bytes, múltiplo de [`CHUNK`] quando pedido.
fn blob(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + 3) as u8).collect()
}

// ───────────────────── o que a ponte IPC recebe (o contrato) ───────────────

/// O relatório é o que volta pelo `invoke` e o que a tela mostra. O contrato
/// do fio é testado AQUI, no crate que compila nas três plataformas da
/// matriz — o crate do Tauri só a CI do Windows constrói. Um campo virando
/// `chunk_count` passaria pelo typecheck do Rust e apareceria `undefined` na
/// tela, sem erro em nenhum lugar.
#[test]
fn o_relatorio_serializa_com_o_contrato_do_fio() {
    let rel = IrUploadReport {
        slot: 7,
        chunks: 295,
        acks: 296,
        bytes: 15 * 295,
    };
    let json = serde_json::to_value(&rel).expect("serializável");
    assert_eq!(json["slot"], 7);
    assert_eq!(json["chunks"], 295);
    assert_eq!(json["acks"], 296);
    assert_eq!(json["bytes"], 15 * 295);
    assert!(
        json.get("chunk_count").is_none() && json.get("Chunk").is_none(),
        "o nome do campo é o contrato: {json}"
    );
    // e volta: a UI reidrata o relatório sem perder nada
    let de_volta: IrUploadReport = serde_json::from_value(json).expect("desserializável");
    assert_eq!(de_volta, rel);
}

// ──────────────── pré-condições: nada disso pode chegar ao fio ─────────────

/// Slot 20 é o primeiro INVÁLIDO: `<ppIRInfo0..19>` são 20 tags (§13.12).
/// O slot 0 é VÁLIDO aqui (ao contrário do SnapTone, que começa em 1) — e um
/// teste que aceitasse só 1..=19 passaria com o 0 recusado, que é o erro que
/// apareceria primeiro na tela do dono.
#[test]
fn os_vinte_slots_passam_e_o_vinte_e_um_e_recusado() {
    for slot in 0..SLOTS {
        let mut r = Roteiro::ack_por_chunk(slot, 1);
        let rel = r
            .sessao()
            .upload_ir(slot, &blob(CHUNK))
            .expect("slot válido");
        assert_eq!(rel.slot, slot);
    }
    for slot in [20u8, 99, 255] {
        let mut r = Roteiro::ack_por_chunk(0, 1);
        let e = r
            .sessao()
            .upload_ir(slot, &blob(CHUNK))
            .expect_err("slot inexistente");
        assert!(
            matches!(e, ProtocolError::InvalidShape { .. }),
            "slot {slot}: veio {e:?}"
        );
        assert!(
            r.enviados.is_empty(),
            "slot {slot}: nenhum byte foi para o fio"
        );
    }
}

/// **Blob de 0 bytes é PANIC** se a FSM não recusar: `chunks` fica vazio e o
/// marcador de fim indexa `chunks[len - 1]`.
///
/// Este teste existe porque o arquivo vem do disco escolhido pelo dono — um
/// `.ir` de 0 bytes é um clique errado, e o panic do lado do Tauri não mostra
/// erro: mata o app. O que a tela precisa é de um erro que ela translate.
#[test]
fn blob_de_zero_bytes_e_recusado_e_nao_entra_no_fio() {
    let mut r = Roteiro::ack_por_chunk(0, 1);
    let e = r
        .sessao()
        .upload_ir(0, &[])
        .expect_err("blob vazio");
    assert!(
        matches!(e, ProtocolError::InvalidShape { .. }),
        "veio {e:?}"
    );
    assert!(r.enviados.is_empty(), "nem o BEGIN foi para o fio");
}

/// O pedaço final é REJEITADO, não padado (strict, rev.2 do ADR-6): o device
/// real conta os chunks pelo payload e um chunk de 7 bytes faz o contador
/// divergir do resto do arquivo — o sintoma é um IR que "grava" e depois soa
/// como lixo.
#[test]
fn blob_que_nao_e_multiplo_de_15_e_recusado() {
    for n in [1usize, 7, 14, 16, 31] {
        let mut r = Roteiro::ack_por_chunk(0, 1);
        let e = r.sessao().upload_ir(0, &blob(n)).expect_err("cauda");
        assert!(
            matches!(e, ProtocolError::InvalidShape { .. }),
            "{n} bytes: veio {e:?}"
        );
        assert!(r.enviados.is_empty(), "{n} bytes: nada foi enviado");
    }
}

// ─────────────────────────── o stream frame a frame ────────────────────────

/// O BEGIN é o primeiro frame e reserva o slot; depois, UM frame por chunk, e
/// o último chunk vai DUPLICADO (o marcador de fim é a duplicação em si —
/// §13.7 corrigido, 296 sends/ACKs para 295 chunks na captura da S2).
///
/// O teste compara frame a frame com o codec: é ele que amarra a FSM ao
/// `ir_chunk` e impede uma FSM que "quase" manda o stream certo.
#[test]
fn o_stream_vai_frame_a_frame_e_o_ultimo_chunk_e_duplicado() {
    let b = blob(CHUNK * 4); // 4 chunks
    let mut r = Roteiro::ack_por_chunk(2, 4);
    let rel = r.sessao().upload_ir(2, &b).expect("upload");

    assert_eq!(rel.chunks, 4, "chunks ÚNICOS");
    assert_eq!(rel.acks, 5, "4 + o ACK do marcador duplicado");
    assert_eq!(rel.bytes, 60);
    assert_eq!(rel.slot, 2);

    let chunks = b.as_chunks::<CHUNK>().0;
    let mut esperado = vec![ir_begin(2).expect("begin")];
    for (i, c) in chunks.iter().enumerate() {
        esperado.push(ir_chunk(2, i as u16, c).expect("chunk"));
    }
    // marcador: o ÚLTIMO chunk, payload REAL, de novo (§13.7)
    esperado.push(ir_chunk(2, 3, &chunks[3]).expect("marcador"));
    assert_eq!(r.enviados, esperado, "frame a frame, igual ao §13.7");
}

/// Os índices andam em PÁGINAS de 128 com lacunas (0-127, 256-383, …): é o
/// que impede um byte `F7` cru no meio do SysEx e truncar a mensagem. Um
/// índice contínuo (128, 129…) é o erro que passaria em qualquer contagem e
/// só apareceria com o aparelho na mão.
#[test]
fn os_indices_vigem_em_paginas_de_128() {
    let b = blob(CHUNK * 129); // cruza a fronteira da 1ª página
    let mut r = Roteiro::ack_por_chunk(0, 129);
    let rel = r.sessao().upload_ir(0, &b).expect("upload");
    assert_eq!(rel.chunks, 129);

    // frame de dados #128 (0-based) tem idx 256, e o #127 tem 127.
    let chunks = b.as_chunks::<CHUNK>().0;
    assert_eq!(
        r.enviados[1 + 127],
        ir_chunk(0, 127, &chunks[127]).expect("chunk 127"),
        "o último da 1ª página é o 127"
    );
    assert_eq!(
        r.enviados[1 + 128],
        ir_chunk(0, 256, &chunks[128]).expect("chunk 256"),
        "o primeiro da 2ª página é o 256 — a lacuna 128-255 é o que salva o F7"
    );
}

/// ACK de OUTRO chunk é erro TIPADO e o stream PARA ali: continuar mandando
/// depois de um ACK que não é o do chunk que foi enviado deixa o aparelho com
/// um arquivo pela metade e o relatório dizendo que deu certo.
#[test]
fn ack_de_outro_chunk_aborta_o_upload() {
    // o roteiro responde com o idx 5 para os ACKs — o primeiro chunk (idx 0)
    // já recebe um ACK que não é o dele.
    let ack_errado = envelope(ir_chunk_ack_payload(1, 5));
    let mut r = Roteiro::com(vec![ack_errado.clone(), ack_errado]);
    let e = r
        .sessao()
        .upload_ir(1, &blob(CHUNK * 2))
        .expect_err("ACK de outro chunk");
    assert!(
        matches!(e, ProtocolError::UnexpectedAck { .. }),
        "veio {e:?}"
    );
    assert_eq!(r.enviados.len(), 2, "BEGIN + 1 chunk; o resto NÃO foi");
}

/// ACK de outro SLOT é o mesmo erro pelo mesmo motivo: `ir_chunk_ack_payload`
/// carrega o slot, e um ACK de slot diferente é de outra transferência.
#[test]
fn ack_de_outro_slot_aborta_o_upload() {
    let mut r = Roteiro::com(vec![envelope(ir_chunk_ack_payload(9, 0))]);
    let e = r
        .sessao()
        .upload_ir(1, &blob(CHUNK))
        .expect_err("ACK de outro slot");
    assert!(
        matches!(e, ProtocolError::UnexpectedAck { .. }),
        "veio {e:?}"
    );
}

/// Device mudo = `Timeout` com o endereço do ACK (D6), não panic e não
/// `InvalidShape` genérico: a UI distingue "não respondeu" de "respondeu
/// errado", e as duas têm remediação diferente.
#[test]
fn ack_ausente_e_timeout_com_o_endereco() {
    let mut r = Roteiro::com(vec![]);
    let e = r
        .sessao()
        .upload_ir(1, &blob(CHUNK))
        .expect_err("sem ACK");
    assert!(
        matches!(&e, ProtocolError::Timeout { addr, .. } if addr == "12/12001002"),
        "veio {e:?}"
    );
}

/// D7: um push do GP-100 no meio do stream NÃO é o ACK. Ele vai para o
/// backlog e o upload continua — sem isso, o primeiro chunk pararia esperando
/// uma mensagem que não é resposta dele.
#[test]
fn push_intercalado_vai_para_o_backlog_e_nao_e_ack() {
    // 2 chunks (idx 0 e 1) + o marcador, que repete o idx 1.
    let a0 = envelope(ir_chunk_ack_payload(0, 0));
    let a1 = envelope(ir_chunk_ack_payload(0, 1));
    let mut r = Roteiro::com(vec![push_gp100(), a0, a1.clone(), a1]);
    let mut s = r.sessao();
    let rel = s.upload_ir(0, &blob(CHUNK * 2)).expect("push não é ACK");
    assert_eq!(rel.acks, 3, "3 ACKs: 2 chunks + marcador duplicado");
    let backlog = s.pending_pushes().expect("backlog legível");
    assert_eq!(backlog, vec![push_gp100()], "o push foi preservado (D7)");
}

// ───────────────────── a entrega real: o mock remontando o IR ──────────────

/// O `.ir` atravessa o mock e o aparelho o ACEITA: BEGIN + 3 chunks + o
/// marcador, todos com ACK, zero divergência. Este é o teste que dispensa
/// hardware para a maior parte do risco do laboratório.
#[test]
fn o_ir_atravessa_o_mock_com_ack_em_cada_chunk() {
    let b = blob(CHUNK * 3);
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let rel = {
        let mut s = Session::new(&mut dev);
        s.upload_ir(0, &b).expect("upload dos 3 chunks")
    };
    assert_eq!(rel.chunks, 3);
    assert_eq!(rel.acks, 4, "3 + marcador duplicado");
    assert_eq!(rel.bytes, 45);
    assert_eq!(rel.slot, 0);
    assert_eq!(dev.state().rejected, 0, "nenhuma divergência no mock");
}

/// Dois slots seguidos não se misturam: o segundo BEGIN (slot 1) é o que
/// fecha a transferência do primeiro, e o mock recusa frame de chunk sem
/// BEGIN — um upload que esquecesse o BEGIN passaria no teste de um slot só.
#[test]
fn dois_slots_seguidos_nao_se_misturam() {
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    for (slot, n) in [(0u8, 1usize), (3, 2)] {
        let mut s = Session::new(&mut dev);
        let rel = s.upload_ir(slot, &blob(CHUNK * n)).expect("upload");
        assert_eq!(rel.slot, slot);
        assert_eq!(rel.chunks, n);
    }
    assert_eq!(dev.state().rejected, 0);
}

/// A tabela de User IRs do aparelho é o que a tela mostra como "o que já está
/// gravado": 20 slots, e um slot vazio vem com nome VAZIO (não com lixo).
/// A UI depende dessas duas coisas para não listar 20 linhas ocupadas.
#[test]
fn a_tabela_de_irs_tem_vinte_slots_e_nome_vazio_no_que_esta_livre() {
    let mut dev = MockDevice::new().expect("mock");
    dev.open().expect("abre");
    let tabela = {
        let mut s = Session::new(&mut dev);
        s.list_user_irs().expect("20 leituras de tabela")
    };
    assert_eq!(tabela.slots.len(), 20, "§13.12: 20 User IRs");
    assert!(
        tabela.slots.iter().all(|(_, nome)| nome.is_empty()),
        "mock sem IR importado: nenhum slot tem nome {:?}",
        tabela
            .slots
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>()
    );
    // o número do slot é a posição da página (§13.12: [0] = slot ecoado)
    for (esperado, (slot, _)) in tabela.slots.iter().enumerate() {
        assert_eq!(*slot as usize, esperado, "slot {slot} fora de ordem");
    }
}
