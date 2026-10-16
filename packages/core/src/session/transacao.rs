//! a TRANSAÇÃO do fio (ADR-3/ADR-6): montar o pedido, enviar, esperar a resposta
//! do endpoint certo e validar a FORMA.
//!
//! D1 (a transação é dona do endpoint), D5 (sem match = `InvalidShape`), D6
//! (sem retry: `TX_TIMEOUT_MS` sobe como `Timeout`) e D7 (msg de outro endpoint
//! vai para o backlog) vivem AQUI — os módulos de coorte só descrevem o que
//! pedir. Fonte: ADR-6 rev.3; `docs/tecnico/PROTOCOL.md` §13.

use std::time::Duration;

use crate::golden::GoldenFile;
use crate::transport::{DeviceTransport, TransportError, WireKind};
use crate::ProtocolError;

use super::Session;

/// Janela da transação (ADR-3): 3000 ms por request→resposta.
pub(super) const TX_TIMEOUT_MS: u64 = 3000;

impl<T: DeviceTransport> Session<T> {
    /// Uma transação: build_request (desambiguação por var_count) → send →
    /// espera a resposta no endpoint de RESPOSTA (D1/D7). Devolve o payload.
    pub(super) fn tx_req(
        &mut self,
        golden: &GoldenFile,
        func_out: u8,
        addr: &[u8; 4],
        vars: &[u8],
    ) -> Result<Vec<u8>, ProtocolError> {
        self.tx_req_in(golden, func_out, addr, vars, addr, WireKind::Read)
    }

    /// Igual a [`Session::tx_req`], com endpoint de RESPOSTA explícito —
    /// o fio NÃO responde no mesmo endereço (replay S1): select/open
    /// `13010002` respondem meta6 em `13010001`; página `13010004` (PG 0..7)
    /// responde página em `13010003` (by-len 196/32); PG 8 responde em
    /// `13010005`; select `13010000` (família 1302) responde meta6 em
    /// `13020001`; read `13020004` responde em `13020003`; PG 8 em `13020005`.
    pub(super) fn tx_req_in(
        &mut self,
        golden: &GoldenFile,
        func_out: u8,
        addr: &[u8; 4],
        vars: &[u8],
        in_addr: &[u8; 4],
        kind: WireKind,
    ) -> Result<Vec<u8>, ProtocolError> {
        let req = golden.build_request(func_out, addr, vars)?;
        self.transport.send_raw(&req, kind).map_err(tx_err)?;
        self.wait_for(0x12, in_addr)
    }

    /// Só envia (write fire-and-forget, D4).
    pub(super) fn send_build(
        &mut self,
        golden: &GoldenFile,
        func: u8,
        addr: &[u8; 4],
        vars: &[u8],
        kind: WireKind,
    ) -> Result<(), ProtocolError> {
        let req = golden.build_request(func, addr, vars)?;
        self.transport.send_raw(&req, kind).map_err(tx_err)
    }

    /// Espera uma mensagem no endpoint `(func, addr)` e devolve o PAYLOAD
    /// (D1/D7): msgs de outros endpoints vão para o backlog; no endpoint
    /// certo, sem match no golden = InvalidShape (D5); nada na janela ADR-3
    /// = Timeout (D6).
    pub(super) fn wait_for(&mut self, func: u8, addr: &[u8; 4]) -> Result<Vec<u8>, ProtocolError> {
        let golden = GoldenFile::embedded()?;
        self.wait_for_com(func, addr, Forma::Golden(golden))
    }

    /// [`wait_for`] com a validação de forma PLUGÁVEL. Por que existe: os
    /// registradores globais da §13.13 não estão no golden (aquele arquivo
    /// é o mundo do preset) — a resposta `12/000200xx` é validada pelos
    /// COMPRIMENTOS medidos em G3–G6 (§13.13.1), não por template.
    pub(super) fn wait_for_com(
        &mut self,
        func: u8,
        addr: &[u8; 4],
        forma: Forma<'_>,
    ) -> Result<Vec<u8>, ProtocolError> {
        loop {
            let msg = self
                .transport
                .recv_raw(Duration::from_millis(TX_TIMEOUT_MS))
                .map_err(|e| match e {
                    TransportError::RecvTimeout { timeout_ms } => ProtocolError::Timeout {
                        timeout_ms,
                        addr: format!("{func:02x}/{}", addr_hex(addr)),
                    },
                    // DeviceGone tem TIPO próprio (o front decide diferente:
                    // LED off + retry explícito, sem retry automático).
                    TransportError::DeviceGone { why } => ProtocolError::DeviceGone { why },
                    other => ProtocolError::InvalidShape {
                        expected: "transporte saudável".into(),
                        got: other.to_string(),
                    },
                })?;
            let (f, a, payload) = crate::golden::decode_envelope(&msg)?;
            if std::env::var_os("GP100_TRACE").is_some() {
                eprintln!(
                    "wait_for {func:02x}/{} <- {f:02x}/{} ({}B)",
                    addr_hex(addr),
                    addr_hex(&a),
                    payload.len()
                );
            }
            if f == func && a == *addr {
                // FORMA só (#177): endereço + comprimento (by-len) são
                // schema; os bytes `const` do golden são o estado de fábrica
                // do aparelho ANALISADO, e o dono editou o dele (renomear um
                // pp muda pg0 — medido no boot do H4). O conteúdo é validado
                // pelo decode dos objetos (`preset_pages`, `codec`), que
                // conhecem o schema dos dados.
                forma.valida(func, addr, payload)?;
                return Ok(payload.to_vec());
            }
            self.backlog.push(msg); // D7
        }
    }
}

/// Mapeia erro de transporte para erro de transação (D6/D5).
///
/// `DeviceGone` NÃO é achatado em `InvalidShape`: o device ausente é uma
/// falha com tipo próprio (a UI para de tentar e mostra recuperação; o H1
/// trata como divergência de campo).
/// Forma da resposta esperada por `Session::wait_for_com`.
pub(super) enum Forma<'a> {
    /// Golden by-len (#177) — o caminho de sempre (mundo do preset).
    Golden(&'a GoldenFile),
    /// Comprimentos MEDIDOS (§13.13.1) — a família `000102xx`/`000200xx`
    /// não está no golden; a forma é a da sessão G3–G6.
    Medido(&'a [usize]),
}

impl Forma<'_> {
    fn valida(&self, func: u8, addr: &[u8; 4], payload: &[u8]) -> Result<(), ProtocolError> {
        match self {
            Forma::Golden(g) => {
                g.match_shape(func, addr, payload)
                    .ok_or_else(|| ProtocolError::InvalidShape {
                        expected: format!(
                            "forma de resposta do golden em {func:02x}/{}",
                            addr_hex(addr)
                        ),
                        got: hex_short(payload),
                    })?;
                Ok(())
            }
            Forma::Medido(lens) => {
                if lens.contains(&payload.len()) {
                    return Ok(());
                }
                let esperado = lens
                    .iter()
                    .map(|n| format!("{n}B"))
                    .collect::<Vec<_>>()
                    .join(" ou ");
                Err(ProtocolError::InvalidShape {
                    expected: format!(
                        "payload {esperado} medido (§13.13.1) em {func:02x}/{}",
                        addr_hex(addr)
                    ),
                    got: hex_short(payload),
                })
            }
        }
    }
}

pub(super) fn tx_err(e: TransportError) -> ProtocolError {
    match e {
        TransportError::SendFailed { why } => ProtocolError::InvalidShape {
            expected: "frame aceito pelo device".into(),
            got: why,
        },
        TransportError::DeviceGone { why } => ProtocolError::DeviceGone { why },
        other => ProtocolError::InvalidShape {
            expected: "transporte saudável".into(),
            got: other.to_string(),
        },
    }
}

/// Hex curto (até 12 bytes) para diffs de divergência.
pub(super) fn hex_short(data: &[u8]) -> String {
    let s: String = data.iter().take(12).map(|b| format!("{b:02x}")).collect();
    if data.len() > 12 {
        format!("{s}…(+{})", data.len() - 12)
    } else {
        s
    }
}

/// Addr 4B em hex (mensagens de erro).
fn addr_hex(addr: &[u8; 4]) -> String {
    addr.iter().map(|b| format!("{b:02x}")).collect()
}
