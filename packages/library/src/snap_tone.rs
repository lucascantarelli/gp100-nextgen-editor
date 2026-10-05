//! Tons de SnapTone/NAM: o armazenamento dos modelos convertidos (issue #25).
//!
//! **O que é guardado aqui.** O arquivo `.clo` que o Valeton Suite produz a
//! partir de um `.nam` — as strings do binário (`exe_strings.txt`) mostram o
//! fluxo inteiro: `Choose a nam file to open it` → `*.nam` → a conversão →
//! `/name.clo` + `nam_output_clo.wav`. O `.nam` é a ENTRADA do `clone` e o
//! `.clo` é a SAÍDA: é o `.clo` que vai para o device. Como a conversão
//! acontece no desktop (o motor NAM mora no exe da Valeton — `BLOCKERS.md`),
//! este crate **guarda bytes**, nunca os fabrica.
//!
//! **Duas tabelas, e por quê.** A lista do gestor é leve (nome, tamanho, CRC,
//! slot) e o modelo são ~2,7 KB por tom. Se o blob morasse na mesma tabela,
//! a busca da lista pagaria 2,7 KB por linha para mostrar um nome — o mesmo
//! motivo que separou `has_payload` dos presets na #26. A leitura do modelo é
//! por id, e é ela que a tela de A/B e o upload pedem.
//!
//! **O CRC é NOSSO, não do device.** Ele responde a uma pergunta do app: "os
//! bytes que saíram daqui são os mesmos que entraram?" — conteúdo trocado em
//! disco, import truncado, `.clo` regravado por fora. Não é o `ppIRCRC` do
//! `.prst` nem o CRC-8/SMBUS do §3 do SnapTone, que têm polinômios e
//! refleções diferentes. Por isso o valor canônico do teste é o de IEEE
//! (`123456789` → `CBF43926`) e não um número que "o device usaria".

use serde::{Deserialize, Serialize};

use crate::{Library, LibraryError};

/// Quantidade de slots de SnapTone no GP-100 V2.1 (`SnapTone1..5`, §5).
///
/// Reexporta o valor do core em vez de repetir o `5`: a biblioteca e a FSM
/// precisam concordar sobre quantos slots existem, e dois literais numéricos em
/// crates diferentes são um `CHECK` que aceita 5 e uma validação que aceita 6.
pub const SLOTS: u8 = gp100_core::snap_tone::SLOTS;

/// CRC-32 IEEE (polinômio refletido `0xEDB88320`, init e XOR final `0xFFFFFFFF`).
///
/// É o mesmo do `zlib.crc32`/`crc32` de Python — o que permite provar o
/// resultado contra uma implementação de referência de verdade, e não contra
/// esta mesma função reescrita.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Um tom da biblioteca, como a LISTA mostra (sem o modelo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneRow {
    /// Identificador estável, derivado do conteúdo (`t<crc32 hex>`).
    pub id: String,
    /// Nome que o dono deu.
    pub name: String,
    /// Tamanho do `.clo` em bytes.
    pub bytes: i64,
    /// CRC-32 do `.clo` — a prova de integridade do conteúdo guardado.
    pub crc32: i64,
    /// Slot do device atribuído (1..=5), ou `None` se ainda não foi.
    pub slot: Option<u8>,
    /// ISO-8601 da importação.
    pub saved_at: String,
}

/// Um tom COM o modelo — o que a tela de A/B e o upload pedem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tone {
    /// A linha da lista.
    #[serde(flatten)]
    pub row: ToneRow,
    /// Os bytes do `.clo`.
    pub model: Vec<u8>,
}

/// Colunas da listagem, na ordem do DTO. Uma constante para o SELECT e para o
/// `row`: dois lugares para lembrar a ordem é como um campo troca de nome no
/// caminho.
const COLUNAS: &str = "id, name, bytes, crc32, slot, saved_at";

impl Library {
    /// Importa um `.clo` como tom da biblioteca.
    ///
    /// **O id é derivado do conteúdo**, não de um contador: importar o mesmo
    /// arquivo duas vezes atualiza o mesmo registro em vez de criar um
    /// duplicado — que é o que o dono quer quando reorganiza a pasta de
    /// arquivos. A colisão de CRC (dois arquivos diferentes com o mesmo CRC-32)
    /// não sobrescreve nada: o segundo recebe o sufixo `-2`, e o teste
    /// `id_derivado_do_conteudo_nao_sobrescreve_outro_arquivo` prova que os
    /// dois coexistem.
    ///
    /// `saved_at` é ISO-8601 e vem de quem chama — o crate não tem relógio.
    ///
    /// # Erros
    /// [`LibraryError::Tone`] se o nome for vazio ou o modelo tiver 0 bytes
    /// (um registro sem modelo é um registro que o upload não consegue
    /// usar), e [`LibraryError::Sqlite`] se o banco falhar.
    pub fn import_tone(
        &self,
        name: &str,
        model: &[u8],
        saved_at: &str,
    ) -> Result<ToneRow, LibraryError> {
        if name.trim().is_empty() {
            return Err(LibraryError::Tone("nome vazio".into()));
        }
        if model.is_empty() {
            return Err(LibraryError::Tone("modelo de 0 bytes".into()));
        }
        let crc = crc32(model);
        let id = self.id_para(crc, model)?;
        // Transação: a linha e o blob entram juntos ou nenhum entra. Sem
        // transação, uma falha na segunda INSERT deixaria um tom listado cujo
        // modelo não existe — e a tela de A/B mostraria "não encontrado" para
        // um tom que ela acabou de listar.
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO snap_tone (id, name, bytes, crc32, slot, saved_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                bytes = excluded.bytes,
                crc32 = excluded.crc32,
                saved_at = excluded.saved_at",
            rusqlite::params![id, name.trim(), model.len() as i64, crc as i64, saved_at],
        )?;
        tx.execute(
            "INSERT INTO snap_tone_model (id, model) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET model = excluded.model",
            rusqlite::params![id, model],
        )?;
        tx.commit()?;
        self.ton(&id)?
            .ok_or_else(|| LibraryError::Tone("o tom importado sumiu do banco".into()))
    }

    /// O id de um conteúdo: o CRC em hexadecimal, com sufixo se outro arquivo
    /// já ocupou o nome com bytes diferentes.
    ///
    /// Devolve o id EXISTENTE quando o conteúdo é igual ao que já está lá —
    /// é o que faz a reimportação ser um update e não um duplicado.
    fn id_para(&self, crc: u32, model: &[u8]) -> Result<String, LibraryError> {
        let base = format!("t{crc:08x}");
        let mut id = base.clone();
        let mut n = 2u32;
        loop {
            match self.modelo(&id)? {
                None => return Ok(id),
                Some(guardado) if guardado == model => return Ok(id),
                Some(_) => {
                    id = format!("{base}-{n}");
                    n += 1;
                }
            }
        }
    }

    /// Todos os tons, na ordem que o gestor mostra: os atribuídos primeiro,
    /// pelo número do slot, e os soltos pelo nome.
    ///
    /// A ordem é do arquivo e não do `id`: o dono procura "qual é o tom do
    /// slot 3", e ordenar por CRC mostraria essa resposta em outra linha a
    /// cada importação.
    pub fn tons(&self) -> Result<Vec<ToneRow>, LibraryError> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLUNAS} FROM snap_tone ORDER BY slot IS NULL, slot, name"
        ))?;
        let linhas = stmt.query_map([], linha_tone)?;
        let mut saida = Vec::new();
        for l in linhas {
            saida.push(l?);
        }
        Ok(saida)
    }

    /// Uma linha da lista pelo id. `None` = não existe.
    pub fn ton(&self, id: &str) -> Result<Option<ToneRow>, LibraryError> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLUNAS} FROM snap_tone WHERE id = ?1"),
                [id],
                linha_tone,
            )
            .ok())
    }

    /// Os bytes do modelo de um tom. `None` = não existe.
    pub fn modelo(&self, id: &str) -> Result<Option<Vec<u8>>, LibraryError> {
        let mut stmt = self
            .conn()
            .prepare("SELECT model FROM snap_tone_model WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row.get(0)?))
    }

    /// O tom COM o modelo, que é o que o upload e a tela de A/B pedem.
    pub fn ton_com_modelo(&self, id: &str) -> Result<Option<Tone>, LibraryError> {
        let Some(row) = self.ton(id)? else {
            return Ok(None);
        };
        Ok(Some(Tone {
            row,
            model: self.modelo(id)?.unwrap_or_default(),
        }))
    }

    /// Renomeia. `false` = não existia.
    pub fn renomear_tone(&self, id: &str, nome: &str) -> Result<bool, LibraryError> {
        if nome.trim().is_empty() {
            return Err(LibraryError::Tone("nome vazio".into()));
        }
        let n = self.conn().execute(
            "UPDATE snap_tone SET name = ?2 WHERE id = ?1",
            rusqlite::params![id, nome.trim()],
        )?;
        Ok(n > 0)
    }

    /// Atribui (ou desliga, com `None`) o slot de um tom.
    ///
    /// **Um slot, um tom.** O índice único parcial em `slot` faz o banco
    /// recusar a segunda atribuição, e a checagem antes do `UPDATE` transforma
    /// isso em erro com o nome do dono do slot: "o slot 3 é do 'Marshall 4x12'"
    /// é uma resposta; `UNIQUE constraint failed` é um stack trace.
    ///
    /// # Erros
    /// [`LibraryError::SlotInvalido`] fora de 1..=5,
    /// [`LibraryError::SlotOcupado`] quando outro tom já tem o slot, e
    /// [`LibraryError::Tone`] quando o tom não existe.
    pub fn atribuir_slot(&self, id: &str, slot: Option<u8>) -> Result<(), LibraryError> {
        if let Some(s) = slot {
            if s == 0 || s > SLOTS {
                return Err(LibraryError::SlotInvalido(s as u32, SLOTS));
            }
            if let Some(dono) = self.dono_do_slot(s)? {
                if dono != id {
                    let nome = self.ton(&dono)?.map_or(dono.clone(), |t| t.name);
                    return Err(LibraryError::SlotOcupado {
                        slot: s,
                        dono: nome,
                    });
                }
            }
        }
        let n = self.conn().execute(
            "UPDATE snap_tone SET slot = ?2 WHERE id = ?1",
            rusqlite::params![id, slot],
        )?;
        if n == 0 {
            return Err(LibraryError::Tone(format!("tom {id} inexistente")));
        }
        Ok(())
    }

    /// Quem ocupa um slot, se alguém ocupar.
    pub fn dono_do_slot(&self, slot: u8) -> Result<Option<String>, LibraryError> {
        Ok(self
            .conn()
            .query_row("SELECT id FROM snap_tone WHERE slot = ?1", [slot], |r| {
                r.get(0)
            })
            .ok())
    }

    /// O tom de um slot, se houver (o que a tela de A/B mostra em "A").
    pub fn ton_do_slot(&self, slot: u8) -> Result<Option<Tone>, LibraryError> {
        let Some(id) = self.dono_do_slot(slot)? else {
            return Ok(None);
        };
        self.ton_com_modelo(&id)
    }

    /// Apaga o tom e o modelo junto. `false` = não existia.
    ///
    /// O `ON DELETE CASCADE` apaga o blob; apagar a linha e deixar o blob seria
    /// o tipo de lixo que só aparece quando o dono tenta importar de novo e o
    /// banco cresce até encher.
    pub fn apagar_tone(&self, id: &str) -> Result<bool, LibraryError> {
        Ok(self
            .conn()
            .execute("DELETE FROM snap_tone WHERE id = ?1", [id])?
            > 0)
    }

    /// Nº de tons guardados (rodapé do gestor).
    pub fn total_tons(&self) -> Result<i64, LibraryError> {
        Ok(self
            .conn()
            .query_row("SELECT COUNT(*) FROM snap_tone", [], |r| r.get(0))?)
    }
}

/// Uma linha da tabela `snap_tone` → [`ToneRow`] (ordem = [`COLUNAS`]).
fn linha_tone(r: &rusqlite::Row<'_>) -> rusqlite::Result<ToneRow> {
    Ok(ToneRow {
        id: r.get(0)?,
        name: r.get(1)?,
        bytes: r.get(2)?,
        crc32: r.get(3)?,
        slot: r.get(4)?,
        saved_at: r.get(5)?,
    })
}
