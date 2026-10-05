//! param_range — a trava de CONTEÚDO que impede o assert do firmware (#110).
//!
//! **O problema que este módulo existe para resolver (medido em campo,
//! 05/10/2026, gate H2, GP-100 V2.1):** o `set-param` com valor inventado
//! derrubou o aparelho. O firmware V2.1 tem, em `Drivers/audio/audio.c:1828`,
//!
//! ```text
//! CODE:para <= GetParaMaxVal(
//! ```
//!
//! e ao assertar ele **para de responder a toda transação**, inclusive às
//! leituras puras. O device continua enumerado e `OK` no Windows — o que
//! elimina cabo, driver e porta — e a única recuperação é um power-cycle
//! físico. Pior: o `set-param` é fire-and-forget (§13.11, D4), então o fio
//! não devolve **nenhum** aviso. O operador descobre que quebrou o pedal
//! quando a leitura seguinte toma timeout.
//!
//! **A política (decisão do owner, 05/10/2026 — ver ADR-10):**
//!
//! | par `(code, ctrl)` | regra | o que o [`check`] faz |
//! |---|---|---|
//! | `knob` do dicionário | `min <= v <= max` | recusa fora da faixa |
//! | `switch`/`combox` | `v ∈ option_ids` | recusa id desconhecido |
//! | **sem regra** (código ou `ctrl` fora do dicionário) | — | **aceita** |
//! | `NaN` / `±inf` | — | recusa sempre |
//!
//! **POR QUE O DICIONÁRIO E NÃO SÓ AS AMOSTRAS.** A fonte da faixa é o
//! `algorithm.xml` oficial da Suite (V1.5.1), já versionado e validado em
//! `analysis/parameters.json` — 185 algoritmos, 639 controles, dos quais 575
//! `knob` com `[min,max]` e 64 `switch`/`combox` com `option_ids`. A
//! corroboração empírica está medida e é um gate
//! (`analysis/check_param_ranges.py`): das 92 amostras reais de knob em
//! `analysis/fixtures/knobs.jsonl`, **13 dos 14 pares observados caem
//! inteiros dentro da faixa declarada — zero contradições**. Duas fontes
//! independentes (o que a Suite declara e o que ela de fato manda ao
//! firmware) concordam, e a Suite nunca escreve fora do que o aparelho
//! aceita. Isso é a melhor prova disponível sem derrubar mais um pedal.
//!
//! **O 14º par é a exceção honesta:** `0x0a00002c` (U-ban 4x12, CAB) com
//! `ctrl = 1`. O aparelho **varreu esse knob de 0 a 99** em 8 amostras
//! reais, mas o dicionário só descreve o `ctrl = 0` (Volume) desse cab.
//! É um buraco do dicionário, não um valor perigoso — e recusá-lo quebraria
//! um knob que comprovadamente funciona. Por isso a política acima aceita
//! o que está sem regra. O relatório do gate lista o buraco.
//!
//! **A trava é de conteúdo, não de política.** Ela não sabe se o destino é
//! mock ou aparelho: vale igual nos dois builds (H1 continua seguro por
//! construção) e acontece em [`check`](check), que o
//! [`set_param_payload`](crate::codec::set_param_payload) chama **antes** de
//! existirem bytes para enviar. É diferente do ADR-5 (`write-verified`), que
//! é política de hardware e mora no transporte.
//!
//! **Custo zero em tempo de execução:** a tabela é derivada do dicionário
//! embutido uma única vez, sob um [`OnceLock`], no primeiro `set-param`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::{ControlKind, Dictionary, DICTIONARY_JSON};
use crate::ProtocolError;

/// Módulo da cadeia de cada slot do fio (§13.11: addr `10 [slot] 00 02`).
///
/// O `nibble` do `effectCode` **não** é o módulo — é um índice de família
/// interno, e a mesma família aparece em módulos diferentes (`0x05` = C-Wah
/// no PRE, `0x03` = Green OD no DST). O módulo vem do slot do endereço, e é
/// ele que resolve a ambiguidade do dicionário: `Boost` e `14 Boost` têm o
/// MESMO `effectCode` (`0x0000001a`) com defaults divergentes em PRE e DST
/// (ver `model::Algorithm`). Por isso a chave da tabela carrega o slot, e não
/// só o `code`.
const MODULE_OF_SLOT: [&str; 9] = ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"];

/// A regra de valor de um controle, derivada do dicionário.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueRule {
    /// Knob contínuo: `min <= valor <= max` (normalizado por
    /// [`Control::range`](crate::model::Control::range), porque knobs
    /// bidirecionais vêm com `min > max` — `Pitch.L-Pitch` é 0..-24).
    Range {
        /// Teto inferior físico.
        lo: f64,
        /// Teto superior físico.
        hi: f64,
    },
    /// Chave/combo: o valor de fio é um dos `option_ids` declarados.
    Discrete {
        /// Ids válidos (o mesmo `Vec` do dicionário).
        ids: Vec<u8>,
    },
}

/// Chave da tabela: `(slot 1..=9, effectCode, ctrl)`.
///
/// O slot faz parte da chave pelos dois motivos de [`MODULE_OF_SLOT`] — o
/// módulo é o slot, e sem ele `Boost`/`14 Boost` colidiriam.
type Key = (u8, u32, u8);

/// Tabela derivada do dicionário embutido, montada uma vez.
fn table() -> &'static HashMap<Key, ValueRule> {
    static TABLE: OnceLock<HashMap<Key, ValueRule>> = OnceLock::new();
    TABLE.get_or_init(|| {
        // `DICTIONARY_JSON` é validado por `from_json` e embutido no binário
        // (`include_str!`, R1) — se ele estivesse corrompido, o
        // `expect` panicaria no PRIMEIRO set-param em vez de deixar passar
        // byte errado. O dicionário também tem um gate próprio
        // (`analysis/validate_parameters.py`) que roda antes de qualquer
        // build.
        let dict = Dictionary::from_json(DICTIONARY_JSON)
            .expect("dicionario embutido invalido (gate: analysis/validate_parameters.py)");
        let mut map = HashMap::with_capacity(700);
        for (i, module) in MODULE_OF_SLOT.iter().enumerate() {
            let slot = (i + 1) as u8;
            for alg in dict.algorithms() {
                if alg.module != *module {
                    continue;
                }
                for ctrl in &alg.controls {
                    let rule = match ctrl.kind {
                        // Knob sem faixa utilizável no dicionário fica sem
                        // regra — e "sem regra" é o mesmo que estar fora do
                        // dicionário: aceita (política do ADR-10).
                        ControlKind::Knob => {
                            ctrl.range().map(|(lo, hi)| ValueRule::Range { lo, hi })
                        }
                        ControlKind::Switch | ControlKind::Combox => (!ctrl.option_ids.is_empty())
                            .then(|| ValueRule::Discrete {
                                ids: ctrl.option_ids.clone(),
                            }),
                    };
                    if let Some(rule) = rule {
                        map.insert((slot, alg.code, ctrl.pos), rule);
                    }
                }
            }
        }
        map
    })
}

/// A regra que o dicionário declara para o par, se houver.
///
/// `None` = **sem regra**: código desconhecido, `ctrl` além do último
/// controle do algoritmo, ou knob sem faixa utilizável. Nessa situação a
/// política do ADR-10 é **aceitar** — recusar quebraria o knob do CAB
/// `0x0a00002c`/`ctrl 1`, que o aparelho aceita de 0 a 99 (8 amostras reais).
pub fn rule_for(slot: u8, code: u32, ctrl: u8) -> Option<&'static ValueRule> {
    table().get(&(slot, code, ctrl))
}

/// Nº de pares `(slot, code, ctrl)` com regra — para o relatório e os testes.
pub fn ruled_pairs() -> usize {
    table().len()
}

/// Valida o valor de um `set-param` **antes** de o frame existir.
///
/// Recusa [`ProtocolError::ValueOutOfRange`] quando o valor está fora da
/// regra do dicionário, ou quando não é um número. O endereço e o teto
/// entram na mensagem porque o runbook de campo precisa dizer *qual* knob e
/// *até onde* — sem isso o operador vê um erro e não sabe o que corrigir.
///
/// `NaN` e `±inf` são recusados **sempre**, com ou sem regra: o bit-pattern
/// de `f32` para eles é lixo que nenhum firmware aceitaria, e mandá-lo é
/// indistinguível de-corruption do ponto de vista do assert.
pub fn check(slot: u8, code: u32, ctrl: u8, value: f32) -> Result<(), ProtocolError> {
    let addr = format!("10{slot:02x}0002");
    if !value.is_finite() {
        return Err(ProtocolError::ValueOutOfRange {
            addr,
            param: format!("0x{code:08x}/{ctrl}"),
            got: format!("{value}"),
            allowed: "valor finito (NaN/inf não são parâmetros físicos)".into(),
        });
    }
    let Some(rule) = rule_for(slot, code, ctrl) else {
        // Sem regra: aceita (decisão do owner no ADR-10). O caso conhecido
        // é o knob do CAB que o dicionário não descreve.
        return Ok(());
    };
    let v = value as f64;
    let param = format!("0x{code:08x}/{ctrl}");
    match rule {
        ValueRule::Range { lo, hi } => {
            let allowed = format!("faixa {lo} .. {hi}");
            if v < *lo || v > *hi {
                return Err(ProtocolError::ValueOutOfRange {
                    addr,
                    param,
                    got: format!("{value}"),
                    allowed,
                });
            }
        }
        ValueRule::Discrete { ids } => {
            let allowed = format!("id discreto em {ids:?}");
            let id = (value.trunc() as f64).clamp(i32::MIN as f64, i32::MAX as f64);
            let whole = (v - id).abs() < f64::EPSILON;
            let ok = whole && ids.iter().any(|i| *i as f64 == id);
            if !ok {
                return Err(ProtocolError::ValueOutOfRange {
                    addr,
                    param,
                    got: format!("{value}"),
                    allowed,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O vetor do H2 que derrubou o aparelho: `Bog RedM` Gain, faixa 0..99 no
    /// dicionário, `99.5` acima do teto. **Este é o teste que a #110 pede.**
    #[test]
    fn o_valor_que_derrubou_o_aparelho_e_recusado() {
        let e = check(3, 0x0700_006e, 0, 99.5).expect_err("99.5 acima do teto");
        let msg = e.to_string();
        assert!(
            msg.contains("0x0700006e/0"),
            "a mensagem nomeia o knob: {msg}"
        );
        assert!(
            msg.contains("faixa 0 .. 99"),
            "a mensagem diz o teto: {msg}"
        );
        assert!(msg.contains("10030002"), "a mensagem diz o endereço: {msg}");
    }

    /// O valor real da mesma captura (15.0) continua passando — a trava não
    /// pode barrar o que a Suite de fato manda.
    #[test]
    fn o_valor_real_da_captura_passa() {
        check(3, 0x0700_006e, 0, 15.0).expect("15.0 = vetor real do knobs.jsonl");
        check(3, 0x0700_006e, 0, 99.0).expect("99.0 = topo real da captura");
    }

    /// Faixa bidirecional: o dicionário declara `Pitch.L-Pitch` como 0..-24
    /// (min > max) e `Control::range` normaliza. O teste travaria se a
    /// normalização sumisse.
    #[test]
    fn faixa_bidirecional_e_normalizada() {
        // A-Chorus/Rate é [0.1, 10.0]; 10.5 é o mesmo tipo de erro do 99.5.
        check(7, 0x0400_0000, 1, 10.0).expect("teto real do A-Chorus Rate");
        check(7, 0x0400_0000, 1, 0.1).expect("piso real do A-Chorus Rate");
        assert!(check(7, 0x0400_0000, 1, 10.5).is_err(), "acima do teto");
        assert!(check(7, 0x0400_0000, 1, 0.05).is_err(), "abaixo do piso");
    }

    /// EQ com faixa negativa: as amostras reais vão a -49 e o dicionário
    /// declara -50..50. O valor observado passa; -60 não.
    #[test]
    fn faixa_com_piso_negativo() {
        check(6, 0x0100_003c, 3, -49.0).expect("piso observado (2.2kHz)");
        check(6, 0x0100_003c, 3, 50.0).expect("teto declarado");
        assert!(check(6, 0x0100_003c, 3, -50.5).is_err(), "abaixo do piso");
    }

    /// Switch/combo: o valor de fio é um `option_id` exato, não um float
    /// qualquer dentro de uma faixa. `Boost/Bright` é `[1, 0]` no PRE.
    #[test]
    fn chave_discreta_so_aceita_id_declarado() {
        check(1, 0x0000_001a, 1, 1.0).expect("Bright On");
        check(1, 0x0000_001a, 1, 0.0).expect("Bright Off");
        let e = check(1, 0x0000_001a, 1, 0.5).expect_err("0.5 nao e id");
        assert!(e.to_string().contains("id discreto"), "{e}");
    }

    /// Par SEM regra: o knob do CAB `0x0a00002c`/`ctrl 1`, que o aparelho
    /// varreu de 0 a 99 e o dicionário não descreve. A política aceita —
    /// recusar quebraria um knob que funciona.
    #[test]
    fn par_sem_regra_e_aceito() {
        assert!(
            rule_for(5, 0x0a00_002c, 1).is_none(),
            "sem regra no dicionario"
        );
        check(5, 0x0a00_002c, 1, 0.0).expect("0.0 = 1a amostra real");
        check(5, 0x0a00_002c, 1, 99.0).expect("99.0 = ultima amostra real");
    }

    /// Código totalmente desconhecido (não está no dicionário): aceito, pela
    /// mesma política. Um dia o dicionário ganha a entrada e aí passa a
    /// valer — é o comportamento que o relatório do gate mede.
    #[test]
    fn codigo_desconhecido_e_aceito() {
        assert!(check(1, 0x0300_0001, 0, 42.0).is_ok());
    }

    /// `NaN` e `inf` recusados SEMPRE, mesmo sem regra: o bit-pattern vai
    /// para o firmware e nenhum assert accepta aquilo.
    #[test]
    fn nao_finito_e_recusado_sempre() {
        for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let e = check(5, 0x0a00_002c, 1, v).expect_err("nao-finito");
            assert!(e.to_string().contains("finito"), "{v}: {e}");
            let e = check(3, 0x0700_006e, 0, v).expect_err("nao-finito com regra");
            assert!(e.to_string().contains("finito"), "{v}: {e}");
        }
    }

    /// O slot faz parte da chave: `Boost` e `14 Boost` têm o MESMO effectCode
    /// em PRE e DST. Se a chave fosse só `(code, ctrl)`, uma regra do PRE
    /// contaminaria o DST.
    #[test]
    fn o_slot_faz_parte_da_chave() {
        // Gain de Boost em PRE (slot 1) e em DST (slot 2) são entradas
        // distintas do dicionário — mesmo code, mesmo ctrl.
        let pre = rule_for(1, 0x0000_001a, 0);
        let dst = rule_for(2, 0x0000_001a, 0);
        assert!(
            pre.is_some() || dst.is_some(),
            "pelo menos um modulo tem regra"
        );
    }

    /// A tabela cobre a maior parte dos controles: 636 pares com regra
    /// (573 knob + 63 discretos). O número é do gate
    /// `analysis/check_param_ranges.py` — se cair, o dicionário encolheu e
    /// alguém precisa olhar.
    #[test]
    fn a_tabela_cobre_os_pares_do_dicionario() {
        assert!(
            ruled_pairs() >= 600,
            "cobertura caiu para {} pares — dicionario encolheu?",
            ruled_pairs()
        );
    }
}
