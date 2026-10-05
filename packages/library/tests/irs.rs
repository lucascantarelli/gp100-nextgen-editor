//! Contrato do armazenamento dos IRs do laboratório (issue #24).
//!
//! O que estes testes protegem, em ordem de importancia: o **conteúdo** que
//! entra é o que sai (CRC + bytes), o **slot** é de um IR só e vai de **0** a
//! 19 (é o formato do aparelho, §13.12), o **tamanho** é sempre envíavel pelo
//! fio (§13.7 recusa a cauda), e **reimportar** o mesmo arquivo não duplica
//! nada nem perde o slot que o dono tinha atribuído.

use gp100_library::ir::CHUNK_BYTES;
use gp100_library::{Library, LibraryError};

/// `.ir` sintético de N bytes — o conteúdo é opaco para a biblioteca.
fn blob(n: usize, semente: u8) -> Vec<u8> {
    (0..n)
        .map(|i| (i * 7 + usize::from(semente)) as u8)
        .collect()
}

/// Um `.ir` que o fio aceita: múltiplo de [`CHUNK_BYTES`] e não vazio.
fn ir_valido(n_chunks: usize, semente: u8) -> Vec<u8> {
    blob(n_chunks * CHUNK_BYTES, semente)
}

fn agora() -> &'static str {
    "2026-10-05T12:00:00Z"
}

// ─────────────────────────── conteúdo: entra, sai, volta ────────────────────

/// O que entra é o que sai: os bytes voltam idênticos e o CRC gravado bate
/// com eles. A lista NÃO traz o blob (centenas de KB por linha para mostrar um
/// nome é o que faria a lista do laboratório travar).
#[test]
fn o_que_entra_e_o_que_sai() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = ir_valido(2, 3);
    let linha = lib
        .import_ir("Vintage 4x12", &bytes, agora())
        .expect("importa");

    assert_eq!(linha.name, "Vintage 4x12");
    assert_eq!(linha.bytes, 30);
    assert_eq!(linha.crc32, i64::from(gp100_library::ir::crc32(&bytes)));
    assert_eq!(linha.slot, None, "importar não atribui slot sozinho");

    let lista = lib.irs().expect("lista");
    assert_eq!(lista.len(), 1);
    assert!(
        !format!("{lista:?}").contains("blob"),
        "a lista não carrega o conteúdo"
    );

    let com = lib.ir_com_blob(&linha.id).expect("lê").expect("existe");
    assert_eq!(com.blob, bytes, "o IR volta byte a byte");
    assert_eq!(com.row, linha);
    assert_eq!(lib.total_irs().unwrap(), 1);
}

/// O `.ir` é o arquivo do dono e o CRC é o guardião dele: um IR guardado há
/// meses e regravado por fora (o dono reexporta do Suite) tem que ser
/// reconhecidamente o MESMO registro, não um IR novo com o mesmo nome.
#[test]
fn reimportar_o_mesmo_arquivo_atualiza_e_nao_duplica() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = ir_valido(3, 9);
    let primeiro = lib.import_ir("Cab 4x12", &bytes, agora()).expect("1a");
    let segundo = lib
        .import_ir("Cab 4x12 (bônus)", &bytes, "2026-10-06T12:00:00Z")
        .expect("2a");

    assert_eq!(primeiro.id, segundo.id, "mesmo conteúdo = mesmo registro");
    assert_eq!(segundo.name, "Cab 4x12 (bônus)", "o nome foi atualizado");
    assert_eq!(lib.total_irs().unwrap(), 1);
}

/// Reimportar NÃO perde o slot: quemdepends do slot 3 continua com o IR no 3
/// depois de reimportar o arquivo. (Se a regrava limpasse o slot, o dono
/// perderia a atribuição a cada reexportação do Suite e não veria aviso.)
#[test]
fn reimportar_nao_perde_o_slot() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = ir_valido(1, 1);
    let t = lib.import_ir("Metal", &bytes, agora()).expect("importa");
    lib.atribuir_slot_ir(&t.id, Some(3)).expect("slot 3");
    lib.import_ir("Metal (2)", &bytes, "2026-10-06T12:00:00Z")
        .expect("reimporta");
    assert_eq!(lib.ir(&t.id).unwrap().unwrap().slot, Some(3));
}

/// O id vem do CRC, então uma colisão de CRC NÃO pode sobrescrever outro
/// arquivo. O cenário é forçado (escrever a linha com o id que o CRC vai
/// gerar, mas com bytes diferentes) porque achar dois `.ir` com o mesmo CRC-32
/// por acaso não acontece na vida real — e o caminho do código é o mesmo nos
/// dois casos.
#[test]
fn id_derivado_do_conteudo_nao_sobrescreve_outro_arquivo() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = ir_valido(1, 1);
    let crc = gp100_library::ir::crc32(&bytes);
    let id_cruzado = format!("i{crc:08x}");
    let outro = ir_valido(1, 200);

    lib.conn()
        .execute(
            "INSERT INTO ir_lib (id,name,bytes,crc32,slot,saved_at)
             VALUES (?1,'Voce',15,?2,NULL,?3)",
            rusqlite::params![id_cruzado, i64::from(crc), agora()],
        )
        .expect("semeia a colisão");
    lib.conn()
        .execute(
            "INSERT INTO ir_lib_blob (id,blob) VALUES (?1,?2)",
            rusqlite::params![id_cruzado, outro],
        )
        .expect("semeia o blob");

    let linha = lib.import_ir("Meu", &bytes, agora()).expect("importa");
    assert_ne!(linha.id, id_cruzado, "o arquivo novo ganhou id livre");
    assert_eq!(linha.id, format!("{id_cruzado}-2"));
    assert_eq!(
        lib.blob_ir(&id_cruzado).unwrap().unwrap(),
        outro,
        "e o arquivo que já estava lá continua com os bytes dele"
    );
}

// ─────────────────────────── os slots do aparelho ───────────────────────────

/// São 20 slots (`<ppIRInfo0..19>`, §13.12) e o **0 é o primeiro** — o
/// contrário do SnapTone, que começa em 1. Um teste que aceitasse só 1..=19
/// passaria com o 0 recusado, que é o primeiro erro que o dono veria.
#[test]
fn os_vinte_slots_passam_incluindo_o_zero() {
    assert_eq!(gp100_library::ir::SLOTS, 20, "§13.12: ppIRInfo0..19");
    let lib = Library::open_in_memory().expect("banco");
    for i in 0..gp100_library::ir::SLOTS {
        let t = lib
            .import_ir(&format!("IR {i}"), &ir_valido(1, i), agora())
            .expect("importa");
        lib.atribuir_slot_ir(&t.id, Some(i)).expect("slot válido");
        assert_eq!(lib.ir_do_slot(i).unwrap().unwrap().row.id, t.id);
    }
    assert_eq!(lib.total_irs().unwrap(), 20);
}

/// Slot 20 em diante é erro TIPADO, com a faixa certa na mensagem: a UI
/// escreve o número que veio de um clique, e "esperado 1..=5" num erro de IR
/// seria a tela mentindo sobre o formato do aparelho.
#[test]
fn slot_fora_de_0_a_19_e_erro_tipado() {
    let lib = Library::open_in_memory().expect("banco");
    let t = lib
        .import_ir("A", &ir_valido(1, 1), agora())
        .expect("importa");
    for slot in [20u8, 99, 255] {
        let e = lib
            .atribuir_slot_ir(&t.id, Some(slot))
            .expect_err("slot alto");
        assert!(
            matches!(e, LibraryError::IrSlotInvalido(_, 19)),
            "slot {slot}: veio {e:?}"
        );
        assert!(
            e.to_string().contains("(esperado 0..=19)"),
            "a mensagem diz a FAIXA do IR — 20 slots são 0..=19, e \
             `esperado 0..=20` seria a tela mentindo sobre o aparelho: {e}"
        );
    }
    assert_eq!(
        lib.ir(&t.id).unwrap().unwrap().slot,
        None,
        "nada foi gravado"
    );
}

/// Um slot, um IR: o índice único parcial no banco faz a segunda atribuição
/// falhar, e a checagem antes do `UPDATE` transforma isso em erro COM o nome
/// de quem já tem o slot — que é o que a tela mostra ao dono.
#[test]
fn um_slot_um_ir() {
    let lib = Library::open_in_memory().expect("banco");
    let a = lib
        .import_ir("Vintage", &ir_valido(1, 1), agora())
        .expect("A");
    let b = lib
        .import_ir("Moderno", &ir_valido(1, 2), agora())
        .expect("B");

    lib.atribuir_slot_ir(&a.id, Some(0)).expect("A no 0");
    assert_eq!(
        lib.dono_do_slot_ir(0).unwrap().as_deref(),
        Some(a.id.as_str()),
        "o slot 0 é do A"
    );

    let e = lib
        .atribuir_slot_ir(&b.id, Some(0))
        .expect_err("o 0 é do A");
    match e {
        LibraryError::IrSlotOcupado { slot, dono } => {
            assert_eq!(slot, 0);
            assert_eq!(dono, "Vintage", "o erro diz QUEM tem o slot");
        }
        outro => panic!("esperado IrSlotOcupado, veio {outro:?}"),
    }
    // Reatribuir o MESMO IR ao MESMO slot não é colisão — a UI faz isso a cada
    // clique na mesma linha.
    lib.atribuir_slot_ir(&a.id, Some(0)).expect("idempotente");
    // Desligar o slot libera o de cima (o "remover do slot" da UI).
    lib.atribuir_slot_ir(&a.id, None).expect("desliga");
    assert_eq!(lib.dono_do_slot_ir(0).unwrap(), None);
    lib.atribuir_slot_ir(&b.id, Some(0)).expect("agora é do B");
}

/// Atribuir slot a um IR que não existe é erro, não um `UPDATE` mudo que
/// devolve sucesso — que é como a UI mostraria "atribuído" para um IR que o
/// dono acabou de apagar em outra janela.
#[test]
fn atribuir_slot_a_ir_inexistente_e_erro() {
    let lib = Library::open_in_memory().expect("banco");
    assert!(matches!(
        lib.atribuir_slot_ir("ideadbeef", Some(1)),
        Err(LibraryError::Ir(_))
    ));
    assert_eq!(lib.dono_do_slot_ir(1).unwrap(), None);
}

// ───────────────── a regra do fio, verificada na entrada ───────────────────

/// **A regra que o teste seguinte existe para provar**: §13.7 recusa o pedaço
/// final (strict, rev.2 do ADR-6). Um `.ir` de 22 bytes entraria no banco,
/// apareceria na lista e só falharia no envio — com o dono já com o aparelho
/// na mão e 20 slots à espera. A biblioteca recusa na porta.
#[test]
fn cauda_de_7_bytes_e_recusada_na_importacao() {
    let lib = Library::open_in_memory().expect("banco");
    for n in [1usize, 7, 14, 16, 22, 31] {
        let e = lib
            .import_ir("Truncado", &blob(n, 1), agora())
            .expect_err("cauda não enviável");
        assert!(
            matches!(e, LibraryError::Ir(ref m) if m.contains("multiplo de 15")),
            "{n} bytes: veio {e:?}"
        );
    }
    assert_eq!(lib.total_irs().unwrap(), 0, "nada foi gravado");
    // E o que É múltiplo de 15 entra — inclusive 1 chunk só (15 bytes).
    let ok = lib
        .import_ir("Um chunk", &blob(15, 1), agora())
        .expect("15B");
    assert_eq!(ok.bytes, 15);
}

/// Nome vazio e IR de 0 bytes não viram registro: um registro sem conteúdo é
/// um registro que o envio não consegue usar, e a lista mostraria um item que
/// não faz nada.
#[test]
fn nome_vazio_e_ir_de_zero_bytes_sao_recusados() {
    let lib = Library::open_in_memory().expect("banco");
    assert!(matches!(
        lib.import_ir("  ", &ir_valido(1, 1), agora()),
        Err(LibraryError::Ir(_))
    ));
    assert!(matches!(
        lib.import_ir("Vazio", &[], agora()),
        Err(LibraryError::Ir(_))
    ));
    assert_eq!(lib.total_irs().unwrap(), 0);
}

/// O que está no banco é sempre enviável: a lista inteira passa pela mesma
/// regra de tamanho que o upload. É o teste que amarra a validação da
/// importação com a da FSM (`IR_CHUNK_BYTES` vem do core) — se um número
/// divergisse, o dono importaria um arquivo que o aparelho recusa.
#[test]
fn tudo_que_entra_na_lista_e_enviavel() {
    let lib = Library::open_in_memory().expect("banco");
    for i in 0..5u8 {
        lib.import_ir(&format!("IR {i}"), &ir_valido(i as usize + 1, i), agora())
            .expect("importa");
    }
    for linha in lib.irs().unwrap() {
        let ir = lib.ir_com_blob(&linha.id).unwrap().expect("com blob");
        assert!(
            !ir.blob.is_empty() && ir.blob.len().is_multiple_of(CHUNK_BYTES),
            "{} tem {} bytes: o fio não aceita isso",
            ir.row.name,
            ir.blob.len()
        );
    }
}

// ───────────────────────────── a tela do laboratório ────────────────────────

/// O quadro traz `slots` e `usados` na MESMA leitura da lista — o rodapé não
/// pode dizer "3 em uso" com a lista mostrando 2.
#[test]
fn o_quadro_traz_lista_e_slots_juntos() {
    let lib = Library::open_in_memory().expect("banco");
    let a = lib.import_ir("A", &ir_valido(1, 1), agora()).expect("A");
    lib.import_ir("B", &ir_valido(1, 2), agora()).expect("B");
    lib.atribuir_slot_ir(&a.id, Some(2)).expect("slot");

    let quadro = lib.ir_board().expect("quadro");
    assert_eq!(quadro.slots, 20, "§13.12");
    assert_eq!(quadro.usados, 1);
    assert_eq!(quadro.irs.len(), 2);

    let json = serde_json::to_value(&quadro).expect("serializável");
    assert_eq!(json["slots"], 20);
    assert_eq!(json["usados"], 1);
    assert!(json["irs"].is_array());
}

/// A ordem é a que a tela mostra: atribuídos primeiro pelo número do slot,
/// soltos pelo nome. Ordenar por id (CRC) mostraria "qual é o IR do slot 0" numa
/// linha diferente a cada importação.
#[test]
fn a_ordem_e_por_slot_e_depois_nome() {
    let lib = Library::open_in_memory().expect("banco");
    let z = lib
        .import_ir("Zeta", &ir_valido(1, 1), agora())
        .expect("Zeta");
    lib.import_ir("Alfa", &ir_valido(1, 2), agora())
        .expect("Alfa");
    let m = lib
        .import_ir("Meio", &ir_valido(1, 3), agora())
        .expect("Meio");
    lib.atribuir_slot_ir(&m.id, Some(0)).expect("Meio no 0");
    lib.atribuir_slot_ir(&z.id, Some(4)).expect("Zeta no 4");

    let nomes: Vec<String> = lib.irs().unwrap().into_iter().map(|i| i.name).collect();
    assert_eq!(
        nomes,
        vec!["Meio", "Zeta", "Alfa"],
        "slots 0 e 4 pelo número, depois os soltos pelo nome"
    );
}

/// Apagar leva o blob junto (CASCADE) e apagar duas vezes é `false`.
#[test]
fn apagar_toma_o_blob_junto() {
    let lib = Library::open_in_memory().expect("banco");
    let t = lib
        .import_ir("Temporario", &ir_valido(4, 4), agora())
        .expect("importa");
    assert!(lib.apagar_ir(&t.id).expect("apaga"));
    assert!(!lib.apagar_ir(&t.id).expect("apagar 2x = false"));
    assert_eq!(lib.blob_ir(&t.id).unwrap(), None);
    assert_eq!(lib.total_irs().unwrap(), 0);
}

/// Renomear um IR que não existe devolve `false` (a UI recua da lista) e não
/// inventa registro; nome vazio é erro, não "renomeou para nada".
#[test]
fn renomear_inexistente_nao_cria_nada() {
    let lib = Library::open_in_memory().expect("banco");
    assert!(!lib.renomear_ir("ideadbeef", "Fantasma").expect("executa"));
    assert_eq!(lib.total_irs().unwrap(), 0);
    assert!(lib.renomear_ir("ideadbeef", "  ").is_err(), "nome vazio");
    let t = lib
        .import_ir("Antigo", &ir_valido(1, 1), agora())
        .expect("importa");
    assert!(lib.renomear_ir(&t.id, "  Novo  ").expect("renomeia"));
    assert_eq!(
        lib.ir(&t.id).unwrap().unwrap().name,
        "Novo",
        "o nome é aparado"
    );
}

// ───────────────────────── o contrato da ponte IPC ──────────────────────────

/// O camelCase no fio é o CONTRATO da ponte (`ui/src/ipc/ir.ts`): um campo que
/// virar `saved_at` escreve `saved_at`, o TS lê `undefined` e a tela mostra uma
/// data vazia sem nenhum erro no console.
///
/// Este teste mora no crate da biblioteca, e não no do shell, porque o crate do
/// Tauri é MSVC e só compila na CI do Windows — um contrato de serde verificado
/// aqui é verificado nas três plataformas da matriz a cada push.
#[test]
fn o_contrato_ipc_dos_irs_e_camelcase() {
    let lib = Library::open_in_memory().expect("banco");
    let linha = lib
        .import_ir("Vintage 4x12", &ir_valido(2, 3), agora())
        .expect("importa");

    let linha_json = serde_json::to_value(&linha).expect("serializável");
    assert_eq!(linha_json["savedAt"], agora());
    assert!(linha_json.get("saved_at").is_none());
    assert_eq!(linha_json["crc32"], linha.crc32);
    assert!(linha_json.get("blob").is_none(), "a lista é leve");

    let ir = lib.ir_com_blob(&linha.id).expect("lê").expect("existe");
    let ir_json = serde_json::to_value(&ir).expect("serializável");
    // `flatten`: os campos do IR sobem para o objeto, sem `row` no meio.
    assert_eq!(ir_json["name"], "Vintage 4x12");
    assert!(ir_json.get("row").is_none());
    // O conteúdo vai como ARRAY DE NÚMEROS — é o que o serde dá a `Vec<u8>` em
    // JSON e o que o `Array.from(new Uint8Array(...))` do `<input type="file">`
    // entrega. A representação é fixada aqui porque uma mudança (base64, por
    // exemplo) passaria pelo typecheck dos dois lados e quebraria só com o
    // aparelho na mão.
    assert_eq!(ir_json["blob"], serde_json::json!(ir_valido(2, 3)));
}

/// O CRC é o MESMO dos tons e o mesmo valor canônico do zlib: dois lugares
/// com polinômios diferentes dariam dois ids para o mesmo arquivo, e a lista
/// do dono "sumiria" do nada ao trocar de caminho.
#[test]
fn o_crc32_e_o_ieee_o_mesmo_dos_tons() {
    assert_eq!(
        gp100_library::ir::crc32(b"123456789"),
        0xCBF4_3926,
        "vetor canônico do CRC-32 IEEE"
    );
    assert_eq!(gp100_library::ir::crc32(b""), 0);
    assert_eq!(
        gp100_library::ir::crc32(b"qualquer"),
        gp100_library::snap_tone::crc32(b"qualquer"),
        "IR e tom usam a MESMA função"
    );
}
