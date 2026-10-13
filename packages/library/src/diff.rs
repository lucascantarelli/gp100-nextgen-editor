//! Diff no nivel do knob entre duas versoes de um patch (issue #113, M3-1).
//!
//! **Por que no nivel do knob e nao no arquivo.** O `.prst` e um container
//! binario: um `cmp` entre duas versoes responde "mudou" sem responder "o que".
//! O dono quer ouvir "o Gain do AMP foi de 40 para 55", e e isso que este
//! modulo produz — agrupado por slot, na ordem da cadeia, so o que mudou.
//!
//! **De onde sai a estrutura.** Da cadeia SERIALIZADA que o palco ja grava no
//! `payload` do patch (`BoardSlot[]` do front, `JSON.stringify(slots)` em
//! `userPatches.ts`). Este modulo NAO define esse formato — ele le os campos que
//! o diff precisa e ignora o resto, por `serde_json::Value` e nao por um struct
//! rigido. A razao e a direcao da dependencia: o `gp100-library` nao conhece
//! React, entao nao pode importar o tipo do front; e um struct rigido aqui
//! viraria um SEGUNDO contrato do mesmo JSON, que diverge em silencio na
//! primeira vez que o palco ganhar um campo.
//!
//! **O que este modulo nao faz.** Nao normaliza valor (nao converte "50" em
//! 50.0), nao julga se a mudanca e audivel e nao toca no aparelho. Ele compara
//! o que esta gravado e devolve o que mudou — a decisao de restaurar e do dono.

use serde::{Deserialize, Serialize};

use crate::LibraryError;

/// Uma mudanca de knob: `pos` do knob dentro do slot, o nome, e o antes/depois.
///
/// `from`/`to` sao `Option` porque o `value` do knob e opcional no palco: um
/// knob sem valor lido nao e o mesmo que um knob com valor vazio, e tratar os
/// dois como `""` faria o diff acusar mudanca onde nao houve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnobDiff {
    /// Posicao do knob dentro do slot (identidade estavel — o nome pode mudar
    /// quando o algoritmo troca, o `pos` nao).
    pub pos: u32,
    /// Nome do knob COMO ELE E NA VERSAO NOVA (e o que a tela mostra).
    pub knob: String,
    /// Valor na versao antiga (`None` = o knob nao existia/nao tinha valor).
    pub from: Option<String>,
    /// Valor na versao nova.
    pub to: Option<String>,
}

/// O que mudou num slot da cadeia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotDiff {
    /// Numero do slot (0..8, ordem do sinal).
    pub slot: u32,
    /// Nome do algoritmo antes.
    pub name_before: String,
    /// Nome do algoritmo depois.
    pub name_after: String,
    /// O slot estava ligado antes?
    pub on_before: bool,
    /// O slot esta ligado depois?
    pub on_after: bool,
    /// O ALGORITMO do slot trocou (`name_before != name_after`).
    ///
    /// Existe separado dos knobs de proposito: trocar o AMP troca TODOS os
    /// knobs dele, e sem esta bandeira o diff pareceria "voce girou 12 knobs"
    /// quando o dono trocou um pedal. Os dois fatos sao diferentes.
    pub algorithm_changed: bool,
    /// Os knobs que mudaram, na ordem do `pos`.
    pub knobs: Vec<KnobDiff>,
}

/// O diff inteiro entre duas versoes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainDiff {
    /// Slots com alguma mudanca, na ordem do numero do slot.
    pub slots: Vec<SlotDiff>,
    /// Slots que existem so na versao nova.
    pub slots_added: Vec<u32>,
    /// Slots que existem so na versao antiga.
    pub slots_removed: Vec<u32>,
    /// Nada mudou. A tela usa isto para dizer "identico" em vez de uma lista
    /// vazia — que e ambigua entre "igual" e "nao consegui ler".
    pub identical: bool,
}

/// Um knob reduzido ao que o diff compara.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Knob {
    pos: u32,
    name: String,
    value: Option<String>,
}

/// Um slot reduzido ao que o diff compara.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Slot {
    slot: u32,
    name: String,
    on: bool,
    knobs: Vec<Knob>,
}

/// Le a cadeia serializada de um `payload`.
///
/// O formato e o do palco: um array de slots, cada um com `slot`, `name`,
/// `state` e `knobs`; cada knob com `pos`, `name` e `value` (opcional). Campo
/// ausente tem um default DELIBERADO — e nao um erro: uma cadeia de um app
/// anterior sem `state` nao deve impedir o dono de ver o que mudou nos knobs.
/// O que e erro e o payload nao ser um array de objetos de slot.
fn slots_do_payload(payload: &str) -> Result<Vec<Slot>, LibraryError> {
    let valor: serde_json::Value = serde_json::from_str(payload)?;
    let Some(itens) = valor.as_array() else {
        return Err(LibraryError::SnapshotInvalido(
            "a cadeia nao e um array de slots".into(),
        ));
    };

    let mut slots = Vec::with_capacity(itens.len());
    for (idx, item) in itens.iter().enumerate() {
        let slot = item
            .get("slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(idx as u64) as u32;
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // Ausente = ligado. O palco so grava `state: false` para um slot
        // desligado; um payload sem o campo veio de um app anterior e nao pode
        // fazer o diff dizer que o dono desligou TODOS os pedais.
        let on = item.get("state").and_then(|v| v.as_bool()).unwrap_or(true);

        let mut knobs = Vec::new();
        if let Some(arr) = item.get("knobs").and_then(|v| v.as_array()) {
            for (kidx, k) in arr.iter().enumerate() {
                let pos = k.get("pos").and_then(|v| v.as_u64()).unwrap_or(kidx as u64) as u32;
                let kn = k
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = k
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                knobs.push(Knob {
                    pos,
                    name: kn,
                    value,
                });
            }
        }
        slots.push(Slot {
            slot,
            name,
            on,
            knobs,
        });
    }
    Ok(slots)
}

/// O diff entre duas cadeias serializadas (versao antiga, versao nova).
///
/// Puro: nao toca banco nem device. Existe solto porque o diff e o que a tela
/// mais chama (a cada par de versoes que o dono clica) e ele nao deve custar uma
/// transacao por clique.
///
/// # Erros
/// [`LibraryError::SnapshotInvalido`] se um dos payloads nao for uma cadeia de
/// slots; [`LibraryError::Json`] se nem for JSON.
pub fn diff_cadeias(antes: &str, depois: &str) -> Result<ChainDiff, LibraryError> {
    let a = slots_do_payload(antes)?;
    let b = slots_do_payload(depois)?;

    let mut slots = Vec::new();
    let mut slots_added = Vec::new();
    let mut slots_removed = Vec::new();

    for antigo in &a {
        let Some(novo) = b.iter().find(|s| s.slot == antigo.slot) else {
            slots_removed.push(antigo.slot);
            continue;
        };
        let knobs = knobs_que_mudaram(&antigo.knobs, &novo.knobs);
        let algorithm_changed = antigo.name != novo.name;
        if algorithm_changed || antigo.on != novo.on || !knobs.is_empty() {
            slots.push(SlotDiff {
                slot: antigo.slot,
                name_before: antigo.name.clone(),
                name_after: novo.name.clone(),
                on_before: antigo.on,
                on_after: novo.on,
                algorithm_changed,
                knobs,
            });
        }
    }
    for novo in &b {
        if !a.iter().any(|s| s.slot == novo.slot) {
            slots_added.push(novo.slot);
        }
    }

    // A ordem da cadeia (0..8), nao a ordem em que os arrays vieram: a tela
    // desenha os slots sempre na mesma ordem, e um diff fora de ordem faria a
    // lista "pular" quando o mesmo par fosse comparado ao contrario.
    slots.sort_by_key(|s| s.slot);
    slots_added.sort_unstable();
    slots_removed.sort_unstable();

    let identical = slots.is_empty() && slots_added.is_empty() && slots_removed.is_empty();
    Ok(ChainDiff {
        slots,
        slots_added,
        slots_removed,
        identical,
    })
}

/// O valor do knob `(slot, pos)` numa cadeia serializada.
///
/// Existe para a restauracao PONTUAL: e daqui que sai o valor que volta. Um
/// slot ou um knob que nao existem naquela versao sao ERRO, e nao `None`: o
/// `None` significa "o knob existia e nao tinha valor", e confundir os dois
/// faria a restauracao limpar um knob que so nao era daquele algoritmo.
///
/// # Erros
/// [`LibraryError::SnapshotInvalido`] se a cadeia nao tem o slot ou o knob.
pub fn valor_do_knob(payload: &str, slot: u32, pos: u32) -> Result<Option<String>, LibraryError> {
    let slots = slots_do_payload(payload)?;
    let Some(s) = slots.iter().find(|s| s.slot == slot) else {
        return Err(LibraryError::SnapshotInvalido(format!(
            "a versao nao tem o slot {slot}"
        )));
    };
    let Some(k) = s.knobs.iter().find(|k| k.pos == pos) else {
        return Err(LibraryError::SnapshotInvalido(format!(
            "o slot {slot} nao tem knob na posicao {pos}"
        )));
    };
    Ok(k.value.clone())
}

/// Reescreve o valor do knob `(slot, pos)` na cadeia, preservando TODO o resto.
///
/// Trabalha sobre `serde_json::Value` e nao sobre um struct proprio: a cadeia
/// tem campos que o diff nao le (arquétipo, variante, opcoes do switch) e um
/// round-trip por struct os perderia — restaurar um knob viraria "apagar tudo o
/// que o `gp_library` ainda nao conhece". `value: null` e o valor ausente sao a
/// mesma coisa para [`valor_do_knob`], entao o round-trip fecha.
///
/// # Erros
/// [`LibraryError::SnapshotInvalido`] se a cadeia nao tem o slot ou o knob;
/// [`LibraryError::Json`] se o payload nem for JSON.
pub fn define_valor_do_knob(
    payload: &str,
    slot: u32,
    pos: u32,
    valor: Option<&str>,
) -> Result<String, LibraryError> {
    let mut doc: serde_json::Value = serde_json::from_str(payload)?;
    let Some(itens) = doc.as_array_mut() else {
        return Err(LibraryError::SnapshotInvalido(
            "a cadeia nao e um array de slots".into(),
        ));
    };
    for (idx, item) in itens.iter_mut().enumerate() {
        let s = item
            .get("slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(idx as u64) as u32;
        if s != slot {
            continue;
        }
        let Some(knobs) = item.get_mut("knobs").and_then(|v| v.as_array_mut()) else {
            return Err(LibraryError::SnapshotInvalido(format!(
                "o slot {slot} nao tem lista de knobs"
            )));
        };
        for (kidx, k) in knobs.iter_mut().enumerate() {
            let kp = k.get("pos").and_then(|v| v.as_u64()).unwrap_or(kidx as u64) as u32;
            if kp != pos {
                continue;
            }
            let Some(obj) = k.as_object_mut() else {
                return Err(LibraryError::SnapshotInvalido(format!(
                    "o knob {pos} do slot {slot} nao e um objeto"
                )));
            };
            let novo = match valor {
                Some(v) => serde_json::Value::String(v.to_string()),
                None => serde_json::Value::Null,
            };
            obj.insert("value".to_string(), novo);
            return Ok(serde_json::to_string(&doc)?);
        }
        return Err(LibraryError::SnapshotInvalido(format!(
            "o slot {slot} nao tem knob na posicao {pos}"
        )));
    }
    Err(LibraryError::SnapshotInvalido(format!(
        "a cadeia nao tem o slot {slot}"
    )))
}

/// Os knobs cujo valor mudou, comparando por `pos`.
///
/// Knob que existe so na versao antiga conta como mudanca com `to: None` (o
/// algoritmo novo nao tem aquele knob) — e nao como "nada": a diferenca existe
/// e esconder isso faria o diff mentir sobre o que a restauracao vai desfazer.
fn knobs_que_mudaram(antes: &[Knob], depois: &[Knob]) -> Vec<KnobDiff> {
    let mut out = Vec::new();
    for a in antes {
        match depois.iter().find(|d| d.pos == a.pos) {
            Some(d) if d.value != a.value => out.push(KnobDiff {
                pos: d.pos,
                knob: d.name.clone(),
                from: a.value.clone(),
                to: d.value.clone(),
            }),
            Some(_) => {}
            None => out.push(KnobDiff {
                pos: a.pos,
                knob: a.name.clone(),
                from: a.value.clone(),
                to: None,
            }),
        }
    }
    for d in depois {
        if !antes.iter().any(|a| a.pos == d.pos) {
            out.push(KnobDiff {
                pos: d.pos,
                knob: d.name.clone(),
                from: None,
                to: d.value.clone(),
            });
        }
    }
    out.sort_by_key(|k| k.pos);
    out
}
