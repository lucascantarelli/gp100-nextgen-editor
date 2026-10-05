//! Historico de versao por patch — a biblioteca versionada (issue #113, M3-1).
//!
//! **O que muda com este modulo.** No app oficial, mexer num preset e perder o
//! anterior e o caminho normal. Aqui, cada gravacao de um patch de usuario
//! acrescenta um snapshot IMUTAVEL a `preset_version`, e o registo de `preset`
//! passa a ser so "a versao N" — um atalho para o ultimo snapshot, nao a unica
//! copia. Quem tem o historico e a tabela append-only.
//!
//! **As tres regras, e onde cada uma e garantida.**
//!
//! 1. *Toda gravacao versiona.* Garantida por construcao: `Preset` de usuario
//!    so entra no banco por [`crate::Library::upsert`], que grava o registo e o
//!    snapshot na MESMA transacao. Nao existe caminho que escreva o corrente sem
//!    criar a versao — se existisse, a historia teria um buraco silencioso.
//! 2. *Uma versao nunca e reescrita.* Garantida pelo trigger
//!    `preset_version_sem_reescrita` da migration 6: um `UPDATE` vira erro do
//!    SQLite. O teste `uma_versao_nao_pode_ser_reescrita` prova pelo banco, nao
//!    pela API do crate.
//! 3. *Restaurar cria versao nova.* Restaurar e uma gravacao como qualquer
//!    outra: le o snapshot antigo (ou o tweak de um knob dele) e ACRESCENTA. Um
//!    "desfazer" que apaga o presente e um "desfazer" que mente — aqui ele nao
//!    existe, nem por API nem por SQL.
//!
//! **So patch de usuario versiona.** Preset de fabrica vem do `all.prst`
//! embutido e e imutavel por construcao: criar 99 historicos de um byte cada
//! seria ruido que a tela teria de esconder.

use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};

use crate::{diff, Bank, Library, LibraryError, Preset};

/// Uma versao COMPLETA: o snapshot com a cadeia serializada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    /// Chave da linha (`preset_version.id`).
    ///
    /// E a identidade que a UI usa para pedir diff e restauracao — e ela e
    /// `i64` porque quem a gera e o AUTOINCREMENT do SQLite. O `seq` nao serve
    /// para isso: ele so e unico DENTRO de um patch.
    pub id: i64,
    /// O patch a que a versao pertence (`preset.id`).
    pub preset_id: String,
    /// Numero da versao no patch, 1-based e monotonico.
    pub seq: u32,
    /// ISO-8601 do instante da gravacao (o mesmo do registo corrente).
    pub saved_at: String,
    /// Nome do patch NAQUELE instante — renomear tambem versiona.
    pub name: String,
    /// Cadeia serializada (`BoardSlot[]`, o mesmo formato do `payload`).
    pub payload: Option<String>,
}

/// Uma linha do historico, sem a cadeia.
///
/// Existe pelo mesmo motivo do [`crate::PresetRow`]: o painel de historico
/// mostra dezenas de linhas e nao precisa pagar o JSON de uma cadeia em cada
/// uma para desenhar um numero e uma data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRow {
    /// Chave da linha.
    pub id: i64,
    /// Patch dono.
    pub preset_id: String,
    /// Numero da versao (1-based).
    pub seq: u32,
    /// ISO-8601 da gravacao.
    pub saved_at: String,
    /// Nome do patch naquele instante.
    pub name: String,
    /// Esta versao e a que o registo corrente espelha (o maior `seq`)?
    pub current: bool,
}

/// SQL de gravacao do registo corrente, compartilhado por `upsert` e pelo
/// import: um lugar so, para os dois nao divergirem no dia em que o formato
/// ganhar um campo.
fn grava_registro(conn: &Connection, p: &Preset) -> Result<(), LibraryError> {
    conn.execute(
        "INSERT INTO preset (id, bank, pp, name, pp_type, pp_type_name, saved_at, payload)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
            bank = excluded.bank,
            pp = excluded.pp,
            name = excluded.name,
            pp_type = excluded.pp_type,
            pp_type_name = excluded.pp_type_name,
            saved_at = excluded.saved_at,
            payload = excluded.payload",
        rusqlite::params![
            p.id,
            p.bank.as_str(),
            p.pp,
            p.name,
            p.pp_type,
            p.pp_type_name,
            p.saved_at,
            p.payload,
        ],
    )?;
    Ok(())
}

/// Acrescenta o snapshot de um patch de usuario. Nao faz nada para fabrica.
///
/// `seq` sai de `MAX(seq)+1` na propria tabela, dentro da transacao de quem
/// chama: calcular o proximo numero em memoria e gravar depois seria uma corrida
/// entre o boot e um clique que a UNIQUE só descobriria no erro.
fn append_versao(conn: &Connection, p: &Preset) -> Result<Option<Version>, LibraryError> {
    if p.bank != Bank::User {
        return Ok(None);
    }
    let seq: i64 = conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM preset_version WHERE preset_id = ?1",
        [&p.id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO preset_version (preset_id, seq, saved_at, name, payload)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![p.id, seq, p.saved_at, p.name, p.payload],
    )?;
    Ok(Some(Version {
        id: conn.last_insert_rowid(),
        preset_id: p.id.clone(),
        seq: seq as u32,
        saved_at: p.saved_at.clone(),
        name: p.name.clone(),
        payload: p.payload.clone(),
    }))
}

impl Library {
    /// Grava o registo corrente E o snapshot, na mesma transacao.
    ///
    /// E o unico ponto por onde um patch de usuario entra no banco — e e por
    /// isso que "nao existe gravacao sem versao" e uma propriedade estrutural,
    /// nao uma disciplina. Se a insercao da versao falhar, o registo corrente
    /// volta atras junto: os dois nunca divergem.
    pub(crate) fn upsert_versionado(&self, p: &Preset) -> Result<(), LibraryError> {
        if p.bank != Bank::User {
            return grava_registro(&self.conn, p);
        }
        let tx = self.conn.unchecked_transaction()?;
        grava_registro(&tx, p)?;
        append_versao(&tx, p)?;
        tx.commit()?;
        Ok(())
    }

    /// A mesma gravacao, para quem ja tem uma transacao aberta (o import).
    pub(crate) fn grava_na_transacao(tx: &Transaction<'_>, p: &Preset) -> Result<(), LibraryError> {
        grava_registro(tx, p)?;
        append_versao(tx, p)?;
        Ok(())
    }

    /// O historico de um patch, do mais RECENTE para o mais antigo.
    ///
    /// A ordem e a da tela: quem abre o historico quer ver o agora primeiro. O
    /// desempate por `seq` existe porque `saved_at` tem resolucao de segundo —
    /// salvar 3x no mesmo segundo daria uma ordem que muda a cada leitura.
    pub fn versoes(&self, preset_id: &str) -> Result<Vec<VersionRow>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, preset_id, seq, saved_at, name,
                    seq = (SELECT MAX(seq) FROM preset_version WHERE preset_id = ?1)
             FROM preset_version WHERE preset_id = ?1
             ORDER BY seq DESC",
        )?;
        let mut rows = stmt.query([preset_id])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(VersionRow {
                id: r.get(0)?,
                preset_id: r.get(1)?,
                seq: r.get(2)?,
                saved_at: r.get(3)?,
                name: r.get(4)?,
                current: r.get::<_, i64>(5)? != 0,
            });
        }
        Ok(out)
    }

    /// Le uma versao completa (com a cadeia).
    pub fn versao(&self, id: i64) -> Result<Option<Version>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, preset_id, seq, saved_at, name, payload
             FROM preset_version WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        let Some(r) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(Version {
            id: r.get(0)?,
            preset_id: r.get(1)?,
            seq: r.get(2)?,
            saved_at: r.get(3)?,
            name: r.get(4)?,
            payload: r.get(5)?,
        }))
    }

    /// Restauracao TOTAL: o patch volta a ser o que a versao `id` era.
    ///
    /// Restaurar nao e um `UPDATE` no historico — e uma gravacao NOVA. A versao
    /// restaurada continua onde estava (o presente nao some), e o patch ganha
    /// uma versao `N+1` com o conteudo antigo. O `saved_at` da versao nova e o
    /// `agora` de quem chama: a versao 1 de hoje e a versao 4 de amanha, com a
    /// mesma cadeia e datas diferentes — que e a verdade.
    ///
    /// # Erros
    /// `None` se a versao nao existe; erro do banco se a gravacao falhar.
    pub fn restaura_versao(&self, id: i64, agora: &str) -> Result<Option<Version>, LibraryError> {
        let Some(v) = self.versao(id)? else {
            return Ok(None);
        };
        let p = Preset {
            id: v.preset_id.clone(),
            bank: Bank::User,
            pp: None,
            name: v.name.clone(),
            pp_type: 0,
            pp_type_name: String::new(),
            saved_at: agora.to_string(),
            payload: v.payload.clone(),
        };
        let tx = self.conn.unchecked_transaction()?;
        // O `pp_type`/`pp_type_name` do patch corrente NAO vem da versao: eles
        // descrevem o estilo de origem do patch e nao mudam com a cadeia. Se
        // viessem, restaurar apagaria o rotulo ("Rock") e a lista mostraria um
        // patch sem estilo.
        let (pp_type, pp_type_name): (i64, String) = tx
            .query_row(
                "SELECT pp_type, pp_type_name FROM preset WHERE id = ?1",
                [&v.preset_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0, String::new()));
        let p = Preset {
            pp_type: pp_type as u16,
            pp_type_name,
            ..p
        };
        grava_registro(&tx, &p)?;
        let nova = append_versao(&tx, &p)?.expect("patch de usuario versiona");
        tx.commit()?;
        Ok(Some(nova))
    }

    /// Restauracao PONTUAL: so o knob `(slot, pos)` volta ao valor da versao
    /// `id`; o resto do patch fica como esta.
    ///
    /// Tambem cria uma versao NOVA. E a unica implementacao de "restaurar um
    /// knob" — o diff diz o que mudou e ela diz "volte so isso".
    ///
    /// # Erros
    /// `None` se a versao ou o patch nao existem; erro se a cadeia corrente nao
    /// for legivel como slots.
    pub fn restaura_knob(
        &self,
        id: i64,
        slot: u32,
        pos: u32,
        agora: &str,
    ) -> Result<Option<Version>, LibraryError> {
        let Some(v) = self.versao(id)? else {
            return Ok(None);
        };
        let atual = self.get(&v.preset_id)?;
        let Some(atual) = atual else {
            return Ok(None);
        };
        let Some(corrente) = atual.payload.as_deref() else {
            return Err(LibraryError::SnapshotInvalido(
                "o patch corrente nao tem cadeia gravada".into(),
            ));
        };
        // O valor vem da versao ANTIGA (e o ponto do restaurar), e o alvo e a
        // cadeia CORRENTE — nao a antiga, senao restaurar um knob traria junto
        // todas as outras mudancas e viraria uma restauracao total disfarcada.
        let valor = diff::valor_do_knob(v.payload.as_deref().unwrap_or("[]"), slot, pos)?;
        let novo = diff::define_valor_do_knob(corrente, slot, pos, valor.as_deref())?;

        let p = Preset {
            payload: Some(novo),
            saved_at: agora.to_string(),
            ..atual
        };
        let tx = self.conn.unchecked_transaction()?;
        grava_registro(&tx, &p)?;
        let nova = append_versao(&tx, &p)?.expect("patch de usuario versiona");
        tx.commit()?;
        Ok(Some(nova))
    }

    /// Quantas versoes um patch tem (o rodape do painel de historico).
    pub fn total_versoes(&self, preset_id: &str) -> Result<i64, LibraryError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM preset_version WHERE preset_id = ?1",
            [preset_id],
            |r| r.get(0),
        )?)
    }
}
