//! IRs do usuário: o armazenamento do laboratório (issue #24).
//!
//! **O que é guardado aqui.** O `.ir` que o dono escolheu no disco — o
//! response de cabinets que ele usa ao vivo. Diferente do tom SnapTone (§5),
//! aqui não existe conversão: o `.ir` é o que o aparelho consome, byte a byte.
//!
//! **A regra do chunk é do WIRE e vale na importação.** §13.7: o pedaço final
//! é REJEITADO, não padado (strict, rev.2 do ADR-6) — o device conta os chunks
//! pelo payload, e um arquivo com 7 bytes de resto faz o contador divergir do
//! resto do arquivo (o sintoma é um IR que "grava" e depois soa como lixo).
//! Guardar um arquivo que não pode ser enviado só adia o erro para o momento
//! do upload, quando o dono já está no aparelho; por isso [`Library::import_ir`]
//! recusa na entrada, e o botão de enviar aparece desabilitado com o motivo.
//!
//! **Os 20 slots são do aparelho, não do app.** `<ppIRInfo0..19>` no `.prst`
//! são as tags que definem o slot pela POSIÇÃO (§13.12) — e o slot 0 é
//! válido, ao contrário do SnapTone que começa em 1. `SLOTS` reexporta o `20`
//! do core para biblioteca e FSM não divergirem.
//!
//! **O que NÃO está aqui: o que o aparelho já tem.** A tabela de User IRs do
//! device (`list_user_irs`, §13.12) é lida do fio e vale para a sessão
//! atual — o que o banco guarda é a biblioteca do DONO, não o estado do
//! aparelho. Misturar os dois faria a tela afirmar que o slot 3 tem o IR
//! "Vintage" porque está no arquivo, quando o aparelho tem outra coisa nele.

use serde::{Deserialize, Serialize};

use crate::{Library, LibraryError};

/// Quantidade de slots de User IR no GP-100 (`<ppIRInfo0..19>`, §13.12).
pub const SLOTS: u8 = gp100_core::session::IR_SLOTS;

/// Bytes de payload por chunk no fio (§13.7): 30 nibbles = 15 bytes reais.
///
/// Reexportado do core em vez de repetido: a validação da importação e a
/// checagem da FSM precisam concordar sobre o mesmo número, e dois literais
/// em crates diferentes são um `mod 15` que aceita um e outro.
pub const CHUNK_BYTES: usize = gp100_core::session::IR_CHUNK_BYTES;

/// CRC-32 IEEE — o mesmo dos tons (`snap_tone::crc32`).
///
/// Reexportado em vez de reimplementado: dois lugares calculando o id do
/// conteúdo com polinômios diferentes dariam dois ids para o mesmo arquivo, e
/// a lista do dono "sumiria" do nada ao trocar de caminho.
pub use crate::snap_tone::crc32;

/// Um IR da biblioteca, como a LISTA mostra (sem o blob).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrRow {
    /// Identificador estável, derivado do conteúdo (`i<crc32 hex>`).
    pub id: String,
    /// Nome que o dono deu (é o que a tela de User IR do aparelho mostra).
    pub name: String,
    /// Tamanho do `.ir` em bytes.
    pub bytes: i64,
    /// CRC-32 do `.ir` — a prova de integridade do conteúdo guardado.
    pub crc32: i64,
    /// Slot do device atribuído (0..=19), ou `None` se ainda não foi.
    pub slot: Option<u8>,
    /// ISO-8601 da importação.
    pub saved_at: String,
}

/// Um IR COM o blob — o que o upload envia.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ir {
    /// A linha da lista.
    #[serde(flatten)]
    pub row: IrRow,
    /// Os bytes do `.ir`.
    pub blob: Vec<u8>,
}

/// O quadro do laboratório: os IRs e o estado dos slots, numa leitura só.
///
/// **Por que `usados` sai daqui e não da UI.** A tela mostra os 20 slots e diz
/// quantos têm IR; se as duas coisas viessem de consultas separadas, o rodapé
/// poderia dizer "3 em uso" com a lista mostrando 2. Contando a partir das
/// MESMAS linhas que a lista devolve, as duas não podem divergir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrBoard {
    /// Os IRs, na ordem do laboratório (slot primeiro, depois nome).
    pub irs: Vec<IrRow>,
    /// Quantidade de slots do aparelho (§13.12).
    pub slots: u8,
    /// Quantos slots têm IR atribuído.
    pub usados: u8,
}

/// Colunas da listagem, na ordem do DTO. Uma constante para o SELECT e para o
/// `row`: dois lugares para lembrar a ordem é como um campo troca de nome no
/// caminho.
const COLUNAS: &str = "id, name, bytes, crc32, slot, saved_at";

impl Library {
    /// Importa um `.ir` como registro da biblioteca.
    ///
    /// **O id é derivado do conteúdo**, não de um contador: importar o mesmo
    /// arquivo duas vezes atualiza o mesmo registro em vez de criar um
    /// duplicado — que é o que o dono quer quando reorganiza a pasta. A
    /// colisão de CRC (dois arquivos diferentes com o mesmo CRC-32) não
    /// sobrescreve nada: o segundo recebe o sufixo `-2`.
    ///
    /// `saved_at` é ISO-8601 e vem de quem chama — o crate não tem relógio.
    ///
    /// # Erros
    /// [`LibraryError::Ir`] se o nome for vazio, o blob tiver 0 bytes ou
    /// não for múltiplo de [`CHUNK_BYTES`] (a regra strict do §13.7), e
    /// [`LibraryError::Sqlite`] se o banco falhar.
    pub fn import_ir(
        &self,
        name: &str,
        blob: &[u8],
        saved_at: &str,
    ) -> Result<IrRow, LibraryError> {
        if name.trim().is_empty() {
            return Err(LibraryError::Ir("nome vazio".into()));
        }
        if blob.is_empty() {
            return Err(LibraryError::Ir("IR de 0 bytes".into()));
        }
        if !blob.len().is_multiple_of(CHUNK_BYTES) {
            return Err(LibraryError::Ir(format!(
                "{} bytes: o envio exige multiplo de {CHUNK_BYTES} (o pedaco final e recusado, nao completado)",
                blob.len()
            )));
        }
        let crc = crc32(blob);
        let id = self.id_ir_para(crc, blob)?;
        // Transação: a linha e o blob entram juntos ou nenhum entra. Sem
        // transação, uma falha na segunda INSERT deixaria um IR listado cujo
        // conteúdo não existe — e o envio tentaria ler um id fantasma.
        let tx = self.conn().unchecked_transaction()?;
        tx.execute(
            "INSERT INTO ir_lib (id, name, bytes, crc32, slot, saved_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                bytes = excluded.bytes,
                crc32 = excluded.crc32,
                saved_at = excluded.saved_at",
            rusqlite::params![id, name.trim(), blob.len() as i64, crc as i64, saved_at],
        )?;
        tx.execute(
            "INSERT INTO ir_lib_blob (id, blob) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET blob = excluded.blob",
            rusqlite::params![id, blob],
        )?;
        tx.commit()?;
        self.ir(&id)?
            .ok_or_else(|| LibraryError::Ir("o IR importado sumiu do banco".into()))
    }

    /// O id de um conteúdo: o CRC em hexadecimal, com sufixo se outro arquivo
    /// já ocupou o nome com bytes diferentes.
    ///
    /// Devolve o id EXISTENTE quando o conteúdo é igual ao que já está lá —
    /// é o que faz a reimportação ser um update e não um duplicado.
    fn id_ir_para(&self, crc: u32, blob: &[u8]) -> Result<String, LibraryError> {
        let base = format!("i{crc:08x}");
        let mut id = base.clone();
        let mut n = 2u32;
        loop {
            match self.blob_ir(&id)? {
                None => return Ok(id),
                Some(guardado) if guardado == blob => return Ok(id),
                Some(_) => {
                    id = format!("{base}-{n}");
                    n += 1;
                }
            }
        }
    }

    /// Todos os IRs, na ordem que o laboratório mostra: os atribuídos
    /// primeiro, pelo número do slot, e os soltos pelo nome.
    pub fn irs(&self) -> Result<Vec<IrRow>, LibraryError> {
        let mut stmt = self.conn().prepare(&format!(
            "SELECT {COLUNAS} FROM ir_lib ORDER BY slot IS NULL, slot, name"
        ))?;
        let linhas = stmt.query_map([], linha_ir)?;
        let mut saida = Vec::new();
        for l in linhas {
            saida.push(l?);
        }
        Ok(saida)
    }

    /// Uma linha da lista pelo id. `None` = não existe.
    pub fn ir(&self, id: &str) -> Result<Option<IrRow>, LibraryError> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLUNAS} FROM ir_lib WHERE id = ?1"),
                [id],
                linha_ir,
            )
            .ok())
    }

    /// Os bytes do `.ir`. `None` = não existe.
    pub fn blob_ir(&self, id: &str) -> Result<Option<Vec<u8>>, LibraryError> {
        let mut stmt = self
            .conn()
            .prepare("SELECT blob FROM ir_lib_blob WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row.get(0)?))
    }

    /// O IR COM o blob, que é o que o envio pede.
    pub fn ir_com_blob(&self, id: &str) -> Result<Option<Ir>, LibraryError> {
        let Some(row) = self.ir(id)? else {
            return Ok(None);
        };
        let Some(blob) = self.blob_ir(id)? else {
            return Ok(None);
        };
        Ok(Some(Ir { row, blob }))
    }

    /// Renomeia. `false` = não existia.
    pub fn renomear_ir(&self, id: &str, nome: &str) -> Result<bool, LibraryError> {
        if nome.trim().is_empty() {
            return Err(LibraryError::Ir("nome vazio".into()));
        }
        let n = self.conn().execute(
            "UPDATE ir_lib SET name = ?2 WHERE id = ?1",
            rusqlite::params![id, nome.trim()],
        )?;
        Ok(n > 0)
    }

    /// Atribui (ou desliga, com `None`) o slot de um IR.
    ///
    /// **Um slot, um IR.** O índice único parcial em `slot` faz o banco recusar
    /// a segunda atribuição, e a checagem antes do `UPDATE` transforma isso em
    /// erro com o nome do dono: "o slot 3 é do 'Vintage 4x12'" é uma resposta;
    /// `UNIQUE constraint failed` é um stack trace.
    ///
    /// # Erros
    /// [`LibraryError::IrSlotInvalido`] fora de 0..=19,
    /// [`LibraryError::IrSlotOcupado`] quando outro IR já tem o slot, e
    /// [`LibraryError::Ir`] quando o IR não existe.
    pub fn atribuir_slot_ir(&self, id: &str, slot: Option<u8>) -> Result<(), LibraryError> {
        if let Some(s) = slot {
            if s >= SLOTS {
                // `SLOTS - 1` e não `SLOTS`: o erro diz a FAIXA que o dono
                // pode usar, e "esperado 0..=20" num aparelho de 20 slots
                // (0..=19) é a tela mentindo sobre o formato.
                return Err(LibraryError::IrSlotInvalido(s as u32, SLOTS - 1));
            }
            if let Some(dono) = self.dono_do_slot_ir(s)? {
                if dono != id {
                    let nome = self.ir(&dono)?.map_or(dono.clone(), |t| t.name);
                    return Err(LibraryError::IrSlotOcupado {
                        slot: s,
                        dono: nome,
                    });
                }
            }
        }
        let n = self.conn().execute(
            "UPDATE ir_lib SET slot = ?2 WHERE id = ?1",
            rusqlite::params![id, slot],
        )?;
        if n == 0 {
            return Err(LibraryError::Ir(format!("IR {id} inexistente")));
        }
        Ok(())
    }

    /// Quem ocupa um slot de IR, se alguém ocupar.
    pub fn dono_do_slot_ir(&self, slot: u8) -> Result<Option<String>, LibraryError> {
        Ok(self
            .conn()
            .query_row("SELECT id FROM ir_lib WHERE slot = ?1", [slot], |r| {
                r.get(0)
            })
            .ok())
    }

    /// O IR de um slot, se houver.
    pub fn ir_do_slot(&self, slot: u8) -> Result<Option<Ir>, LibraryError> {
        let Some(id) = self.dono_do_slot_ir(slot)? else {
            return Ok(None);
        };
        self.ir_com_blob(&id)
    }

    /// Apaga o IR e o blob junto. `false` = não existia.
    ///
    /// O `ON DELETE CASCADE` apaga o blob; deixar o órfão é o tipo de lixo que
    /// só aparece quando o dono tenta importar de novo e o banco cresce até
    /// encher — e IR é o conteúdo mais pesado que o app guarda (centenas de KB).
    pub fn apagar_ir(&self, id: &str) -> Result<bool, LibraryError> {
        Ok(self
            .conn()
            .execute("DELETE FROM ir_lib WHERE id = ?1", [id])?
            > 0)
    }

    /// Nº de IRs guardados (rodapé do laboratório).
    pub fn total_irs(&self) -> Result<i64, LibraryError> {
        Ok(self
            .conn()
            .query_row("SELECT COUNT(*) FROM ir_lib", [], |r| r.get(0))?)
    }

    /// O quadro inteiro do laboratório — os IRs e os slots, em UMA leitura.
    ///
    /// # Erros
    /// [`LibraryError::Sqlite`] se o banco falhar.
    pub fn ir_board(&self) -> Result<IrBoard, LibraryError> {
        let irs = self.irs()?;
        let usados = irs.iter().filter(|t| t.slot.is_some()).count() as u8;
        Ok(IrBoard {
            irs,
            slots: SLOTS,
            usados,
        })
    }
}

/// Uma linha da tabela `ir_lib` → [`IrRow`] (ordem = [`COLUNAS`]).
fn linha_ir(r: &rusqlite::Row<'_>) -> rusqlite::Result<IrRow> {
    Ok(IrRow {
        id: r.get(0)?,
        name: r.get(1)?,
        bytes: r.get(2)?,
        crc32: r.get(3)?,
        slot: r.get(4)?,
        saved_at: r.get(5)?,
    })
}
