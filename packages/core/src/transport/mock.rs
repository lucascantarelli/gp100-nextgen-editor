//! mock — `MockDevice`: o device simulado que responde
//! CONFORME O GOLDEN — toda resposta passa por
//! [`crate::golden::Template::build_response`] e, portanto, casa com o
//! `response_pattern` do próprio template (D5 também nas respostas).
//! Conteúdo: estado derivado de `all.prst` + dicionário onde a var é
//! limpa (pp BE no meta6/páginas, eco de slot+idx no ACK de chunk,
//! seleção de preset pelo write `13010000`); exemplo congelado do golden
//! onde o corpo é evidência de captura (bodies mixed 196B de 13xx,
//! tabela de IRs, nomes `11000008`, setlist `12001012`).
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

use std::collections::HashMap;
use std::time::Duration;

use super::{DeviceTransport, TransportError, WireKind};
use crate::golden::decode_envelope;
use crate::golden::GoldenFile;
use crate::model::Dictionary;
use crate::preset::Document;
use crate::{ProtocolError, SYSEX_EOX, SYSEX_HEADER};

const ALL_PRST: &str = include_str!("../../../../files/patches/all.prst");
const PARAMETERS: &str = include_str!("../../../../analysis/parameters.json");

/// Semente fixa do LCG do jitter — dois devices novos percorrem a MESMA
/// sequência (contrato de determinismo provado em `mod tests`; sem `rand`).
const JITTER_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// O `.prst` embedado (all.prst) — acesso público para as projeções de
/// board (`crate::pedalboard::embedded_document`): os dados vivem no core
/// (R1); consumidores externos nunca reabrem o arquivo de disco.
pub fn embedded_preset() -> &'static str {
    ALL_PRST
}

/// Estado do device simulado, derivado de `all.prst` + dicionário.
#[derive(Debug, Clone)]
pub struct MockState {
    /// Nº de presets carregados do `.prst` (99 em `all.prst`).
    pub preset_count: usize,
    /// pp corrente (seleção via write `13010000`; default = 1º do `.prst`).
    pub current_pp: u16,
    /// Nome do pp corrente (do `.prst`; trocado por write `11000000`).
    pub current_name: String,
    /// ppType do pp corrente (do `.prst`; trocado por write `11000005`).
    pub current_pp_type: u16,
    /// CRC32 IEEE de fábrica dos 20 slots de IR (ppIRCRC do `.prst`).
    pub ir_crcs: [u32; 20],
    /// Parâmetros setados: `((nibble, ctrl), (code, value))` — last-wins.
    pub set_params: HashMap<(u8, u8), (u32, f32)>,
    /// Modelo de SnapTone **remontado** dos blocos recebidos (§5). É o que o
    /// mock pode observar do upload: o fio carrega o modelo em blocos com
    /// índice, e não há seletor de slot no stream (R1 — só a contagem e o
    /// payload estão evidenciados), então o que fica é a transferência
    /// corrente, reaberta quando um índice 0 chega.
    pub snap_tone_model: Vec<u8>,
    /// Quantas transferências de SnapTone chegaram (um índice 0 = uma nova).
    pub snap_tone_transfers: usize,
    /// ACKs de SnapTone emitidos — um por bloco aceito (§5).
    pub snap_tone_acks: usize,
    /// nº de frames recusados por D5 (diagnóstico de divergência).
    pub rejected: usize,
}

impl MockState {
    /// Carrega o estado de `all.prst` (embedado). O pp corrente inicial é o
    /// do 1º preset (`ppID` decimal = índice 0-based, o mesmo espaço do
    /// fio); os ppIRCRC de fábrica povoam os 20 slots.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se o `.prst` embedado não parseia ou
    /// nenhum preset tem ppID (impossível no build normal: R4 é travado).
    pub fn load() -> Result<Self, ProtocolError> {
        let doc = Document::parse(ALL_PRST.as_bytes())?;
        let mut ir_crcs = [0u32; 20]; // Os 20 slots de IR são as tags <ppIRInfo0..19> no container
                                      // <ppIRInfo> — irmão de <preset_info> NO NÍVEL DA RAIZ <GP>
                                      // (achado estrutural; o ppIRInfo NÃO está dentro de preset_info nem de
                                      // um preset). CRC: i32 decimal → u32 (ppIRCRC="-1871114785").
        if let Some(info) = doc.root().child("ppIRInfo") {
            for (i, tag) in info.children().iter().enumerate().take(20) {
                if let Some(crc) = tag.attr("ppIRCRC") {
                    if let Ok(c) = crc.parse::<i32>() {
                        ir_crcs[i] = c as u32;
                    }
                }
            }
        }
        let first = doc
            .presets()
            .next()
            .ok_or_else(|| shape_err("preset no all.prst", "nenhum"))?;
        let current_pp = first
            .pp_id()
            .and_then(crate::preset::pp_id_decimal)
            .ok_or_else(|| shape_err("ppID decimal no 1º preset", "ausente"))?;
        Ok(Self {
            preset_count: doc.presets().count(),
            current_pp,
            current_name: first.pp_name().unwrap_or("").to_string(),
            current_pp_type: first.pp_type().and_then(|s| s.parse().ok()).unwrap_or(4),
            ir_crcs,
            set_params: HashMap::new(),
            snap_tone_model: Vec::new(),
            snap_tone_transfers: 0,
            snap_tone_acks: 0,
            rejected: 0,
        })
    }
}

fn shape_err(expected: &str, got: &str) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.to_string(),
        got: got.to_string(),
    }
}

/// Plano de FALHA do mock (test double): o device pode CAIR no meio da
/// sessão — o cenário para o qual [`TransportError::DeviceGone`] existe.
///
/// O mock é o test double declarado do projeto; a falha entra AQUI (e não num
/// `FailingTransport` novo) para o caminho testado ser EXATAMENTE o de
/// produção: transporte → FSM → actor/command → UI. NUNCA afeta o
/// `RealDevice`: a política de hardware (ADR-4/ADR-5) segue intocada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFault {
    /// Depois de `n` transmissões (`send_raw`) o device desaparece: todo
    /// `send_raw`/`recv_raw`/`open` seguinte devolve
    /// [`TransportError::DeviceGone`]. `0` = já saiu antes da 1ª transação.
    DieAfter(u32),

    /// USB **instável** (intermitente): falhas que NÃO matam o device — o
    /// outro lado do moeda do `DieAfter`. Tudo contra-contado, sem
    /// aleatoriedade (mesma determinismo do golden).
    ///
    /// - `send_every`: a cada n-ésima `send_raw` o envio falha UMA vez com
    ///   [`TransportError::SendFailed`] transitório — a tentativa seguinte
    ///   passa (o contador saiu do múltiplo) e o device segue ABERTO. Como
    ///   a falha precede o parse, o device "não recebeu os bytes": não há
    ///   resposta a enfileirar. `0` desliga.
    /// - `drop_every`: a cada n-ésima mensagem que sairia no `recv_raw` o
    ///   mock a **perde no fio** — consome da fila e devolve
    ///   [`TransportError::RecvTimeout`] com o device saudável (a mensagem
    ///   não reaparece; a seguinte chega). `0` desliga.
    UsbFlaky {
        /// Período do envio: a cada n-ésima `send_raw` falha UMA vez
        /// (múltiplos de n); `0` desliga.
        send_every: u32,
        /// Período da resposta: a cada n-ésima mensagem que sairia no
        /// `recv_raw` o mock a perde no fio; `0` desliga.
        drop_every: u32,
    },
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

    /// Arma um plano de falha: o device CAI depois de `n` transmissões.
    ///
    /// Consumidor típico: testes do shell (actor) e o smoke Tauri, que arma
    /// por env (`GP100_DEBUG_FAULT=die-after:<n>`) — sempre no backend MOCK.
    #[must_use]
    pub fn with_fault(mut self, fault: MockFault) -> Self {
        self.fault = Some(fault);
        self
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

    /// Transmissões tentadas até agora (diagnóstico/asserções de teste).
    pub fn transactions(&self) -> u32 {
        self.sent
    }

    /// Falhas transitórias de USB emitidas até aqui ([`MockFault::UsbFlaky`]).
    pub fn transient_failures(&self) -> u32 {
        self.transient
    }

    /// Ativa a latência ida/volta com jitter na faixa `[min, max)` —
    /// **opt-in**: fora daqui o mock continua instantâneo (default da suíte).
    ///
    /// - `send_raw` dorme o jitter ANTES de entregar os bytes (escrita +
    ///   processamento no hardware); `Closed`/`DeviceGone` continuam
    ///   precedendo;
    /// - `recv_raw` com mensagem enfileirada dorme o jitter antes do pop
    ///   (chegada ao host);
    /// - `recv_raw` VAZIO espelha o `real.rs`: espera a JANELA inteira do
    ///   `timeout` e só então acusa o silêncio — é o falso positivo de tempo
    ///   (timeout instantâneo com janela de 3s) que este opt-in fecha.
    ///
    /// O jitter vem de um LCG semeado: determinístico, sem dep de `rand`.
    /// Granularidade de milissegundo; `max <= min` devolve sempre `min`.
    #[must_use]
    pub fn with_latency(mut self, min: Duration, max: Duration) -> Self {
        self.latency = Some((min, max));
        self
    }

    /// Próximo valor do jitter (LCG de 64b, constantes MMIX): mesma semente,
    /// mesma sequência, em qualquer máquina.
    fn proximo_jitter(&mut self) -> Duration {
        self.jitter = self
            .jitter
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let (min, max) = self.latency.unwrap_or((Duration::ZERO, Duration::ZERO));
        let span = max.as_millis().saturating_sub(min.as_millis()) as u64;
        if span == 0 {
            return min;
        }
        let pick = (self.jitter >> 33) % span;
        Duration::from_millis(min.as_millis() as u64 + pick)
    }

    /// O device já caiu? (o plano `DieAfter` foi consumido)
    fn gone(&self) -> bool {
        matches!(self.fault, Some(MockFault::DieAfter(n)) if self.sent > n)
    }

    /// Erro de device ausente (fio cortado), com o motivo do plano de falha.
    fn gone_err(&self) -> TransportError {
        TransportError::DeviceGone {
            why: format!(
                "mock: device caiu depois de {} transmissões (MockFault::DieAfter)",
                self.sent
            ),
        }
    }

    /// O estado atual (diagnóstico/asserções de teste).
    pub fn state(&self) -> &MockState {
        &self.state
    }

    /// Drena a inbox de respostas/pushes NÃO consumidos (ordem FIFO global
    /// — a mais antiga de qualquer endpoint primeiro, mesma regra do
    /// `recv_raw`): observação de pushes pendentes pós-operação (D7 na
    /// perspectiva do DEVICE; o dono do `Session` enxerga o seu backlog
    /// via `pending_pushes`). Usado pelo DeviceActor para reemitir
    /// pushes como eventos da UI.
    pub fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            let oldest = self
                .inbox
                .iter()
                .filter(|(_, q)| !q.is_empty())
                .map(|(k, _)| *k)
                .min();
            match oldest {
                Some(k) => {
                    if let Some(q) = self.inbox.get_mut(&k) {
                        if !q.is_empty() {
                            out.push(q.remove(0));
                        }
                    }
                }
                None => break,
            }
        }
        out
    }

    /// Enfileira um push não solicitado com payload VERBATIM (D7) — o
    /// caminho do teste de push intercalado (a FSM filtra via backlog).
    pub fn queue_push(&mut self, func: u8, addr: [u8; 4], payload: &[u8]) {
        let msg = envelope_of(func, addr, payload);
        self.inbox.entry((func, addr)).or_default().push(msg);
    }

    /// Enfileira um push PADRÃO, construído pelo golden (exemplo congelado
    /// do template `push` do endpoint). `desired_len` escolhe o sub-padrão
    /// em respostas `by-len` (ex.: 75 para a tabela de IRs em `12001002`).
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se não há template push no endpoint
    /// ou a resposta não é buildável (D5: o mock nunca inventa forma).
    pub fn queue_push_template(
        &mut self,
        func: u8,
        addr: [u8; 4],
        desired_len: Option<usize>,
    ) -> Result<(), ProtocolError> {
        let golden = GoldenFile::embedded()?;
        let tpl = push_template(golden, addr)?;
        let msg = build_from_example(tpl, desired_len, None)?;
        self.inbox.entry((func, addr)).or_default().push(msg);
        Ok(())
    }

    /// Enfileira uma mensagem CRUA — fora do envelope do GP-100 (é o que o
    /// transporte devolve para o framing da família, §5).
    ///
    /// A chave da fila é `(0x00, 00000000)`, a MENOR possível: o `recv_raw`
    /// entrega a mensagem mais antiga pela chave mínima e o ACK do SnapTone
    /// tem que sair antes de qualquer push da família GP-100 que esteja
    /// pendente — a resposta é da operação corrente, não do histórico.
    pub fn queue_raw(&mut self, msg: Vec<u8>) {
        self.inbox.entry((0x00, [0u8; 4])).or_default().push(msg);
    }

    /// O modelo de SnapTone remontado e quantas transferências chegaram.
    ///
    /// O par vem junto porque os dois contam a MESMA coisa vista de dois
    /// lados: uma transferência com N bytes pode ser 1 upload ou vários
    /// índices 0 seguidos, e só os dois juntos distinguem.
    pub fn snap_tone(&self) -> (&[u8], usize) {
        (&self.state.snap_tone_model, self.state.snap_tone_transfers)
    }

    /// Despacha um frame do framing da FAMÍLIA (`F0` + nibbles + `F7`): o
    /// SnapTone/NAM (§5), que NÃO usa o envelope do GP-100.
    ///
    /// O que o mock valida (e recusa com D5, incrementando `rejected`):
    /// o CRC-8 do BUF (§3), o `length` contra o payload realmente recebido, e
    /// o `command`. O que ele **remonta** é o modelo, para o upload ser
    /// verificável de ponta a ponta sem hardware — o teste pede 2 700 bytes,
    /// o mock devolve os mesmos 2 700.
    ///
    /// O conteúdo do ACK é **zeros**: o ACK tem 16 bytes (§5), mas o que
    /// esses 16 bytes dizem não está evidenciado, e o mock não inventa
    /// semântica (D5). A FSM valida o tamanho — que é o que a evidência
    /// sustenta.
    fn ingest_familia(&mut self, msg: &[u8]) -> Result<(), TransportError> {
        let b = crate::codec::nibble_collapse(&msg[1..msg.len() - 1])
            .map_err(|e| TransportError::SendFailed { why: e.to_string() })?;
        // BUF mínimo = [crc, command, index, length] (§2).
        if b.len() < 4 {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!("BUF de familia com {} bytes (minimo 4, §2)", b.len()),
            });
        }
        // CRC (§3): o byte do CRC entra ZERADO no cálculo — comparar o valor
        // gravado contra o calculado sobre o BUF com o byte zerado.
        let mut com_crc_zero = b.clone();
        com_crc_zero[0] = 0;
        if crate::snap_tone::crc8(&com_crc_zero) != b[0] {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!(
                    "CRC do bloco: esperado {:02X}, recebeu {:02X}",
                    crate::snap_tone::crc8(&com_crc_zero),
                    b[0]
                ),
            });
        }
        let (cmd, index, len) = (b[1], b[2], usize::from(b[3]));
        let payload = &b[4..];
        if cmd != crate::snap_tone::CMD_SNAP_TONE {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!("command da familia {cmd:02X} != SnapTone 92"),
            });
        }
        if payload.len() != len {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!("length {len} != payload de {} bytes", payload.len()),
            });
        }
        // Índice 0 = transferência nova. O fio NÃO carrega o slot (R1: só o
        // layout do bloco está evidenciado, e nele não há slot), então o
        // mock reabre o stream no 0 — que é o único sinal de começo que a
        // evidência sustenta.
        if index == 0 {
            self.state.snap_tone_model.clear();
            self.state.snap_tone_transfers += 1;
        }
        self.state.snap_tone_model.extend_from_slice(payload);
        self.state.snap_tone_acks += 1;
        let ack = {
            let mut m = vec![0xF0];
            m.extend(crate::codec::nibble_expand(
                &[0u8; crate::snap_tone::ACK_LEN],
            ));
            m.push(crate::SYSEX_EOX);
            m
        };
        self.queue_raw(ack);
        Ok(())
    }

    /// Processa UM frame de entrada (já decodificado): efeito de estado +
    /// resposta a enfileirar, se houver. É o coração do despacho (D1/D3/D5).
    fn ingest(&mut self, func: u8, addr: [u8; 4], payload: &[u8]) -> Result<(), TransportError> {
        // ADR-2/rust-practices: sem panic na lib — golden indisponível vira
        // erro tipado na MESMA conversão do resto do despacho (D5).
        let golden = match GoldenFile::embedded() {
            Ok(g) => g,
            Err(e) => {
                self.state.rejected += 1;
                return Err(TransportError::SendFailed { why: e.to_string() });
            }
        };

        // §13.11 set_param: o golden congela os templates POR INSTÂNCIA
        // (9 templates 10xx0002 com consts de knobs específicos), então o
        // SHAPE é validado pelo CODEC (prova: 92 knobs byte a byte em
        // tests/codec_wire.rs). R1: shape ≠ semântica — o codec é a fonte.
        if func == 0x12 && addr[0] == 0x10 && addr[2] == 0x00 && addr[3] == 0x02 {
            return match crate::codec::set_param_parse(payload) {
                Ok((code, ctrl, value)) => {
                    let nib = (code >> 24) as u8;
                    let idx = code & 0x00FF_FFFF;
                    let _known = self.dict.algorithm(nib, idx).is_some();
                    self.state.set_params.insert((nib, ctrl), (code, value));
                    Ok(()) // D4: sem resposta
                }
                Err(e) => {
                    self.state.rejected += 1;
                    Err(TransportError::SendFailed { why: e.to_string() })
                }
            };
        }
        // D5: o frame DEVE casar com um request_pattern do endpoint.
        let Some(tpl_idx) = golden.match_request(func, &addr, payload) else {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!(
                    "request sem template no golden (D5): func={func:02x} addr={addr:02x?} payload={:.24}",
                    payload.iter().map(|b| format!("{b:02x}")).collect::<String>()
                ),
            });
        };

        // Efeitos de estado dos WRITES (D3: nenhum gera resposta).
        match (func, addr) {
            // seleção de preset (§13.10): payload = [pp BE]
            (0x11, [0x13, 0x01, 0x00, 0x00]) if payload.len() == 2 => {
                self.state.current_pp = u16::from_be_bytes([payload[0], payload[1]]);
            }
            // metadados do save (§13.12): nome (ASCII 12B trunc+pad)
            (0x12, [0x11, 0x00, 0x00, 0x00]) if payload.len() == 20 => {
                let name: String = payload[8..20]
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect();
                if !name.is_empty() {
                    self.state.current_name = name;
                }
            }
            // ppType u16 BE + zeros2
            (0x12, [0x11, 0x00, 0x05, 0x00]) if payload.len() == 4 => {
                self.state.current_pp_type = u16::from_be_bytes([payload[0], payload[1]]);
            }
            // (set_param tratado ANTES do despacho — shape é do codec)
            _ => {}
        }

        // Respostas (D1): o SELECT de preset (write 11/13010000) responde
        // com o meta6 push em `13010001` levando o pp SELECIONADO —
        // pareamento 199↔199 provado no replay S1 (e na sonda 1302); READS
        // func 11 respondem com o push do MESMO endereço (func 12 no fio);
        // transações req 12-func com lado IN respondem pelo template casado.
        // Sonda do banco 02 (T6 do boot §13.10): select CONST `0000` (t14;
        // sem pp) → o meta6 PUSH da sonda (t12 @ `13020001`, const
        // `00000c1c0140` — o device não ecoa pp porque o select não tem pp).
        let reply = if func == 0x11 && addr == [0x13, 0x02, 0x00, 0x00] && payload == [0x00, 0x00] {
            push_template(golden, [0x13, 0x02, 0x00, 0x01])
                .ok()
                .and_then(|t| build_from_example(t, None, None).ok())
        } else if func == 0x11 && addr == [0x13, 0x01, 0x00, 0x00] && payload.len() == 2 {
            let pp = payload[..2].to_vec();
            push_template(golden, [0x13, 0x01, 0x00, 0x01])
                .ok()
                .and_then(|t| {
                    build_with_fill(t, None, |i, count| {
                        if i == 0 && count == 2 {
                            pp.clone()
                        } else {
                            example_var(t, None, i, count)
                        }
                    })
                    .ok()
                })
        } else if func == 0x11 {
            let tpl = push_template(golden, addr).ok();
            let desired = if addr == [0x12, 0x00, 0x10, 0x02] {
                Some(75)
            } else {
                None
            }; // Tabela de IRs (t2/t3): o IN ecoa a PÁGINA pedida no byte[0]
               // (captura S1 rows 78–84: IN `13 0f…` para a leitura `[13]`).
               // Vale para o var inteiro (ACK 1B ou tabela 75B): byte 0 = página.
            let page = payload.first().copied();
            let is_table = addr == [0x12, 0x00, 0x10, 0x02];
            tpl.and_then(|t| {
                build_with_fill(t, desired, |i, count| {
                    if is_table && i == 0 {
                        let mut v = example_var(t, None, 0, count);
                        if let (Some(p), Some(b0)) = (page, v.first_mut()) {
                            *b0 = p;
                        }
                        v
                    } else {
                        example_var(t, None, i, count)
                    }
                })
                .ok()
            })
        } else {
            match (func, addr) {
                // ACK de chunk de IR (§13.7): eco [slot][idx BE] + 01
                (0x12, [0x12, 0x00, 0x10, 0x02]) => {
                    let tpl = &golden.templates()[tpl_idx];
                    let echo = payload[..3].to_vec();
                    build_with_fill(tpl, Some(4), |i, count| {
                        if i == 0 {
                            let mut v = echo.clone();
                            v.resize(count, 0);
                            v
                        } else {
                            example_var(tpl, None, i - 1, count)
                        }
                    })
                    .ok()
                }
                // meta6 (t6, const) / abertura (t7/t13) / 13010005 (t11/
                // t16) / páginas (t8/t15, by-len).
                //
                // Regra do arm: o shape vem SEMPRE do template (mock não
                // inventa forma, D5) e o conteúdo por EVIDÊNCIA:
                //   abertura → o device responde com a PÁGINA 0 (196B) no
                //     endpoint de página — ordem da captura S1 rows 89–93
                //     (open open → pág0 pág0); o golden t7 pareou a abertura
                //     com o meta6 do select (mis-pairing inofensivo até
                //     aqui: o boot só tinha rodado no transporte de replay);
                //   páginas → by-len 196/32 pelo in_addr do template, com
                //     eco do pp do REQUEST e o número da página RESPONDIDA
                //     (= PG+1, §13.10 — evidência S1 rows 93–107);
                //   PG 8 → 4B em `1301/1302 0005` (t11/t16, pp no var0).
                (0x12, [0x13, ..]) => {
                    let tpl = &golden.templates()[tpl_idx];
                    // by-len decide-se pelo ENDEREÇO DA RESPOSTA (IN do
                    // template), não pelo frame recebido: t8 lê em
                    // 13010004 e responde em 13010003 (196B/32B).
                    let in_addr = tpl
                        .in_
                        .as_ref()
                        .and_then(|e| e.parsed().ok())
                        .map(|(_, a)| a);
                    let desired = match in_addr {
                        Some([0x13, 0x01, 0x00, 0x03]) | Some([0x13, 0x02, 0x00, 0x03]) => {
                            Some(196)
                        }
                        _ => None,
                    };
                    // Eco do pp VEM DO REQUEST (payload[0..2]): o scan abre/
                    // pagina o pp pedido — o state.current_pp ainda é o
                    // ANTERIOR durante o scan. Fallback = corrente.
                    let echo_pp = payload
                        .get(0..2)
                        .map(|s| s.to_vec())
                        .unwrap_or_else(|| self.state.current_pp.to_be_bytes().to_vec());
                    let requested_pg = payload.get(3).copied();
                    // A página 8 — resposta ao req PG 7 — mede 32B na
                    // captura real (4B header + 28B de nibbles = 14
                    // decodificados); as outras 8 medem 196B (4B + 192B).
                    // O golden aceita os dois (`lens: [32, 196]` em
                    // `13010003`), então servir 196B em tudo passava
                    // silenciosamente: o shape batia por forma e só o
                    // `decode` das páginas (§13.10) acusava, como 9 páginas
                    // de 196B onde o aparelho manda 8+1.
                    let desired = match requested_pg {
                        Some(7) => Some(32),
                        _ => desired,
                    };
                    let is_page =
                        matches!(addr, [0x13, 0x01, 0x00, 0x04] | [0x13, 0x02, 0x00, 0x04]);
                    // PG 8 (§13.10): a resposta NÃO é página 196B — é 4B em
                    // `1301/1302 0005` (t11/t16, pp no var0; evidência S1
                    // rows 107–109). A by-len não tem sub-padrão 4B.
                    let pg8_in: [u8; 4] = if addr[1] == 0x02 {
                        [0x13, 0x02, 0x00, 0x05]
                    } else {
                        [0x13, 0x01, 0x00, 0x05]
                    };
                    let pg8 = if is_page && payload.get(3) == Some(&0x08) {
                        golden
                            .templates()
                            .iter()
                            .find(|t| {
                                t.template_type == "req"
                                    && t.in_.as_ref().is_some_and(|e| {
                                        e.parsed().map(|(_, a)| a).is_ok_and(|a| a == pg8_in)
                                    })
                            })
                            .and_then(|t11| {
                                build_with_fill(t11, None, |i, count| {
                                    if i == 0 && count == 2 {
                                        echo_pp.clone()
                                    } else {
                                        example_var(t11, None, i, count)
                                    }
                                })
                                .ok()
                            })
                    } else {
                        None
                    };
                    if let Some(msg) = pg8 {
                        Some(msg)
                    } else {
                        let is_open =
                            matches!(addr, [0x13, 0x01, 0x00, 0x02] | [0x13, 0x02, 0x00, 0x02])
                                && payload.len() == 3;
                        if is_open {
                            let want_in: [u8; 4] = if addr[1] == 0x02 {
                                [0x13, 0x02, 0x00, 0x03]
                            } else {
                                [0x13, 0x01, 0x00, 0x03]
                            };
                            // Variante #161: banco 01 (o scan que enche o
                            // cache `self.pages`) com o pp pedido na lista
                            // de slots sem nome → corpo da pg0 zerado. A
                            // sonda 02 não vai ao cache, fica de fora.
                            let sem_nome = addr[1] == 0x01
                                && self
                                    .slots_sem_nome
                                    .iter()
                                    .any(|&pp| echo_pp == pp.to_be_bytes());
                            let tpl_page = golden
                                .templates()
                                .iter()
                                .find(|t| {
                                    t.template_type == "req"
                                        && t.in_.as_ref().is_some_and(|e| {
                                            e.parsed().map(|(_, a)| a).is_ok_and(|a| a == want_in)
                                        })
                                })
                                .ok_or(TransportError::SendFailed {
                                    why: "template da página (t8/t15) ausente no golden".into(),
                                })?;
                            let page = build_with_fill(tpl_page, Some(196), |i, count| {
                                if i == 0 && count == 2 {
                                    echo_pp.clone()
                                } else if i == 1 && count == 1 {
                                    vec![0] // PG 0 da abertura
                                } else if sem_nome {
                                    // Slot sem nome (#161): o corpo vem
                                    // zerado — mesmos bytes de shape, zero
                                    // de conteúdo. `decode` aceita, `nome`
                                    // recusa "nome vazio".
                                    vec![0; count]
                                } else {
                                    // O exemplo congelado do golden É o
                                    // conteúdo real — é o mesmo caminho dos
                                    // reqs de página. O corpo do `open` é a
                                    // pg0, onde mora o NOME do preset (#155);
                                    // zeros aqui deixavam `preset_pages::nome`
                                    // sem o que ler e derrubavam o palco.
                                    example_var(tpl_page, Some(196), i, count)
                                }
                            })
                            .map_err(|e: ProtocolError| {
                                TransportError::SendFailed { why: e.to_string() }
                            })?;
                            Some(page)
                        } else {
                            // Páginas (t8/t15, by-len): eco do request onde
                            // evidente e exemplo da resposta para o resto —
                            // shape sempre correto (by-len casa por forma).
                            //
                            // O layout do segmento `by_len["196"]` é
                            // `[var 2B = pp][const 1B = 00][var 18B = corpo]…`,
                            // e o NÚMERO DA PÁGINA é o byte 0 desse var de 18B
                            // — é o `d[3]` do §13.10, que substituiu o
                            // "contador de versão" do §13.3.
                            //
                            // O número é `PG + 1`, NÃO `PG`: o §13.10 diz
                            // "req pg0..7 → pág1..8" (e a PG 8 responde o
                            // header de 4B com d[3]=9). Ecoar o PG do request
                            // fazia TODAS as 8 páginas responderem 0.
                            //
                            // O estrago disso era invisível e caro: o
                            // `StatePage` é opaco (ninguém lia esse byte), o
                            // dump prints OK, e a referência de
                            // `analysis/h1_reference/` — que é a SAÍDA do
                            // mock — ficava com drift. Como o H1 compara o
                            // device real contra essa referência, cada
                            // sessão de campo acusaria 8 "divergências de
                            // estado" por dump que não são de estado — e
                            // elas seriam coladas no H1_REPORT como afirmações
                            // sobre a pedaleira.
                            build_with_fill(tpl, desired, |i, count| {
                                if i == 0 && count == 2 {
                                    echo_pp.clone()
                                } else if is_page && i == 1 && count >= 1 {
                                    let mut corpo = example_var(tpl, desired, i, count);
                                    // o exemplo congelado é a evidência do
                                    // corpo; só d[3] (offset 3 do payload) é
                                    // reescrito com a página desta resposta.
                                    corpo[0] = requested_pg.unwrap_or(0).wrapping_add(1);
                                    corpo
                                } else {
                                    example_var(tpl, desired, i, count)
                                }
                            })
                            .map(Some)
                            .map_err(|e: ProtocolError| {
                                TransportError::SendFailed { why: e.to_string() }
                            })?
                        }
                    }
                }
                _ => None,
            }
        };

        if let Some(msg) = reply {
            let in_key = decode_envelope(&msg)
                .map(|(f, a, _)| (f, a))
                .unwrap_or((func, addr));
            self.inbox.entry(in_key).or_default().push(msg);
        }
        Ok(())
    }
}

impl DeviceTransport for MockDevice {
    fn open(&mut self) -> Result<(), TransportError> {
        // Device que CAIU não ressuscita no mock (o re-plug é sessão nova):
        // reabrir o mesmo objeto devolve `DeviceGone`, não `Closed`.
        if self.gone() {
            return Err(self.gone_err());
        }
        self.opened = true;
        Ok(())
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.opened = false;
        Ok(())
    }

    /// O mock **sempre** aceita escrita (ADR-5): a trava é do transporte
    /// REAL, e o mock é o test double dos replays de knob/save/IR — uma
    /// trava aqui apagaria justamente os ensumos que provam o codec.
    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        // A MORTE precede o parse: um device que já não está no fio não
        // recebe bytes, então o plano de falha é checado ANTES do envelope
        // (o shape do frame não importa para quem sumiu).
        self.sent += 1;
        if self.gone() {
            return Err(self.gone_err());
        }
        // O USB instável precede o parse pelo MESMO motivo: o SO recusou o
        // buffer e o device nunca viu os bytes — SendFailed TRANSITÓRIO,
        // device vivo, e a tentativa seguinte passa (contador saiu do
        // múltiplo). Nenhuma resposta nasce de um envio que não chegou.
        if let Some(MockFault::UsbFlaky { send_every, .. }) = self.fault {
            if send_every > 0 && self.sent.is_multiple_of(send_every) {
                self.transient += 1;
                return Err(TransportError::SendFailed {
                    why: format!(
                        "mock: USB instável — o envio {0} (múltiplo de {send_every}) \
                         falhou de forma transitória (MockFault::UsbFlaky)",
                        self.sent
                    ),
                });
            }
        }
        // Latência de IDA (opt-in): os bytes chegam ao device depois do
        // jitter. Sem `with_latency` não existe um único sleep neste mock.
        if self.latency.is_some() {
            let jitter = self.proximo_jitter();
            std::thread::sleep(jitter);
        }
        // O framing da FAMÍLIA (SnapTone/NAM, §5) vem antes do decode do
        // envelope: `F0` + nibbles + `F7` não tem o cabeçalho de 8B do
        // GP-100, então sem esta volta um bloco de SnapTone viraria
        // SendFailed "cabeçalho inválido" e o upload do §5 — o único
        // caminho de escrita com ACK por bloco que dá para testar sem
        // hardware — ficaria impossível de exercitar.
        if crate::snap_tone::eh_familia(data) {
            return self.ingest_familia(data);
        }
        let (func, addr, payload) =
            decode_envelope(data).map_err(|e| TransportError::SendFailed { why: e.to_string() })?;
        self.ingest(func, addr, payload)
    }

    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        if self.gone() {
            return Err(self.gone_err());
        }
        // D1: FIFO global — a mensagem mais antiga de qualquer endpoint.
        let oldest = self
            .inbox
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, _)| *k)
            .min();
        if let Some(k) = oldest {
            // sai da fila ANTES dos sleeps: o `&mut self` do próximo_jitter
            // não pode conviver com o empréstimo do `inbox`.
            let popped = match self.inbox.get_mut(&k) {
                Some(q) if !q.is_empty() => Some(q.remove(0)),
                _ => None,
            };
            if let Some(msg) = popped {
                // Latência de VOLTA (opt-in): a mensagem passa pelo fio
                // antes de chegar ao host.
                if self.latency.is_some() {
                    let jitter = self.proximo_jitter();
                    std::thread::sleep(jitter);
                }
                // USB instável: a n-ésima mensagem que SAÍ da fila é
                // PERDIDA no fio — consumida aqui (não reaparece) e o host
                // vê RecvTimeout com o device saudável. Sem latência
                // optada o retorno é imediato: a prova do drop é a fila
                // encurtar (a seguinte entrega normalmente), não o relógio.
                // O contador é de SAÍDAS, não de entregas — senão o segundo
                // drop prenderia toda a fila restante.
                self.popped += 1;
                if let Some(MockFault::UsbFlaky { drop_every, .. }) = self.fault {
                    if drop_every > 0 && self.popped.is_multiple_of(drop_every) {
                        return Err(TransportError::RecvTimeout {
                            timeout_ms: timeout.as_millis() as u64,
                        });
                    }
                }
                return Ok(msg);
            }
        }
        // Fila vazia: SEM latência o mock é o de sempre — RecvTimeout
        // IMEDIATO (o default que a suíte inteira conhece). COM latência
        // optada ele espelha o `real.rs`: respeita a JANELA da chamada e só
        // então acusa o silêncio. Aqui morria o falso positivo de tempo
        // (janela de 3s resolvida em microssegundos).
        if self.latency.is_some() {
            std::thread::sleep(timeout);
        }
        Err(TransportError::RecvTimeout {
            timeout_ms: timeout.as_millis() as u64,
        })
    }
}

/// O template `push` do endpoint (lado IN), se houver. Casa por ADDR: os
/// pushes do golden têm IN func 12, mas os READS chegam com func 11 — a
/// resposta deles é o push do MESMO endereço (func 12, como no fio).
fn push_template(
    golden: &GoldenFile,
    addr: [u8; 4],
) -> Result<&crate::golden::Template, ProtocolError> {
    golden
        .templates()
        .iter()
        .find(|t| {
            t.template_type == "push"
                && t.in_
                    .as_ref()
                    .is_some_and(|e| e.parsed().map(|(_, a)| a).is_ok_and(|a| a == addr))
        })
        .ok_or_else(|| shape_err("template push no golden", &format!("addr={addr:02x?}")))
}

/// Bytes do exemplo congelado do LADO IN (response_hex/hex).
fn example_bytes(tpl: &crate::golden::Template) -> Vec<u8> {
    let ex = &tpl.example;
    let hex = ex
        .response_hex
        .as_deref()
        .or(ex.hex.as_deref())
        .unwrap_or("");
    crate::golden::hex_decode(hex).unwrap_or_default()
}

/// O `i`-ésimo segmento var do exemplo (posicional), redimensionado para
/// `count` (o exemplo É a evidência; state-first entra por outro caminho).
fn example_var(
    tpl: &crate::golden::Template,
    len_hint: Option<usize>,
    i: usize,
    count: usize,
) -> Vec<u8> {
    let ex = example_bytes(tpl);
    let pat = tpl.response_pattern();
    let vars: Vec<Vec<u8>> = match pat {
        Some(p) => p
            .extract_vars(&ex)
            .map(|v| v.iter().map(|s| s.to_vec()).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    };
    // by-len: se o exemplo não casou (len ≠ sub do exemplo), tenta a partir
    // dos vars concatenados do sub-padrão pedido — simplificação: devolve
    // zeros para além do exemplo (shape do mock, documentado no módulo).
    match vars.get(i) {
        Some(v) => {
            let mut b = v.clone();
            b.resize(count, 0);
            b
        }
        None => {
            let _ = len_hint;
            vec![0u8; count]
        }
    }
}

/// Constrói a resposta do template com o exemplo congelado (`fill_custom`
/// substitui vars específicas — pp BE, echo de chunk).
fn build_from_example(
    tpl: &crate::golden::Template,
    desired_len: Option<usize>,
    fill_custom: Option<&dyn Fn(usize, usize) -> Vec<u8>>,
) -> Result<Vec<u8>, ProtocolError> {
    build_with_fill(tpl, desired_len, &mut |i, count| match fill_custom {
        Some(f) => f(i, count),
        None => example_var(tpl, desired_len, i, count),
    })
}

/// Constrói a resposta com fill arbitrário (var indexado).
fn build_with_fill(
    tpl: &crate::golden::Template,
    desired_len: Option<usize>,
    mut fill: impl FnMut(usize, usize) -> Vec<u8>,
) -> Result<Vec<u8>, ProtocolError> {
    tpl.build_response(&mut fill, desired_len)
}

/// Envelope SysEx completo (helper local sobre o header do crate).
fn envelope_of(func: u8, addr: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(SYSEX_HEADER.len() + 5 + payload.len() + 1);
    m.extend_from_slice(&SYSEX_HEADER);
    m.push(func);
    m.extend_from_slice(&addr);
    m.extend_from_slice(payload);
    m.push(SYSEX_EOX);
    m
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
