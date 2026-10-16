//! as PÁGINAS de estado do preset (família 13xx): ler do fio e cachear.
//!
//! `reload_paginas_correntes` é o read-back do #164 (tudo `WireKind::Read`) e
//! o cache é o que o app consulta sem tocar no fio (`preset_state`). Fonte:
//! `docs/tecnico/PROTOCOL.md` §13.10 e `crate::preset_pages` (o decode).

use crate::golden::GoldenFile;
use crate::transport::{DeviceTransport, WireKind};
use crate::ProtocolError;

use super::tipos::StatePage;
use super::Session;

/// Alimenta o array de páginas de um pp com a resposta de uma transação.
///
/// O índice da página vem do **próprio payload** (`raw[3]`, 0..=8) — não da
/// posição do pedido: o `open` `13010002` entrega a página 0, os reqs
/// `13010004` PG 0..7 entregam as páginas 1..8, e o req PG 8 devolve um
/// ACK de 4B em `13010005` que NÃO é página. Só o comprimento certifica:
/// 196B (4B header + 192B) ou 32B (4B + 28B) é corpo de página.
///
/// Sem efeito no fio — só o VALOR, que já era esperado por `wait_for`.
pub(super) fn guarda_pagina(paginas: &mut [StatePage; 9], raw: Vec<u8>) {
    if !matches!(raw.len(), 196 | 32) {
        return; // ACK de 4B, push ou meta6 — não é corpo de página
    }
    if let Some(idx) = raw.get(3).copied().filter(|&i| i < 9) {
        paginas[idx as usize] = StatePage { raw };
    }
}

impl<T: DeviceTransport> Session<T> {
    /// Uma página (0..=8) do preset selecionado (§13.10): req
    /// `12/13010004 [pp][PG]01` — PG 0..7 respondem em `13010003` (by-len
    /// 196/32B), PG 8 em `13010005` (4B). Layout byte-a-byte da 13xx segue
    /// Fora de escopo: a página devolve os bytes crus em [`StatePage`].
    pub fn state_page(&mut self, page: u8) -> Result<StatePage, ProtocolError> {
        if page > 8 {
            return Err(ProtocolError::InvalidShape {
                expected: "página 0..=8 (§13.10)".into(),
                got: format!("{page}"),
            });
        }
        let golden = GoldenFile::embedded()?;
        let pp_be = self.current_pp.to_be_bytes();
        let vars = [pp_be[0], pp_be[1], page];
        let in_addr: &[u8; 4] = if page < 8 {
            &[0x13, 0x01, 0x00, 0x03]
        } else {
            &[0x13, 0x01, 0x00, 0x05]
        };
        let payload = self.tx_req_in(
            golden,
            0x12,
            &[0x13, 0x01, 0x00, 0x04],
            &vars,
            in_addr,
            WireKind::Read,
        )?;
        Ok(StatePage { raw: payload })
    }

    /// Página 0 do pp corrente (equivalente a
    /// [`Session::state_page`]`(0)` — mantida para os contratos existentes).
    pub fn scan_state(&mut self) -> Result<StatePage, ProtocolError> {
        self.state_page(0)
    }

    /// Recarrega as 9 páginas do preset corrente DIRETO do fio (read-back
    /// do #164), pela sequência do boot (§13.10): `select` + `open` — o
    /// `open` entrega a página 0 (`raw[3] == 0`, o slot 0 do decode) — e
    /// os reqs PG 0..7, que respondem as páginas 1..8 (`raw[3] == PG + 1`,
    /// §13.10).
    ///
    /// **Por que não é um laço de [`Session::state_page`]:** aquela devolve
    /// o payload CRU do req — na ordem 0..8 o slot 0 viraria o número 1 e o
    /// [`crate::preset_pages::decode`] cairia em "página fora de ordem:
    /// esperada 0, achada 1". Quem preenche o slot 0 é o `open`, e o
    /// `state_page` não faz `open` (o caminho do Dump lê a página solta,
    /// sem decode das 9).
    ///
    /// Atualiza o cache de [`Session::preset_state`] do pp corrente: quem
    /// relê do fio, atualiza — o aparelho vence.
    ///
    /// # Erros
    /// Timeout D6, shape D5 e `DeviceGone` do fio. Nenhum frame de escrita
    /// sai daqui: toda a sequência é `WireKind::Read`.
    pub fn reload_paginas_correntes(&mut self) -> Result<[StatePage; 9], ProtocolError> {
        let pp = self.current_pp;
        // Select antes do open, como o T5 do boot: a seleção é estado do
        // APARELHO (um replug sem reboot pode perdê-la) — sem ela o read-
        // back leria o preset errado em silêncio e reconciliaria o cache
        // com dados de outro pp.
        self.select_preset(pp)?;
        let golden = GoldenFile::embedded()?;
        let pp_be = pp.to_be_bytes();
        let mut paginas: [StatePage; 9] = std::array::from_fn(|_| StatePage { raw: Vec::new() });
        let abertura = self.tx_req_in(
            golden,
            0x12,
            &[0x13, 0x01, 0x00, 0x02],
            &pp_be,
            &[0x13, 0x01, 0x00, 0x03],
            WireKind::Read,
        )?;
        guarda_pagina(&mut paginas, abertura);
        for pg in 0u16..8u16 {
            // t8: var2 (pp) + const 00 + var1 (PG baixo) + const 01.
            let vars = [pp_be[0], pp_be[1], pg as u8];
            let resposta = self.tx_req_in(
                golden,
                0x12,
                &[0x13, 0x01, 0x00, 0x04],
                &vars,
                &[0x13, 0x01, 0x00, 0x03],
                WireKind::Read,
            )?;
            guarda_pagina(&mut paginas, resposta);
        }
        self.pages.insert(pp, paginas.clone());
        Ok(paginas)
    }

    /// As 9 páginas brutas de um pp, guardadas pelo scan (T5).
    ///
    /// `None` antes do boot (ou para pp fora do inventário escaneado) — é
    /// a matéria-prima de [`crate::preset_pages::decode`]: o MESMO payload
    /// que [`Session::state_page`] mandaria buscar agora, só que sem tocar
    /// no fio (o boot já leu tudo).
    pub fn preset_state(&self, pp: u16) -> Option<&[StatePage; 9]> {
        self.pages.get(&pp)
    }

    /// Pps com cache de páginas, em ordem crescente (o escaneado no boot).
    pub fn cached_pps(&self) -> Vec<u16> {
        self.pages.keys().copied().collect()
    }
}
