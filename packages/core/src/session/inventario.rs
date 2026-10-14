//! o ESPAÇO de presets do aparelho (banco/slot, §13.4) — o que ele tem, não o que
//! se supõe.
//!
//! Fonte medida: captura S1 do Suite (#132/ADR-12). `pp_e_valido` é o predicado
//! estático do MESMO espaço que `inventario_do_aparelho` enumera na ordem do
//! scan — `tests/pp_gate.rs` prende as duas no fixture da S1.

/// O espaço de presets no fio é BANCO/SLOT (§13.4): 198 pps em dois bancos
/// de 99 — NÃO é um intervalo linear. As capturas (S1–S4, 796 selects)
/// contêm exatamente `0000..=0062` e `0100..=0162`; os pps `0x0063..0x00C5`
/// NÃO existem, e o firmware V2.1 morre no assert `PresetNum < TOTAL_PA`
/// (`audio.c:912`) ao recebê-los — o scan do boot os mandava por suposição
/// linear nunca conferida em campo (issue #148, campo 07/10).
pub fn pp_e_valido(pp: u16) -> bool {
    let banco = pp >> 8;
    let slot = pp & 0x00FF;
    slot <= 0x0062 && (banco == 0x00 || banco == 0x01)
}

/// Os 198 `pp` que o **aparelho** tem, na ordem em que o Suite varre na
/// captura S1 (`packages/core/data/fixtures/boot.jsonl` — os mesmos 2299 OUTs que
/// `tests/replay_fixtures.rs::replay_boot_byte_a_byte` reproduz).
///
/// **De onde o número vem (issue #132 — nada aqui é inventado).** A captura
/// do próprio Suite escaneando o pedal manda, em `11/13010000`:
///
/// ```text
/// 0100 0101 … 0162  0000 0001 … 0062
/// ```
///
/// → **99 pps no banco `0x01xx` + 99 no banco `0x00xx`**, os 99 slots de
/// `all.prst` (ppID `"0".."98"`) nos DOIS bancos, com o byte alto do banco
/// no byte mais significativo. A sonda de campo de 06/10 corroborou o
/// espaço (`dump-preset 0x0100` respondeu) e o `H1` já tinha lido
/// `0x0000`/`0x0031`/`0x0062` no aparelho.
///
/// **Por que este é o default do APARELHO e não o `0..198` de sempre.**
/// `0..197` manda 99 selects que o aparelho NÃO tem (`0x0063..0x00c5` — o
/// `PresetNum < TOTAL_PA` do assert de `audio.c:912` é literalmente sobre
/// isto) e NUNCA alcança o banco `0x01xx`, onde o pedal liga. O mock fica
/// com o `0..198` de sempre (o espaço dele é o do documento, e os testes
/// varrem pps arbitrários).
///
/// A ordem é a da captura (banco 1 primeiro) porque a ordem TAMBÉM é
/// evidência: com este inventário o `boot()` de um aparelho recém-ligado
/// reproduz a sequência do Suite.
///
/// **É a ÚNICA fonte do espaço do aparelho.** O CONJUNTO (99 no banco
/// `0x01xx` + 99 no `0x00xx`) e a ORDEM vivem aqui — não existe uma segunda
/// lista do mesmo espaço, com outra ordem, para divergir desta. A ordem
/// canônica do banco/slot (`0x00xx` primeiro) não tem consumidor: quem
/// varre é a captura. `pp_e_valido` declara o MESMO espaço como predicado
/// estático, e `pp_gate.rs` prende as duas no fixture da S1.
pub fn inventario_do_aparelho() -> Vec<u16> {
    let mut pps: Vec<u16> = (0x0100u16..=0x0162).collect();
    pps.extend(0x0000u16..=0x0062);
    pps
}

/// O inventário escrito como faixas legíveis para o runbook de campo
/// (`0x0000..0x0062, 0x0100..0x0162`) — a mensagem da recusa precisa dizer
/// não só O QUE foi recusado, mas até onde o aparelho vai.
pub(super) fn faixa_legivel(pps: &[u16]) -> String {
    let mut ordenados = pps.to_vec();
    ordenados.sort_unstable();
    ordenados.dedup();
    let mut faixas: Vec<String> = Vec::new();
    let mut inicio = match ordenados.first() {
        Some(&p) => p,
        None => return "(inventário vazio)".into(),
    };
    let mut anterior = inicio;
    for &p in ordenados.iter().skip(1) {
        if p == anterior + 1 {
            anterior = p;
            continue;
        }
        faixas.push(descreve_faixa(inicio, anterior));
        inicio = p;
        anterior = p;
    }
    faixas.push(descreve_faixa(inicio, anterior));
    faixas.join(", ")
}

fn descreve_faixa(inicio: u16, fim: u16) -> String {
    if inicio == fim {
        format!("{inicio:#06x}")
    } else {
        format!("{inicio:#06x}..{fim:#06x}")
    }
}
