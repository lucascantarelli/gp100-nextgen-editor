//! O `ppID` é o índice **DECIMAL** do arquivo (0-based) — o mesmo espaço do
//! fio (banco/slot, [`pp_e_valido`]).
//!
//! #156: interpretá-lo como HEX (`from_str_radix(s, 16)`) só coincidia em
//! `'0'..'9'`; a partir de `"10"` o app endereçava um pp que o fio nem tem.
//! Consequências medidas antes da correção:
//!
//! * **36 dos 99** presets tinham um pp recusado por `select_preset`
//!   (`"esperado pp no espaço banco/slot: 0000..=0062 ou 0100..=0162"`);
//! * o `current_pp` do mock nascia errado (`0x98` em vez de `98`);
//! * a UI — que já usava decimal nos seus artefatos (`presetData.ts`,
//!   `pp: 10 = "Fat Plexi"`) — recebia um `pp` diferente do backend.
//!
//! Este teste trava os três lados: o espaço do fio, o inventário do boot
//! (`session::inventario_do_aparelho`) e o caso que o hex destruía.

use gp100_core::pedalboard::{embedded_document, preset_list};
use gp100_core::session::{inventario_do_aparelho, pp_e_valido};

/// Todo `pp` que a UI enxerga existe no fio e está no inventário do scan.
#[test]
fn preset_list_vive_no_espaco_do_fio() {
    let doc = embedded_document().expect("all.prst embutido");
    let lista = preset_list(&doc);
    assert_eq!(lista.len(), 99, "99 presets de fábrica");

    let inventario = inventario_do_aparelho();
    for e in &lista {
        assert!(
            pp_e_valido(e.pp),
            "pp {:#06x} ({}) fora do espaço banco/slot — a UI mostraria um \
             preset que o fio recusa",
            e.pp,
            e.name
        );
        assert!(
            inventario.contains(&e.pp),
            "pp {:#06x} ({}) ausente do inventário do boot",
            e.pp,
            e.name
        );
    }
}

/// O caso que o hex destruía: `ppID="10"` é o índice 10, não `0x10`.
///
/// O hex ainda passaria no teste acima (`0x10` é válido no fio!) mas
/// apontaria para o preset ERRADO — por isso a prova é pelo nome.
#[test]
fn pp_id_decimal_na_caspa_do_10() {
    let doc = embedded_document().expect("all.prst embutido");
    let lista = preset_list(&doc);
    let p10 = lista
        .iter()
        .find(|e| e.pp == 10)
        .expect("o índice 10 existe no arquivo");
    assert_eq!(
        p10.name, "Fat Plexi",
        "ppID \"10\" é o índice 10; com hex sairia 0x10 e casaria com o \
         preset errado"
    );
    assert!(
        !lista.iter().any(|e| e.pp > 98),
        "nenhum pp acima do último índice (98)"
    );
}
