//! Testes de CONTRATO do `.prst` — ROADMAP M0.2, regra sagrada R4:
//! abrir→salvar sem editar deve reproduzir os arquivos BYTE-A-BYTE.
//!
//! Ângulo: caixa-preta (só API `pub`), consumindo os 3 arquivos reais de
//! `files/patches/` — o mesmo material que o editor vai mexer.

use std::path::PathBuf;

use gp100_core::preset::{escape_value, Document};

/// Os 3 arquivos reais (all.prst = 99 presets; os outros, 1 cada).
fn patch_files() -> Vec<PathBuf> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop(); // raiz do repo (gp100-core/ -> .)
    root.push("files");
    root.push("patches");
    ["all.prst", "Blink OD.prst", "its gp100.prst"]
        .into_iter()
        .map(|n| root.join(n))
        .collect()
}

/// R4 NUCLEAR: parse → to_bytes == arquivo original, byte a byte.
#[test]
fn roundtrip_is_byte_identical() {
    for path in patch_files() {
        let original = std::fs::read(&path).expect("arquivo existe");
        let doc = Document::parse(&original).expect("dialecto válido");
        assert_eq!(
            doc.to_bytes(),
            original,
            "round-trip divergiu em {}",
            path.display()
        );
    }
}

/// Double-parse: serializar e reparsear é estável (idempotência do writer).
#[test]
fn double_parse_is_stable() {
    for path in patch_files() {
        let original = std::fs::read(&path).expect("arquivo existe");
        let doc = Document::parse(&original).expect("válido");
        let once = doc.to_bytes();
        let doc2 = Document::parse(&once).expect("serialização re-parseável");
        assert_eq!(
            doc2.to_bytes(),
            once,
            "writer não é idempotente: {}",
            path.display()
        );
    }
}

/// Contagens do inventário (all.prst: 99 presets; 9 Effect por preset).
#[test]
fn inventory_counts() {
    let bytes = std::fs::read(patch_files()[0].clone()).expect("all.prst");
    let doc = Document::parse(&bytes).expect("válido");
    let presets: Vec<_> = doc.presets().collect();
    assert_eq!(presets.len(), 99, "99 presets no all.prst");
    assert_eq!(doc.preset_info().unwrap().attr("count"), Some("99"));
    // todo preset tem os 9 efeitos da cadeia (x = 0..8). ORDEM OBSERVADA:
    // x DESCENDE no arquivo (RVB x=8 primeiro, PRE x=0 último) — o parser
    // preserva a ordem original; aqui só provamos o conjunto.
    for p in &presets {
        let mut xs: Vec<u32> = p
            .effects()
            .map(|e| e.element().attr_u32("x").unwrap())
            .collect();
        xs.sort_unstable();
        assert_eq!(
            xs,
            (0..9).collect::<Vec<_>>(),
            "cadeia x=0..8 em '{}'",
            p.pp_name().unwrap_or("?")
        );
    }
}

/// Vocabulário §13.9 em um preset real (Blink OD): pp* e params_0..14.
#[test]
fn preset_view_vocabulary() {
    let bytes = std::fs::read(patch_files()[1].clone()).expect("Blink OD.prst");
    let doc = Document::parse(&bytes).expect("válido");
    let p = doc.presets().next().expect("1 preset");
    assert_eq!(p.pp_name(), Some("Blink OD"));
    assert_eq!(p.pp_id(), Some("1"));
    assert_eq!(p.pp_type(), Some("6"));
    assert_eq!(p.pp_type_name(), Some("Pop"));

    let rvb = p.effect("RVB").expect("efeito RVB");
    assert_eq!(rvb.name(), Some("N-Star"));
    assert_eq!(rvb.code(), Some(201326598)); // nibble 0x0C, index 6
    assert_eq!(rvb.state(), Some("1"));
    // params_0..14 existem e são acessíveis (incl. valores grandes)
    for n in 0..14u8 {
        assert!(rvb.param(n).is_some(), "params_{n} ausente");
    }
    assert_eq!(rvb.param(4), Some("26478"));

    // preservação de atributos NÃO modelados (o contrato do M0.2):
    let ppctrl = doc
        .root()
        .child("presets")
        .unwrap()
        .child("ppCtrl")
        .unwrap();
    assert_eq!(ppctrl.attr("c21"), Some("65535"));
    let exp = doc
        .root()
        .child("presets")
        .unwrap()
        .child("ppEXP1")
        .unwrap();
    assert_eq!(
        exp.child("ppEXP1_0").unwrap().attr("expCode"),
        Some("524295")
    );
}

/// Entidades: `Dub&amp;Vibe` sobrevive intacta (forma crua preservada) e o
/// round-trip continua idêntico — o escape só é aplicado em valores NOVOS.
#[test]
fn entity_is_preserved_verbatim() {
    let bytes = std::fs::read(patch_files()[0].clone()).expect("all.prst");
    let doc = Document::parse(&bytes).expect("válido");
    let dub = doc
        .presets()
        .find(|p| p.pp_name().unwrap_or("").contains("amp;"))
        .expect("preset 'Dub&amp;Vibe' presente");
    assert_eq!(dub.pp_name(), Some("Dub&amp;Vibe"));
    // escape_value é o caminho para valores NOVOS (idempotente p/ &amp;)
    assert_eq!(escape_value("Dub&Vibe"), "Dub&amp;Vibe");
    assert_eq!(escape_value("Dub&amp;Vibe"), "Dub&amp;amp;Vibe");
}

/// Edição estável (o caso de uso real do editor): mudar UM valor e salvar
/// preserva TODO o resto byte a byte — só os dígitos editados mudam.
#[test]
fn edited_preset_diffs_only_in_edited_attr() {
    let path = patch_files()[1].clone(); // Blink OD (pequeno)
    let original = std::fs::read(&path).expect("arquivo");

    // localiza ppVolume="55" no ORIGINAL (sem depender de cwd)
    let needle = b"ppVolume=\"55\"";
    let off = original
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("ppVolume=\"55\" presente");
    let first_digit = off + "ppVolume=\"".len();

    let mut doc = Document::parse(&original).expect("válido");
    doc.root_mut()
        .child_mut("presets")
        .expect("bloco presets")
        .set_attr("ppVolume", "40")
        .expect("attr existente");

    let edited = doc.to_bytes();
    assert_ne!(edited, original, "a edição deve aparecer");
    assert_eq!(edited.len(), original.len(), "mesmo tamanho (55 -> 40)");
    // EXATAMENTE os 2 bytes dos dígitos mudam (nada mais — R4 preservado)
    let diffs: Vec<usize> = edited
        .iter()
        .zip(original.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(diffs, vec![first_digit, first_digit + 1]);

    // e o re-parse da versão editada mostra o valor novo
    let doc2 = Document::parse(&edited).expect("editado re-parseável");
    assert_eq!(
        doc2.root().child("presets").unwrap().attr("ppVolume"),
        Some("40")
    );
}

/// set_attr NÃO cria atributo novo (mudaria o layout — R4) e rejeita valor
/// cru não escapado.
#[test]
fn set_attr_is_strict() {
    let bytes = std::fs::read(patch_files()[1].clone()).expect("arquivo");
    let mut doc = Document::parse(&bytes).expect("válido");
    let presets = doc.root_mut().child_mut("presets").expect("presets");

    assert!(presets.set_attr("attr_inexistente", "1").is_err());
    assert!(presets.set_attr("ppName", "tem \" aspa").is_err());
    assert!(presets.set_attr("ppName", "cru & solto").is_err());
    // forma escapada é aceita:
    assert!(presets.set_attr("ppName", "Dub&amp;Vibe 2").is_ok());
}

/// Strict do dialecto: rejeita lixo fora do formato (não tenta "consertar").
#[test]
fn rejects_out_of_dialect() {
    assert!(Document::parse(b"<other/>").is_err());
    assert!(Document::parse(b"nope").is_err());
    assert!(Document::parse(b"\xef\xbb\xbf<?xml version=\"1.0\"?><GP-100></GP-100>").is_err());
    assert!(Document::parse(b"<?xml version=\"1.0\"?><GP-100><a></b></GP-100>").is_err());
    // comentário é fora do dialecto (não observado nos arquivos reais)
    assert!(Document::parse(b"<?xml version=\"1.0\"?><GP-100><!-- x --></GP-100>").is_err());
    // texto livre entre tags também
    assert!(Document::parse(b"<?xml version=\"1.0\"?><GP-100> oi </GP-100>").is_err());
}
