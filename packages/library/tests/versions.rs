//! Historico de versao por patch — issue #113 (M3-1).
//!
//! O que estes testes provam, e por que cada um existe:
//!
//! - **A gravacao versiona.** Nao existe caminho que escreva o patch corrente
//!   sem criar um snapshot — se existisse, a historia teria um buraco que o
//!   dono so descobre quando precisa voltar.
//! - **A versao nao pode ser reescrita.** Provado pelo BANCO (o trigger da
//!   migration 6), nao pela API do crate: o teste tenta o `UPDATE` cru.
//! - **Restaurar cria versao nova.** Nem total nem pontual apagam o presente.
//! - **Migrar de uma versao antiga nao perde o patch.** O backfill da v6 da a
//!   todo patch que ja existia a sua versao 1.

use gp100_library::Library;

mod comum;
use comum::{arquivo_temporario, cadeia_json, patch_de_usuario, slot_json};

/// Uma cadeia de 2 slots com um knob cada — o minimo que tem diff.
fn cadeia(gain: &str, treble: &str) -> String {
    cadeia_json(&[
        slot_json(0, "Green OD", true, &[(0, "Gain", Some(gain))]),
        slot_json(2, "Bog RedM", true, &[(1, "Treble", Some(treble))]),
    ])
}

#[test]
fn gravar_um_patch_cria_a_versao_1() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "Meu clean", &cadeia("40", "50")))
        .expect("grava");

    let v = lib.versoes("u1").expect("le historico");
    assert_eq!(v.len(), 1, "a primeira gravacao cria UMA versao");
    assert_eq!(v[0].seq, 1);
    assert!(v[0].current, "a unica versao e a corrente");
    assert_eq!(v[0].name, "Meu clean");
    assert_eq!(lib.total_versoes("u1").expect("conta"), 1);
}

#[test]
fn salvar_tres_vezes_cresce_o_historico_e_o_corrente_e_o_ultimo() {
    let lib = Library::open_in_memory().expect("abre");
    for (i, gain) in ["40", "55", "70"].iter().enumerate() {
        lib.upsert(&patch_de_usuario(
            "u1",
            &format!("Take {i}"),
            &cadeia(gain, "50"),
        ))
        .expect("grava");
    }

    let v = lib.versoes("u1").expect("le historico");
    assert_eq!(v.len(), 3, "tres gravacoes, tres versoes");
    // Do mais NOVO para o mais antigo: e a ordem que a tela mostra.
    assert_eq!(v.iter().map(|r| r.seq).collect::<Vec<_>>(), vec![3, 2, 1]);
    assert_eq!(
        v.iter().filter(|r| r.current).count(),
        1,
        "exatamente um corrente"
    );
    assert!(v[0].current, "o corrente e o de MAIOR seq");

    // E o registo corrente espelha a ultima gravacao, nao a primeira.
    let atual = lib.get("u1").expect("le").expect("existe");
    assert_eq!(atual.name, "Take 2");
    assert!(atual.payload.unwrap().contains("\"70\""));
}

#[test]
fn uma_versao_nao_pode_ser_reescrita() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "Meu clean", &cadeia("40", "50")))
        .expect("grava");
    let id = lib.versoes("u1").expect("historico")[0].id;

    // O UPDATE vai CRU no banco: se a imutabilidade dependesse da API do crate,
    // uma migration futura distraida reescreveria a historia sem aviso.
    let err = lib
        .conn()
        .execute(
            "UPDATE preset_version SET name = 'outro' WHERE id = ?1",
            [id],
        )
        .expect_err("o banco deve recusar");
    assert!(
        err.to_string().contains("imutavel"),
        "a recusa e a do trigger: {err}"
    );

    // E o dado continua intacto.
    let v = lib.versao(id).expect("le").expect("existe");
    assert_eq!(v.name, "Meu clean");
}

#[test]
fn preset_de_fabrica_nao_versiona() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&comum::preset_de_fabrica(0, "Its GP100", 4, "Rock"))
        .expect("grava");
    assert_eq!(
        lib.total_versoes("f0").expect("conta"),
        0,
        "fabrica vem do all.prst e nao tem historico"
    );
}

#[test]
fn o_historico_nao_mistura_patches() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "A", &cadeia("40", "50")))
        .expect("grava");
    lib.upsert(&patch_de_usuario("u2", "B", &cadeia("10", "20")))
        .expect("grava");
    lib.upsert(&patch_de_usuario("u1", "A2", &cadeia("45", "50")))
        .expect("grava");

    assert_eq!(lib.total_versoes("u1").expect("conta"), 2);
    assert_eq!(lib.total_versoes("u2").expect("conta"), 1);
    assert_eq!(
        lib.versoes("u2").expect("historico")[0].seq,
        1,
        "o seq conta por patch"
    );
}

#[test]
fn migrar_de_v5_da_a_versao_1_a_todo_patch_que_ja_existia() {
    let caminho = arquivo_temporario("migra-v6");
    let payload = cadeia("40", "50");

    // O estado de um usuario que atualizou: o banco parou na v5, ANTES da
    // tabela de versoes existir.
    {
        let conn = rusqlite::Connection::open(caminho.path()).expect("abre cru");
        for v in 1..=5 {
            conn.execute_batch(gp100_library::migrations::sql_da(v).expect("sql"))
                .expect("aplica etapa");
        }
        conn.pragma_update(None, "user_version", 5u32)
            .expect("marca v5");
        conn.execute(
            "INSERT INTO preset (id,bank,pp,name,pp_type,pp_type_name,saved_at,payload)
             VALUES ('u9','user',NULL,'Meu patch',4,'Rock','2026-03-03T00:00:00Z',?1)",
            [&payload],
        )
        .expect("patch do usuario antigo");
        conn.execute(
            "INSERT INTO preset (id,bank,pp,name,pp_type,pp_type_name,saved_at,payload)
             VALUES ('f0','factory',0,'Its GP100',4,'Rock','2026-01-01T00:00:00Z',NULL)",
            [],
        )
        .expect("preset de fabrica");
    }

    let lib = Library::open(caminho.path()).expect("migra para v6");
    assert_eq!(
        lib.schema_version().unwrap(),
        gp100_library::migrations::latest()
    );

    let v = lib.versoes("u9").expect("historico");
    assert_eq!(v.len(), 1, "o patch antigo ganhou a versao 1");
    assert_eq!(v[0].seq, 1);
    let completa = lib.versao(v[0].id).expect("le").expect("existe");
    assert_eq!(
        completa.payload.as_deref(),
        Some(payload.as_str()),
        "a versao 1 e o retrato do patch como ele estava"
    );
    assert_eq!(
        lib.total_versoes("f0").expect("conta"),
        0,
        "o backfill nao inventa historico de fabrica"
    );
}

#[test]
fn restaurar_total_cria_versao_nova_com_a_cadeia_antiga() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "Take 0", &cadeia("40", "50")))
        .expect("grava v1");
    let id_v1 = lib.versoes("u1").expect("historico")[0].id;
    lib.upsert(&patch_de_usuario("u1", "Take 1", &cadeia("90", "5")))
        .expect("grava v2");

    let nova = lib
        .restaura_versao(id_v1, "2026-10-05T12:00:00Z")
        .expect("restaura")
        .expect("a versao existe");
    assert_eq!(nova.seq, 3, "restaurar ACRESCENTA, nao volta o numero");
    assert_eq!(nova.saved_at, "2026-10-05T12:00:00Z");
    assert!(nova.payload.unwrap().contains("\"40\""));

    // As tres versoes continuam la, e a corrente e a restaurada.
    let v = lib.versoes("u1").expect("historico");
    assert_eq!(v.len(), 3);
    assert!(v[0].current, "a versao 3 (a restauracao) e a corrente");
    let atual = lib.get("u1").expect("le").expect("existe");
    assert!(atual.payload.unwrap().contains("\"40\""));
    assert_eq!(
        atual.pp_type_name, "Rock",
        "restaurar nao apaga o rotulo de estilo do patch"
    );
}

#[test]
fn restaurar_um_knob_muda_so_esse_knob() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "A", &cadeia("40", "85")))
        .expect("grava v1");
    let id_v1 = lib.versoes("u1").expect("historico")[0].id;
    lib.upsert(&patch_de_usuario("u1", "B", &cadeia("95", "10")))
        .expect("grava v2");

    // Volta SO o Gain do slot 0 para o valor da v1.
    let nova = lib
        .restaura_knob(id_v1, 0, 0, "2026-10-05T12:00:00Z")
        .expect("restaura")
        .expect("existe");
    assert_eq!(nova.seq, 3);

    let atual = lib.get("u1").expect("le").expect("existe");
    let payload = atual.payload.unwrap();
    assert!(payload.contains("\"40\""), "o knob ganhou o valor antigo");
    assert!(
        payload.contains("\"10\""),
        "o outro knob NAO voltou junto: {payload}"
    );
    assert_eq!(lib.total_versoes("u1").expect("conta"), 3);
}

#[test]
fn restaurar_um_knob_que_nao_existe_na_versao_e_erro() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "A", &cadeia("40", "85")))
        .expect("grava");
    let id_v1 = lib.versoes("u1").expect("historico")[0].id;

    // O slot 1 nao existe na cadeia: restaurar um knob dele nao pode virar um
    // "escreve null em qualquer coisa" silencioso.
    let err = lib
        .restaura_knob(id_v1, 1, 0, "2026-10-05T12:00:00Z")
        .expect_err("deve recusar");
    assert!(err.to_string().contains("slot 1"), "{err}");
    assert_eq!(
        lib.total_versoes("u1").expect("conta"),
        1,
        "a gravacao que falhou nao deixou versao"
    );
}

#[test]
fn apagar_o_patch_leva_a_historia_junto() {
    let lib = Library::open_in_memory().expect("abre");
    lib.upsert(&patch_de_usuario("u1", "A", &cadeia("40", "50")))
        .expect("grava");
    lib.upsert(&patch_de_usuario("u1", "A2", &cadeia("45", "50")))
        .expect("grava");
    assert_eq!(lib.total_versoes("u1").expect("conta"), 2);

    assert!(lib.delete("u1").expect("apaga"));
    assert_eq!(
        lib.total_versoes("u1").expect("conta"),
        0,
        "historico de patch que nao existe e lixo que o dono nao ve para limpar"
    );
}

#[test]
fn restaurar_versao_inexistente_e_none_e_nao_erro() {
    let lib = Library::open_in_memory().expect("abre");
    assert!(lib
        .restaura_versao(9999, "2026-10-05T12:00:00Z")
        .expect("sem erro")
        .is_none());
}

#[test]
fn o_registro_de_fabrica_importado_nao_versiona_mas_o_de_usuario_sim() {
    // Import e OUTRA porta de entrada para o banco. Se ele escrevesse o
    // registo corrente sem criar versao, a historia teria um buraco por onde
    // um backup restaurado entra.
    let mut lib = Library::open_in_memory().expect("abre");
    let json = serde_json::json!({
        "format": "gp100.library",
        "version": 1,
        "exportedAt": "2026-10-05T12:00:00Z",
        "sqlite": "3.0.0",
        "schema": 6,
        "presets": [
            {
                "id": "u7", "bank": "user", "pp": serde_json::Value::Null,
                "name": "Importado", "ppType": 4, "ppTypeName": "Rock",
                "savedAt": "2026-04-04T00:00:00Z", "payload": cadeia("40", "50")
            },
            {
                "id": "f3", "bank": "factory", "pp": 3,
                "name": "Factory", "ppType": 4, "ppTypeName": "Rock",
                "savedAt": "2026-01-01T00:00:00Z", "payload": serde_json::Value::Null
            }
        ]
    })
    .to_string();

    let rel = lib
        .import_json(&json, gp100_library::ImportMode::Insert)
        .expect("importa");
    assert_eq!(rel.inserted, 2);
    assert_eq!(lib.total_versoes("u7").expect("conta"), 1);
    assert_eq!(lib.total_versoes("f3").expect("conta"), 0);
}
