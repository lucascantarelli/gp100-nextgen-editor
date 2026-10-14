//! o espaço de `pp`: o DECIMAL do documento (`ppID`) e o banco/slot do FIO.

/// O `pp` de um atributo `ppID` — **DECIMAL**: a base ÚNICA do espaço de
/// `pp` (#132/ADR-12).
///
/// **Por que decimal, e não hex.** Os 99 `ppID` do `all.prst` são as
/// strings `"0".."98"`, e o aparelho numera exatamente assim: a captura S1
/// varre `0x0000..=0x0062` (99 valores contíguos = `0..98` em hex no fio) —
/// hex no `ppID` daria uma lista ESPARSA (`0x0a` não existe, `"98"` →
/// `0x98` = 152, fora de qualquer banco do aparelho). Quem mais lê este
/// atributo em decimal é a semente da biblioteca (`gp100_library::seed`)
/// e o gerador do artefato do front (gerador da era RE; dissolução na
/// #168, que
/// ASSERTA `0..98`): esta função é onde core, biblioteca e front passam a
/// concordar (#132, critério de aceite).
pub fn pp_id_decimal(pp_id: &str) -> Option<u16> {
    pp_id.trim().parse().ok()
}

/// Converte um `pp` do **fio** no índice do **documento**.
///
/// O fio carrega o banco no byte alto: a captura S1 varre `0x0000..=0x0062`
/// E `0x0100..=0x0162` — os mesmos 99 slots nos dois bancos, com o conteúdo
/// idêntico índice a índice (97/99 iguais byte a byte na captura; os 2
/// diferentes são slots editados). O `all.prst` não tem banco, então o
/// índice é o byte baixo: `0x0100` → `0`.
///
/// `pp` com byte alto fora de `0x00`/`0x01` passa intacto — é um número
/// que nenhum aparelho prova ter, e o lookup tem de FALHAR em vez de
/// resolver outro preset por engano (um `pp & 0xFF` cego faria `0x0200`
/// abrir o preset 0).
pub fn indice_do_documento(pp: u16) -> u16 {
    if pp > 0x01FF {
        pp
    } else {
        pp & 0xFF
    }
}
