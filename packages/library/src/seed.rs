//! Seed de fábrica: os 99 presets do `all.prst` entram no banco (ADR-9, decisão 4).
//!
//! **Uma implementação do `.prst`, não duas.** O formato tem parser no
//! `gp100-core` (`preset::Document`) e ele é o dono. O seed chama esse parser —
//! se o `all.prst` mudar, quem lê o formato novo é o mesmo código de antes, e um
//! segundo leitor aqui seria um segundo lugar para o `.prst` divergir.
//!
//! **Idempotente.** Rodar o seed duas vezes regrava os mesmos 99 registros com o
//! mesmo id (`f{pp}`) e não duplica nada. Isso importa porque o seed roda no
//! boot: um app que sobe e acha a biblioteca com 198 presets porque "rodou o
//! seed duas vezes" é um bug que a UI não consegue explicar.

use gp100_core::preset::{pp_id_decimal, Document};

use crate::{Bank, Library, LibraryError, Preset};

/// Quantos presets o `all.prst` de fábrica tem (§13.9; o índice vai de 0).
pub const TOTAL_DE_FABRICA: usize = 99;

impl Library {
    /// Importa o `all.prst` como registros de fábrica.
    ///
    /// Devolve quantos entraram. `all_prst` são os bytes crus do arquivo.
    pub fn seed_factory(&self, all_prst: &[u8], agora: &str) -> Result<usize, LibraryError> {
        let doc = Document::parse(all_prst).map_err(|e| LibraryError::Prst(e.to_string()))?;

        let mut n = 0usize;
        for preset in doc.presets() {
            let Some(pp_txt) = preset.pp_id() else {
                continue;
            };
            // Mesma base do core (`preset_list`, `board_view_for`) e do
            // artefato do front: `ppID` DECIMAL — #132/ADR-12. A função é
            // do core de propósito: se a base virar outra um dia, ela
            // muda num lugar só e este teste cruzado (`seed.rs`) acusa.
            let Some(pp) = pp_id_decimal(pp_txt) else {
                return Err(LibraryError::Prst(format!("ppID nao numerico: {pp_txt:?}")));
            };
            let nome = preset.pp_name().unwrap_or("").trim().to_string();
            let tipo_txt = preset.pp_type().unwrap_or("0");
            let pp_type: u16 = tipo_txt.trim().parse().unwrap_or(0);

            self.upsert(&Preset {
                id: format!("f{pp}"),
                bank: Bank::Factory,
                pp: Some(pp),
                name: nome,
                pp_type,
                pp_type_name: preset.pp_type_name().unwrap_or("").trim().to_string(),
                saved_at: agora.to_string(),
                payload: None,
            })?;
            n += 1;
        }
        Ok(n)
    }

    /// Já semeou a fábrica? A UI chama antes de mostrar a biblioteca para não
    /// desenhar lista vazia no primeiro boot.
    pub fn tem_fabrica(&self) -> Result<bool, LibraryError> {
        let n: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM preset WHERE bank = 'factory'",
            [],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }
}
