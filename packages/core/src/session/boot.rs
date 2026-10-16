//! o script de BOOT + SCAN (§13.10) — a única passada LONGA da sessão.
//!
//! A ordem é a da captura S1 (T1 tabelas → T5 scan → T6 sonda → T2 estado5 →
//! T3 nomes → T4 keepalives) e o replay byte-a-byte é o contrato
//! (`tests/replay_fixtures.rs`). `progress` é OBSERVACIONAL: um `beat!` por
//! transação, nada no fio. Fonte: prova C do `validate_golden.py`.

use crate::golden::GoldenFile;
use crate::transport::{DeviceTransport, WireKind};
use crate::ProtocolError;

use super::paginas::guarda_pagina;
use super::tipos::{BootProgress, BootProgressFn, BootReport, BootStage, StatePage};
use super::Session;

impl<T: DeviceTransport> Session<T> {
    /// Boot + scan (§13.10) SEM progresso — a forma canônica do ADR-6
    /// (assinatura/sequência intocadas; os contratos de replay a consomem).
    /// Para barra de progresso na UI, use [`Session::boot_with_progress`].
    ///
    /// # Erros
    /// [`ProtocolError::Timeout`] (D6), [`ProtocolError::InvalidShape`]
    /// (D5) — e [`ProtocolError::UnexpectedAck`] (diff hex da resposta).
    pub fn boot(&mut self) -> Result<BootReport, ProtocolError> {
        self.boot_inner(None)
    }

    /// Boot + scan (§13.10) com hook de progresso: chama `on_progress`
    /// a cada transação completada com [`BootProgress`] { stage, done, total }.
    /// O total é função do inventário (pp corrente duplicado soma +2 — quirk
    /// §13.4); `done` cresce monótono até `total` na última transação.
    ///
    /// # Erros
    /// Os mesmos de [`Session::boot`] (o hook é observacional — nada muda no
    /// fio: D1–D8 intocados).
    pub fn boot_with_progress(
        &mut self,
        on_progress: Option<&mut BootProgressFn<'_>>,
    ) -> Result<BootReport, ProtocolError> {
        self.boot_inner(on_progress)
    }

    /// Corpo do boot + scan (§13.10): o script completo do Suite ao ligar,
    /// gerado pelas MESMAS regras da prova C do `validate_golden.py`
    /// (2299/2299): T1) tabela de IRs `11/12001002`: 20 páginas ×2 leituras;
    /// T2) `11/12001012`: índices 0..4; T3) nomes `11/11000008`: bancos
    /// 0x00–0x02 completos (16) + 0x03 com 13; T4) keepalive `12/00020001`
    /// ×2; T5) scan: para cada pp (ordem do inventário; o ATUAL 0x0100 com
    /// select+open DUPLICADOS — quirk de boot): select `11/13010000` [pp],
    /// open `12/13010002` [pp]01, 9 páginas `12/13010004` [pp][PG]01;
    /// T6) sonda do banco 02: select 0000 + open 000001 + 9 páginas.
    ///
    /// INTERLEAVE (D2): cada transação espera a PRÓPRIA resposta (D1) e
    /// pushes de boot não solicitados (dump 13000000, meta6 13010001,
    /// páginas 13010003, estado 12001012, nomes 11000008) vão para o
    /// BACKLOG (D7), não confundem as transações. `progress` é
    /// observacional: um `beat!` por transação, nada no fio.
    fn boot_inner(
        &mut self,
        mut progress: Option<&mut BootProgressFn<'_>>,
    ) -> Result<BootReport, ProtocolError> {
        let golden = GoldenFile::embedded()?;
        let mut tx = 0usize;
        // Total esperado: T1(40) + scan(11/pp +2 se pp corrente duplicado)
        // + sonda(11) + estado5(5) + nomes(61) + keepalive(2). O replay da
        // captura (198 pps, 0x0100 duplicado) fecha em 2299 = prova C.
        let pps = self.inventory();
        let doubled = pps.iter().filter(|&&p| p == 0x0100).count();
        let total = 40 + pps.len() * 11 + doubled * 2 + 11 + 5 + 61 + 2;
        let mut stage = BootStage::Tables;
        macro_rules! beat {
            () => {
                if let Some(cb) = progress.as_mut() {
                    cb(BootProgress {
                        stage,
                        done: tx,
                        total,
                        current_pp: self.current_pp,
                    });
                }
            };
        }

        // ORDEM REAL do boot S1 (prova C / replay): T1 → scan (T5) →
        // sonda 1302 (T6) → estado5 (T2) → nomes (T3) → keepalives ×2.
        // T1: 20 páginas ×2 (regra da prova C)
        for p in 0u8..0x14 {
            for _ in 0..2 {
                self.tx_req(golden, 0x11, &[0x12, 0x00, 0x10, 0x02], &[p])?;
                tx += 1;
                beat!();
            }
        }
        stage = BootStage::Scan;
        // T5: scan — pareamento REAL (replay S1): select `13010000` →
        // meta6 `13010001`; open `13010002` [pp]01 → página 0 em `13010003`;
        // req `13010004` [pp][PG]01 (PG 0..7) → página PG+1 (196B/32B);
        // PG 8 → `13010005` (4B). O preset ATUAL 0x0100 tem select e open
        // DUPLICADOS (quirk §13.4: 2 selects + 2 opens; página 0 extra).
        for pp in self.inventory() {
            let pp_be = pp.to_be_bytes();
            let doubled = pp == 0x0100;
            if doubled {
                self.tx_req_in(
                    golden,
                    0x11,
                    &[0x13, 0x01, 0x00, 0x00],
                    &pp_be,
                    &[0x13, 0x01, 0x00, 0x01],
                    WireKind::Read,
                )?;
                tx += 1;
                beat!();
            }
            self.tx_req_in(
                golden,
                0x11,
                &[0x13, 0x01, 0x00, 0x00],
                &pp_be,
                &[0x13, 0x01, 0x00, 0x01],
                WireKind::Read,
            )?;
            tx += 1;
            beat!();
            // O `open` entrega a página 0 (`raw[3] == 0`) — as 9 páginas
            // do pp, capturadas sem mudar o fio: o scan já esperava cada
            // resposta em `wait_for`; só o VALOR era descartado com `?`.
            let mut paginas: [StatePage; 9] =
                std::array::from_fn(|_| StatePage { raw: Vec::new() });
            let abertura = self.tx_req_in(
                golden,
                0x12,
                &[0x13, 0x01, 0x00, 0x02],
                &pp_be,
                &[0x13, 0x01, 0x00, 0x03],
                WireKind::Read,
            )?;
            guarda_pagina(&mut paginas, abertura);
            tx += 1;
            beat!();
            if doubled {
                // ...e open duplicado: a página 0 chega DE NOVO em
                // `13010003` (captura S1 rows 89–93: open open → pág0 pág0;
                // NÃO é meta6 — era este o desalinhamento do replay)
                let abertura2 = self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x01, 0x00, 0x02],
                    &pp_be,
                    &[0x13, 0x01, 0x00, 0x03],
                    WireKind::Read,
                )?;
                guarda_pagina(&mut paginas, abertura2);
                tx += 1;
                beat!();
            }
            for pg in 0u16..9u16 {
                // t8: var2 (pp) + const 00 + var1 (PG baixo) + const 01
                let vars = [pp_be[0], pp_be[1], pg as u8];
                let in_addr: &[u8; 4] = if pg < 8 {
                    &[0x13, 0x01, 0x00, 0x03]
                } else {
                    &[0x13, 0x01, 0x00, 0x05]
                };
                // PG 0..7 respondem em `13010003` com a página SEGUINTE
                // (`raw[3] == pg+1`); PG 8 responde em `13010005` com um
                // ACK de 4B — `guarda_pagina` distingue pelo comprimento.
                let resposta = self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x01, 0x00, 0x04],
                    &vars,
                    in_addr,
                    WireKind::Read,
                )?;
                guarda_pagina(&mut paginas, resposta);
                tx += 1;
                beat!();
            }
            self.pages.insert(pp, paginas);
            self.current_pp = pp;
        }
        stage = BootStage::Probe;
        // T6: sonda do banco 02 (mesmo pareamento; 13020001/13020003/13020005).
        // O select é CONST "0000" no golden (sem var de pp — é a sonda do
        // banco 02, não um select de preset).
        self.tx_req_in(
            golden,
            0x11,
            &[0x13, 0x02, 0x00, 0x00],
            &[],
            &[0x13, 0x02, 0x00, 0x01],
            WireKind::Read,
        )?;
        tx += 1;
        beat!();
        self.tx_req_in(
            golden,
            0x12,
            &[0x13, 0x02, 0x00, 0x02],
            &[],
            &[0x13, 0x02, 0x00, 0x03],
            WireKind::Read,
        )?;
        tx += 1;
        beat!();
        for pg in 0u16..9u16 {
            // t15: const 000000 + var1 (PG baixo) + const 01
            let vars = [pg as u8];
            if pg < 8 {
                self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x02, 0x00, 0x04],
                    &vars,
                    &[0x13, 0x02, 0x00, 0x03],
                    WireKind::Read,
                )?;
            } else {
                self.tx_req_in(
                    golden,
                    0x12,
                    &[0x13, 0x02, 0x00, 0x04],
                    &vars,
                    &[0x13, 0x02, 0x00, 0x05],
                    WireKind::Read,
                )?;
            }
            tx += 1;
            beat!();
        }
        stage = BootStage::State5;
        // T2: 5 entradas de `12001012` (estado; não é setlist — GP100_DEVICE §5#1)
        for i in 0u8..5 {
            self.tx_req(golden, 0x11, &[0x12, 0x00, 0x10, 0x12], &[i])?;
            tx += 1;
            beat!();
        }
        stage = BootStage::Names;
        // T3: nomes — FIRE-AND-FORGET (D4; captura S1: 61 leituras, só 57
        // respostas — o device omitiu 4 do banco 00 [idx 01,03,06,0a] e o
        // Suite seguiu): respostas = pushes de contexto (D2) → backlog (D7);
        // esperar por-leitura estouraria Timeout (D6). Vars = [banco, idx]:
        // o template é mixed 2 vars + const 0000 (o golden completa a cauda).
        for bank in 0u8..3 {
            for idx in 0u8..16 {
                self.send_build(
                    golden,
                    0x11,
                    &[0x11, 0x00, 0x00, 0x08],
                    &[bank, idx],
                    WireKind::Read,
                )?;
                tx += 1;
                beat!();
            }
        }
        for idx in 0u8..13 {
            self.send_build(
                golden,
                0x11,
                &[0x11, 0x00, 0x00, 0x08],
                &[3, idx],
                WireKind::Read,
            )?;
            tx += 1;
            beat!();
        }
        stage = BootStage::Keepalive;
        // T4: keepalive ×2 (D4) — contam como transação para o progresso.
        //
        // Um transporte que NÃO deixa escrever (ADR-5 com a trava fechada)
        // recusaria este frame, e o boot inteiro cairia no ÚLTIMO passo —
        // um build de leitura ficaria sem leitura. Por isso ele é OMITIDO,
        // não mandado e ignorado: a contagem do relatório é do que de fato
        // saiu no fio. Decisão do owner (06/10, face (A) da #126); a
        // alternativa registrada em `tests/write_gate.rs` era exigir
        // `write-verified` no passo B5 do H1 — a leitura não podia existir
        // antes disso.
        if self.transport.permite_escrita() {
            for _ in 0..2 {
                self.send_build(
                    golden,
                    0x12,
                    &[0x00, 0x02, 0x00, 0x01],
                    &[],
                    WireKind::Write,
                )?;
                tx += 1;
                beat!();
            }
        }
        // #161 — medição honesta do que saiu do fio: o inventário que o
        // scan deixou no cache e quantos nomes a pg0 deles decodifica.
        // Só conta; quem interpreta ("198/198 do catálogo") é o gate do
        // front — aqui o relatório não mente nem inventa.
        let presets = self.pages.len();
        let mut names = 0usize;
        for pags in self.pages.values() {
            if let Ok(pg) = crate::preset_pages::decode(pags) {
                if pg.nome().is_ok() {
                    names += 1;
                }
            }
        }
        Ok(BootReport {
            transactions: tx,
            presets,
            names,
        })
    }
}
