//! Diff no nivel do knob — issue #113 (M3-1).
//!
//! A cadeia sintetica vem do MESMO helper que o resto dos testes (`slot_json`),
//! e ele grava os campos que `userPatches.ts` grava de verdade. O diff e testado
//! contra o formato que o app produz — nao contra um JSON inventado, que
//! passaria aqui e falharia no palco.

mod comum;
use comum::{cadeia_json, slot_json};
use gp100_library::diff_cadeias;

#[test]
fn cadeia_igual_nao_tem_diff() {
    let c = cadeia_json(&[slot_json(0, "Green OD", true, &[(0, "Gain", Some("40"))])]);
    let d = diff_cadeias(&c, &c).expect("diff");
    assert!(d.identical, "o mesmo texto nao tem diff");
    assert!(d.slots.is_empty());
    assert!(d.slots_added.is_empty() && d.slots_removed.is_empty());
}

#[test]
fn o_diff_lista_so_o_knob_que_mudou() {
    let antes = cadeia_json(&[slot_json(
        0,
        "Green OD",
        true,
        &[(0, "Gain", Some("40")), (1, "Tone", Some("50"))],
    )]);
    let depois = cadeia_json(&[slot_json(
        0,
        "Green OD",
        true,
        &[(0, "Gain", Some("70")), (1, "Tone", Some("50"))],
    )]);

    let d = diff_cadeias(&antes, &depois).expect("diff");
    assert!(!d.identical);
    assert_eq!(d.slots.len(), 1, "um slot mudou");
    let s = &d.slots[0];
    assert_eq!(s.slot, 0);
    assert!(!s.algorithm_changed, "o algoritmo e o mesmo");
    assert_eq!(
        s.knobs.len(),
        1,
        "so o Gain mudou — o Tone igual nao entra: {:?}",
        s.knobs
    );
    assert_eq!(s.knobs[0].knob, "Gain");
    assert_eq!(s.knobs[0].from.as_deref(), Some("40"));
    assert_eq!(s.knobs[0].to.as_deref(), Some("70"));
}

#[test]
fn trocar_o_algoritmo_nao_vira_diff_de_knobs() {
    // Trocar o AMP troca TODOS os knobs dele. Sem a bandeira, o diff diria
    // "voce girou 2 knobs" quando o dono trocou um pedal inteiro.
    let antes = cadeia_json(&[slot_json(
        2,
        "Bog RedM",
        true,
        &[(0, "Gain", Some("40")), (1, "Treble", Some("50"))],
    )]);
    let depois = cadeia_json(&[slot_json(
        2,
        "Plexi",
        true,
        &[(0, "Gain", Some("10")), (1, "Treble", Some("90"))],
    )]);

    let d = diff_cadeias(&antes, &depois).expect("diff");
    let s = &d.slots[0];
    assert!(s.algorithm_changed);
    assert_eq!(s.name_before, "Bog RedM");
    assert_eq!(s.name_after, "Plexi");
    assert_eq!(
        s.knobs.len(),
        2,
        "os knobs tambem mudaram e continuam listados"
    );
}

#[test]
fn desligar_um_slot_aparece_no_diff() {
    let antes = cadeia_json(&[slot_json(1, "Phaser", true, &[])]);
    let depois = cadeia_json(&[slot_json(1, "Phaser", false, &[])]);

    let d = diff_cadeias(&antes, &depois).expect("diff");
    assert_eq!(d.slots.len(), 1, "o on/off e uma mudanca do slot");
    assert!(d.slots[0].on_before);
    assert!(!d.slots[0].on_after);
    assert!(d.slots[0].knobs.is_empty());
    assert!(!d.slots[0].algorithm_changed);
}

#[test]
fn o_diff_vem_na_ordem_da_cadeia() {
    let antes = cadeia_json(&[
        slot_json(0, "A", true, &[(0, "Gain", Some("10"))]),
        slot_json(4, "B", true, &[(0, "Gain", Some("10"))]),
    ]);
    let depois = cadeia_json(&[
        slot_json(0, "A", true, &[(0, "Gain", Some("20"))]),
        slot_json(4, "B", true, &[(0, "Gain", Some("30"))]),
    ]);

    let d = diff_cadeias(&antes, &depois).expect("diff");
    assert_eq!(
        d.slots.iter().map(|s| s.slot).collect::<Vec<_>>(),
        vec![0, 4],
        "os slots saem na ordem do sinal, nao na ordem do array"
    );
}

#[test]
fn um_slot_que_so_existe_num_lado_nao_vira_mudanca_de_knob() {
    let antes = cadeia_json(&[slot_json(0, "A", true, &[(0, "Gain", Some("10"))])]);
    let depois = cadeia_json(&[
        slot_json(0, "A", true, &[(0, "Gain", Some("10"))]),
        slot_json(8, "Hall", true, &[(0, "Mix", Some("30"))]),
    ]);

    let d = diff_cadeias(&antes, &depois).expect("diff");
    assert_eq!(d.slots_added, vec![8]);
    assert!(d.slots_removed.is_empty());
    assert!(d.slots.is_empty(), "o slot novo nao e mudanca de knob");
    assert!(!d.identical, "mas o diff NAO e vazio");
}

#[test]
fn knob_sem_valor_num_lado_conta_como_mudanca() {
    // `value` e opcional no palco: ausente e uma coisa, "40" e outra. Tratar os
    // dois como string vazia faria o diff mentir sobre o que a restauracao
    // desfaz.
    let antes = cadeia_json(&[slot_json(0, "A", true, &[(0, "Gain", None)])]);
    let depois = cadeia_json(&[slot_json(0, "A", true, &[(0, "Gain", Some("40"))])]);

    let d = diff_cadeias(&antes, &depois).expect("diff");
    assert_eq!(d.slots.len(), 1);
    assert_eq!(d.slots[0].knobs[0].from, None);
    assert_eq!(d.slots[0].knobs[0].to.as_deref(), Some("40"));
}

#[test]
fn cadeia_que_nao_e_lista_de_slots_e_erro_tipado() {
    let err = diff_cadeias("{\"nao\":\"e uma lista\"}", "[]").expect_err("recusa");
    assert!(err.to_string().contains("cadeia invalida"), "{err}");
    // E o payload que nem e JSON tem o erro de JSON, que e outra mensagem.
    assert!(diff_cadeias("{", "[]").is_err());
}
