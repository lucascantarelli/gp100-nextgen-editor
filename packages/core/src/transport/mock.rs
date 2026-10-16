//! mock — `MockDevice`: o device simulado que responde
//! CONFORME O GOLDEN — toda resposta passa por
//! [`crate::golden::Template::build_response`] e, portanto, casa com o
//! `response_pattern` do próprio template (D5 também nas respostas).
//! Conteúdo: estado derivado de `all.prst` + dicionário onde a var é
//! limpa (pp BE no meta6/páginas, eco de slot+idx no ACK de chunk,
//! seleção de preset pelo write `13010000`); exemplo congelado do golden
//! onde o corpo é evidência de captura (bodies mixed 196B de 13xx,
//! tabela de IRs, nomes `11000008`, estado `12001012`).
//!
//! **Contrato de comportamento = D1–D8 do ADR-6 rev.3:**
//! - **D1** fila IN por endpoint tipado `(func, addr)`; `recv_raw` devolve
//!   a mensagem mais antiga de QUALQUER endpoint (FIFO global — a ordem
//!   real de chegada);
//! - **D3** writes são FIRE-AND-FORGET: metadados `11xx`, `12000002`, ops
//!   `00020000`, `set_param` 10xx0002 e begin/reserva de IR NÃO geram
//!   resposta nem resync (burst de fim de sessão não é emitido — quirk
//!   fechado no §13.7);
//! - **D5** frame que não casa com nenhum `request_pattern` do endpoint =
//!   `SendFailed` com hex curto (nunca engolir);
//! - **D7** `queue_push`/`queue_push_template` injetam pushes não
//!   solicitados a qualquer momento (push intercalado no meio de upload);
//! - **D8** a fila é exclusiva da `Session` (consumidor único).
//!
//! **O mock conhece DOIS framings.** O envelope do GP-100
//! (`F0 21 25 7F 47 50 2D 64 | FUNC | ADDR | payload | F7`) e o da família
//! GP-50 (`F0` + nibble-expand(BUF) + `F7`), que é o do SnapTone/NAM (§5).
//! Os dois começam no mesmo `0xF0` e se discriminam no byte seguinte: `0x21`
//! do GP-100 contra um nibble da família. O caminho de SnapTone tem ACK de
//! 16B por bloco (§5) — é ele que torna o upload do modelo verificável de
//! ponta a ponta sem hardware.
//!
//! **Shape gerado × layout:** as páginas 196B de 13xx são GERADAS (pp BE +
//! corpo do exemplo congelado) — capacidades 196/32B são shape do mock, não
//! layout decifrado (o byte-a-byte da 13xx permanece fora; o replay
//! valida contra as fixtures reais).

mod despacho;
mod estado;
mod falha;
mod familia;
mod fila;
mod respostas;

pub use estado::MockState;
pub use falha::MockFault;

use std::collections::HashMap;
use std::time::Duration;

use super::{DeviceTransport, TransportError, WireKind};
use crate::model::Dictionary;
use crate::ProtocolError;

const ALL_PRST: &str = include_str!("../../../../files/patches/all.prst");

const PARAMETERS: &str = include_str!("../../data/parameters.json");

/// Semente fixa do LCG do jitter — dois devices novos percorrem a MESMA
/// sequência (contrato de determinismo provado em `mod tests`; sem `rand`).
const JITTER_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// O `.prst` embedado (all.prst) — acesso público para as projeções de
/// board (`crate::pedalboard::embedded_document`): os dados vivem no core
/// (R1); consumidores externos nunca reabrem o arquivo de disco.
pub fn embedded_preset() -> &'static str {
    ALL_PRST
}

fn shape_err(expected: &str, got: &str) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.to_string(),
        got: got.to_string(),
    }
}

/// O device simulado (default do ADR-4/ADR-5: sempre permite writes).
#[derive(Debug, Clone)]
pub struct MockDevice {
    opened: bool,
    state: MockState,
    /// Dicionário (carregado 1x na criação) para validar `set_param`.
    dict: Dictionary,
    /// Fila IN por endpoint (D1): mensagens SysEx completas.
    inbox: HashMap<(u8, [u8; 4]), Vec<Vec<u8>>>,
    /// Plano de falha (test double). `None` = device saudável (default).
    fault: Option<MockFault>,
    /// Transmissões TENTADAS até agora (inclusive as que morreram).
    sent: u32,
    /// Latência ida/volta (opt-in por [`MockDevice::with_latency`]): faixa
    /// `[min, max)` do jitter. `None` = device rápido de teste — o DEFAULT,
    /// em que nenhum `sleep` acontece (a suíte inteira depende disto).
    latency: Option<(Duration, Duration)>,
    /// Estado do LCG do jitter (semente fixa — reproduzível entre devices).
    jitter: u64,
    /// Falhas transitórias de USB já emitidas (diagnóstico de teste).
    transient: u32,
    /// Mensagens que SAÍRAM da fila (entregues ou perdidas) — base do
    /// `drop_every`: a n-ésima saída é a que o fio leva.
    popped: u32,
    /// Pps com o slot PRESENTE mas SEM nome escrito (variante de frota,
    /// #161). Ver [`MockDevice::with_slot_sem_nome`].
    slots_sem_nome: Vec<u16>,
}

impl MockDevice {
    /// Cria o mock com estado de `all.prst` + dicionário (embedados).
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se um embedado não parseia.
    pub fn new() -> Result<Self, ProtocolError> {
        Ok(Self {
            opened: false,
            state: MockState::load()?,
            dict: Dictionary::from_json(PARAMETERS)?,
            inbox: HashMap::new(),
            fault: None,
            sent: 0,
            latency: None,
            jitter: JITTER_SEED,
            transient: 0,
            popped: 0,
            slots_sem_nome: Vec::new(),
        })
    }

    /// Variante de INVENTÁRIO (#161): o pp `pedido` tem o slot no
    /// inventário, mas a pg0 vem com o CORPO ZERADO — o device não gravou
    /// nome naquele slot (frota real varia; não é falha de transporte, é
    /// um aparelho saudável com um slot vazio).
    ///
    /// O shape continua EXATO (mesmo template, mesmos tamanhos — o
    /// `decode` aceita nibbles zero); só o conteúdo do nome some, e
    /// `Paginas::nome` recusa "nome vazio". O `BootReport` do scan sai
    /// `198 presets / 197 nomes` — é o cenário em que o gate da #161 tem
    /// de barra a casca NOMEANDO a leitura `nomes`, em vez de aceitar
    /// "198/198" que o aparelho não deu.
    #[must_use]
    pub fn with_slot_sem_nome(mut self, pp: u16) -> Self {
        self.slots_sem_nome.push(pp);
        self
    }

    /// O estado atual (diagnóstico/asserções de teste).
    pub fn state(&self) -> &MockState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O estado carrega de all.prst: 99 presets, pp/nome/tipo do 1º e
    /// ppIRCRC de fábrica nos 20 slots de IR.
    #[test]
    fn estado_deriva_de_all_prst() {
        let st = MockState::load().expect("all.prst embedado parseia (R4)");
        assert_eq!(st.preset_count, 99);
        assert!(!st.current_name.is_empty());
        assert!(
            st.ir_crcs.iter().any(|&c| c != 0),
            "ppIRCRC de fábrica presente em algum slot"
        );
    }

    /// O jitter do `with_latency` é LCG de semente fixa: dOUS devices novos
    /// percorrem a MESMA sequência — determinismo reproduzível em qualquer
    /// máquina (a suíte não pode depender de `rand` nem de sorte de hardware)
    /// — e a sequência varia de verdade dentro da faixa `[min, max)`.
    #[test]
    fn jitter_deterministico_reproduzivel_entre_devices() {
        let min = Duration::from_millis(10);
        let max = Duration::from_millis(40);
        let mut a = MockDevice::new()
            .expect("mock montado")
            .with_latency(min, max);
        let mut b = MockDevice::new()
            .expect("mock montado")
            .with_latency(min, max);

        let seq_a: Vec<Duration> = (0..24).map(|_| a.proximo_jitter()).collect();
        let seq_b: Vec<Duration> = (0..24).map(|_| b.proximo_jitter()).collect();

        assert_eq!(seq_a, seq_b, "mesma semente => mesma sequência");
        assert!(
            seq_a.iter().all(|d| *d >= min && *d < max),
            "faixa [min, max): {seq_a:?}"
        );
        assert!(
            seq_a.iter().any(|d| *d != seq_a[0]),
            "o jitter varia — não é um sleep constante disfarçado"
        );
    }

    /// `MockFault::DieAfter`: o device CAI depois de n transmissões — o
    /// 1º select passa, o 2º já devolve `DeviceGone`, e `recv`/`open`
    /// seguintes confirmam que ele CONTINUA ausente (sem re-plug no mock).
    #[test]
    fn mock_fault_mata_o_device_depois_de_n_transacoes() {
        let mut dev = MockDevice::new()
            .expect("mock montado")
            .with_fault(MockFault::DieAfter(1));
        dev.open().expect("open antes da morte");
        let golden = crate::golden::GoldenFile::embedded().expect("golden embedado");
        let select = golden
            .build_request(0x11, &[0x13, 0x01, 0x00, 0x00], &[0x00, 0x03])
            .expect("select §13.10 montado pelo golden");

        // transação 1: device vivo (o select entra e o pp muda)
        dev.send_raw(&select, WireKind::Read)
            .expect("1º send com o device vivo");
        assert_eq!(dev.transactions(), 1);
        assert_eq!(dev.state().current_pp, 3);

        // transação 2: o device SUMIU no meio da sessão
        let err = dev
            .send_raw(&select, WireKind::Read)
            .expect_err("2º send: device caiu");
        assert!(
            matches!(err, TransportError::DeviceGone { .. }),
            "esperado DeviceGone, veio {err:?}"
        );
        assert!(
            err.to_string().contains("device sumiu no meio da sessão"),
            "mensagem humana do DeviceGone: {err}"
        );
        // já ausente: recv não acha mais nada E open não ressuscita
        assert!(matches!(
            dev.recv_raw(Duration::from_millis(1)),
            Err(TransportError::DeviceGone { .. })
        ));
        assert!(matches!(dev.open(), Err(TransportError::DeviceGone { .. })));
    }

    /// `DieAfter(0)`: o device já saiu ANTES da 1ª transação — a morte é
    /// reportada sem nem olhar o frame (não há ninguém no fio para recebê-lo).
    #[test]
    fn mock_fault_zero_reporta_antes_do_parse() {
        let mut dev = MockDevice::new()
            .expect("mock montado")
            .with_fault(MockFault::DieAfter(0));
        dev.open().expect("open com o device ainda presente");
        let err = dev
            .send_raw(b"nao-e-sysex", WireKind::Write)
            .expect_err("morre antes do parse");
        assert!(
            matches!(err, TransportError::DeviceGone { .. }),
            "DeviceGone precede o shape: {err:?}"
        );
        assert_eq!(dev.transactions(), 1);
    }

    /// §13.10: o byte `d[3]` da página é o NÚMERO DA PÁGINA, e a resposta ao
    /// request da PG é a página `PG + 1` ("req pg0..7 → pág1..8").
    ///
    /// Este teste existe porque o ecoar `PG` passava despercebido: o
    /// `StatePage` é opaco (ninguém lê esse byte), então o mock respondia
    /// 0 nas 8 páginas e nada quebrava — só a referência do H1 saía de
    /// sincronia, e o estrago só apareceria em campo, como 8 "divergências
    /// de estado" falsas por dump.
    #[test]
    fn pagina_responde_o_numero_da_pagina_mais_um() {
        for (pedida, esperada) in [(0u8, 1u8), (1, 2), (7, 8)] {
            let mut dev = MockDevice::new().expect("mock montado");
            dev.open().expect("open");
            let mut sess = crate::session::Session::new(&mut dev);
            sess.select_preset(0).expect("select §13.10");
            let pagina = sess.state_page(pedida).expect("página");
            assert_eq!(
                pagina.raw[3], esperada,
                "req PG={pedida} deve responder a página {esperada} (§13.10), \
                 veio {}",
                pagina.raw[3]
            );
        }
    }

    /// A PG 8 não é página: é o header de 4B em `13010005` (§13.10). E o
    /// byte existe — se o `+1` do teste anterior vazasse para cá, este
    /// acusaria.
    #[test]
    fn pagina_8_e_o_header_de_4_bytes() {
        let mut dev = MockDevice::new().expect("mock montado");
        dev.open().expect("open");
        let mut sess = crate::session::Session::new(&mut dev);
        sess.select_preset(0).expect("select §13.10");
        let pagina = sess.state_page(8).expect("PG 8");
        assert_eq!(pagina.raw.len(), 4, "PG 8 responde 4B (§13.10)");
    }

    /// O dicionário embedado carrega (185 algs) e reconhece um effectCode
    /// real (Bog RedM, vetor da fixture knobs: 0x0700006e).
    #[test]
    fn dicionario_reconhece_effectcode_real() {
        let dict = Dictionary::from_json(PARAMETERS).expect("parameters.json embedado");
        assert_eq!(dict.len(), 185);
        let nib = 0x07u8;
        let idx = 0x0700_006eu32 & 0x00FF_FFFF;
        assert!(
            dict.algorithm(nib, idx).is_some(),
            "Bog RedM (nibble 7) deve estar no dicionário"
        );
    }
}
