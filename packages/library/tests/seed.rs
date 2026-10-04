//! Seed de fábrica contra o `all.prst` REAL (issue #26).
//!
//! Este teste usa o arquivo de verdade (`files/patches/all.prst`), não um
//! `.prst` sintético. A diferença importa: um seed testado contra um arquivo
//! que o próprio teste construiu passa e continua errado no `all.prst` que o
//! dono tem.

use gp100_library::{Bank, Library, SearchQuery};

mod comum;
use comum::arquivo_temporario;

fn all_prst() -> Vec<u8> {
    // `CARGO_MANIFEST_DIR` = packages/library; dois níveis sobem até a raiz.
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("raiz do repo");
    let p = raiz.join("files").join("patches").join("all.prst");
    std::fs::read(&p).unwrap_or_else(|e| panic!("le {}: {e}", p.display()))
}

#[test]
fn o_seed_entra_com_os_99_presets_de_fabrica() {
    let lib = Library::open_in_memory().unwrap();
    let n = lib
        .seed_factory(&all_prst(), "2026-01-01T00:00:00Z")
        .unwrap();
    assert_eq!(n, gp100_library::seed::TOTAL_DE_FABRICA);
    assert!(lib.tem_fabrica().unwrap());

    let fab = lib.factory_presets().unwrap();
    assert_eq!(fab.len(), 99);
    assert_eq!(fab[0].pp, Some(0));
    assert_eq!(fab[0].name, "It's GP100");
    assert_eq!(fab[0].pp_type_name, "Rock");
}

#[test]
fn o_seed_e_idempotente_rodar_de_novo_nao_duplica() {
    let caminho = arquivo_temporario("seed-idempotente");
    let bytes = all_prst();
    {
        let lib = Library::open(caminho.path()).unwrap();
        assert_eq!(
            lib.seed_factory(&bytes, "2026-01-01T00:00:00Z").unwrap(),
            99
        );
        assert_eq!(lib.count().unwrap(), 99);
        // Segundo boot: o app semeia de novo se algo mandar.
        assert_eq!(
            lib.seed_factory(&bytes, "2026-01-01T00:00:00Z").unwrap(),
            99
        );
        assert_eq!(lib.count().unwrap(), 99, "99, não 198");
    }
}

#[test]
fn o_seed_sobrescreve_o_nome_mas_nao_apaga_patch_de_usuario() {
    let caminho = arquivo_temporario("seed-nao-apaga");
    let bytes = all_prst();
    let lib = Library::open(caminho.path()).unwrap();
    lib.seed_factory(&bytes, "2026-01-01T00:00:00Z").unwrap();
    lib.upsert(&comum::patch_de_usuario("u1", "MEU PATCH", "{}"))
        .unwrap();

    lib.seed_factory(&bytes, "2026-02-02T00:00:00Z").unwrap();

    assert_eq!(lib.count().unwrap(), 100);
    assert!(lib.get("u1").unwrap().is_some(), "o patch do dono continua");
}

#[test]
fn os_99_nomes_batem_com_o_artefato_do_front() {
    // O front tem `presetData.ts` GERADO do mesmo all.prst. Se o seed e o
    // artefato divergirem, a biblioteca passa a ter 99 presets e a UI 99
    // presets DIFERENTES — e ninguém descobre até o usuário abrir "P25" e ver
    // outro som. Este é o teste que amarra as duas pontas.
    let artefato = std::fs::read_to_string(artefato_preset_data()).expect("le presetData.ts");
    let lib = Library::open_in_memory().unwrap();
    lib.seed_factory(&all_prst(), "2026-01-01T00:00:00Z")
        .unwrap();

    let mut esperado = Vec::new();
    for linha in artefato.lines() {
        const CHAVE: &str = "name: \"";
        let Some(i) = linha.find(CHAVE) else { continue };
        let nome = linha[i + CHAVE.len()..].split('"').next().unwrap_or("");
        esperado.push(nome.to_string());
    }
    assert_eq!(esperado.len(), 99, "o artefato tem 99 linhas de preset");

    let do_banco: Vec<String> = lib
        .factory_presets()
        .unwrap()
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(do_banco, esperado, "seed e artefato discordam do nome");
}

fn artefato_preset_data() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = packages/library → dois níveis sobem até a raiz.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("raiz do repo")
        .join("packages")
        .join("app")
        .join("ui")
        .join("src")
        .join("artifacts")
        .join("presetData.ts")
}

#[test]
fn a_busca_encontra_preset_de_fabrica_por_tipo() {
    // Fecha o ciclo: o que a #26 pede ("busca por estilo") sobre dado REAL.
    let lib = Library::open_in_memory().unwrap();
    lib.seed_factory(&all_prst(), "2026-01-01T00:00:00Z")
        .unwrap();
    let rock = lib
        .search(&SearchQuery {
            pp_type: Some(4),
            bank: Some(Bank::Factory),
            ..SearchQuery::default()
        })
        .unwrap();
    assert!(!rock.is_empty(), "tem Rock no all.prst");
    assert!(rock.iter().all(|r| r.pp_type_name == "Rock"));
    assert!(
        rock.iter()
            .all(|r| r.name.to_lowercase().contains("gp") || r.pp_type == 4),
        "filtro por tipo não é ilusão"
    );
}
