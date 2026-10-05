//! Contrato do armazenamento dos tons de SnapTone/NAM (issue #25).
//!
//! O que estes testesProtectem, em ordem de importance: o **conteúdo** que
//! entra é o que sai (CRC + bytes), o **slot** é de um tom só (é o recurso do
//! device, `SnapTone1..5`), e **reimportar** o mesmo arquivo não duplica nada
//! (o dono reorganiza a pasta e reimporta).

use gp100_library::snap_tone::crc32;
use gp100_library::{Library, LibraryError};

/// O camelCase no fio é o CONTRATO da ponte IPC (`ui/src/ipc/tones.ts`): se um
/// campo virar `saved_at`, o serde escreve `saved_at`, o TS lê `undefined` e a
/// tela mostra uma data vazia sem nenhum erro no console.
///
/// Este teste existe no crate da biblioteca, e não no do shell, por uma razão
/// concreta: o crate do Tauri é MSVC e só compila na CI do Windows — um
/// contrato de serde verificado aqui é verificado nas três plataformas da
/// matriz a cada push.
#[test]
fn o_contrato_ipc_dos_tons_e_camelcase() {
    let lib = Library::open_in_memory().expect("banco");
    let linha = lib
        .import_tone("Marshall", &modelo(3, 1), agora())
        .expect("importa");

    let linha_json = serde_json::to_value(&linha).expect("serializável");
    assert_eq!(linha_json["savedAt"], agora());
    assert!(linha_json.get("saved_at").is_none());
    assert_eq!(linha_json["crc32"], linha.crc32);
    // A LISTA não carrega o modelo: são ~2,7 KB por tom e a tela só mostra o
    // nome. Ler o modelo é `tone_get`.
    assert!(linha_json.get("model").is_none());

    let tom = lib.ton_com_modelo(&linha.id).expect("lê").expect("existe");
    let tom_json = serde_json::to_value(&tom).expect("serializável");
    // `flatten`: os campos do tom sobem para o objeto, sem `row` no meio.
    assert_eq!(tom_json["name"], "Marshall");
    assert!(tom_json.get("row").is_none());
    assert_eq!(tom_json["savedAt"], agora());
    // O modelo vai como ARRAY DE NÚMEROS — é a representação que o serde dá a
    // `Vec<u8>` em JSON e a que o `Array.from(new Uint8Array(...))` do
    // `<input type="file">` entrega. O teste fixa a representação porque uma
    // mudança aqui (base64, por exemplo) passaria pelo typecheck dos dois
    // lados e quebraria só com o aparelho na mão.
    assert_eq!(tom_json["model"], serde_json::json!(modelo(3, 1)));
}

/// O quadro do gestor traz `slots` e `usados` na MESMA leitura da lista — o
/// rodapé não pode dizer "3 em uso" com a lista mostrando 2.
#[test]
fn o_quadro_traz_lista_e_slots_juntos() {
    let lib = Library::open_in_memory().expect("banco");
    let a = lib.import_tone("A", &modelo(10, 1), agora()).expect("A");
    lib.import_tone("B", &modelo(10, 2), agora()).expect("B");
    lib.atribuir_slot(&a.id, Some(2)).expect("slot");

    let quadro = lib.tone_board().expect("quadro");
    assert_eq!(quadro.slots, 5, "§5: SnapTone1..5");
    assert_eq!(quadro.usados, 1);
    assert_eq!(quadro.tones.len(), 2);

    let json = serde_json::to_value(&quadro).expect("serializável");
    assert_eq!(json["slots"], 5);
    assert_eq!(json["usados"], 1);
    assert!(json["tones"].is_array());
}

/// Modelo sintético de N bytes (o conteúdo é opaco para a biblioteca).
fn modelo(n: usize, semente: u8) -> Vec<u8> {
    (0..n)
        .map(|i| (i * 7 + usize::from(semente)) as u8)
        .collect()
}

fn agora() -> &'static str {
    "2026-10-04T12:00:00Z"
}

/// O CRC-32 é o IEEE, e é conferido contra o valor canônico — o mesmo que
/// `zlib.crc32("123456789")` devolve no Python. Um CRC "parecido" (outro
/// polinômio, outra reflexão) passaria numa tabela de consulta que só o próprio
/// código alimenta, e a falha apareceria quando o `.clo` fosse regravado por
/// fora do app.
#[test]
fn o_crc32_e_o_ieee() {
    assert_eq!(
        crc32(b"123456789"),
        0xCBF4_3926,
        "vetor canônico do CRC-32 IEEE"
    );
    assert_eq!(crc32(b""), 0, "vazio = 0");
    // 32 bytes conhecidos: o mesmo valor que o zlib devolve.
    assert_eq!(
        crc32(b"The quick brown fox jumps over the"),
        crc32_de_referencia(b"The quick brown fox jumps over the")
    );
}

/// "Referência" independente: CRC-32 calculado aqui com a tabela do padrão,
/// escrito de um jeito diferente do da implementação (nibble, tabela pronta).
/// Dois jeitos de escrever a mesma conta é o que separa "está certo" de
/// "bate com ele mesmo".
fn crc32_de_referencia(dados: &[u8]) -> u32 {
    let mut tabela = [0u32; 256];
    for (i, t) in tabela.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
        *t = c;
    }
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in dados {
        crc = tabela[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// O que entra é o que sai: os bytes voltam idênticos e o CRC gravado bate com
/// eles. A lista NÃO traz o modelo (2,7 KB por linha para mostrar um nome).
#[test]
fn o_que_entra_e_o_que_sai() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = modelo(2700, 3);
    let linha = lib
        .import_tone("Marshall 4x12", &bytes, agora())
        .expect("importa");

    assert_eq!(linha.name, "Marshall 4x12");
    assert_eq!(linha.bytes, 2700);
    assert_eq!(linha.crc32, crc32(&bytes) as i64);
    assert_eq!(linha.slot, None, "importar não atribui slot sozinho");

    let lista = lib.tons().expect("lista");
    assert_eq!(lista.len(), 1);
    // O DTO da lista não tem campo de modelo — e é a lista que a tela lê.
    assert!(!format!("{lista:?}").contains("model"));

    let com = lib.ton_com_modelo(&linha.id).expect("lê").expect("existe");
    assert_eq!(com.model, bytes, "o modelo volta byte a byte");
    assert_eq!(com.row, linha);
    assert_eq!(lib.total_tons().unwrap(), 1);
}

/// Reimportar o mesmo arquivo ATUALIZA o registro: o dono reorganiza a pasta,
/// reexporta do Suite e importa de novo — três importações do mesmo `.clo`
/// são um tom, não três.
#[test]
fn reimportar_o_mesmo_arquivo_atualiza_e_nao_duplica() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = modelo(100, 9);
    let primeiro = lib.import_tone("Tone A", &bytes, agora()).expect("1a");
    let segundo = lib
        .import_tone("Tone A renomeado", &bytes, "2026-10-05T12:00:00Z")
        .expect("2a");

    assert_eq!(primeiro.id, segundo.id, "mesmo conteúdo = mesmo registro");
    assert_eq!(segundo.name, "Tone A renomeado", "o nome foi atualizado");
    assert_eq!(lib.total_tons().unwrap(), 1);
    // O slot NÃO é perdido na regrava: quem atribuiu o slot 2 continua com o
    // tom no 2. (Se a regrava limpasse o slot, o dono perderia a atribuição
    // a cada reimportação e nem veria aviso.)
    lib.atribuir_slot(&primeiro.id, Some(2)).expect("atribui");
    lib.import_tone("Tone A de novo", &bytes, agora())
        .expect("3a");
    assert_eq!(lib.ton(&primeiro.id).unwrap().unwrap().slot, Some(2));
}

/// O id vem do CRC, então uma colisão de CRC NÃO pode sobrescrever outro
/// arquivo. O cenário é forçado (escrever a linha com o id que o CRC vai
/// gerar, mas com bytes diferentes) porque achar dois arquivos de 2,7 KB com o
/// mesmo CRC por acaso não acontece na vida real — e o caminho do código é o
/// mesmo nos dois casos.
#[test]
fn id_derivado_do_conteudo_nao_sobrescreve_outro_arquivo() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = modelo(50, 1);
    let crc = crc32(&bytes);
    let id_cruzado = format!("t{crc:08x}");
    let outro = modelo(50, 200);

    // Outro arquivo que "ocupou" o nome com conteúdo diferente.
    lib.conn()
        .execute(
            "INSERT INTO snap_tone (id,name,bytes,crc32,slot,saved_at)
             VALUES (?1,'Voce',50,?2,NULL,?3)",
            rusqlite::params![id_cruzado, crc as i64, agora()],
        )
        .expect("semeia a colisão");
    lib.conn()
        .execute(
            "INSERT INTO snap_tone_model (id,model) VALUES (?1,?2)",
            rusqlite::params![id_cruzado, outro],
        )
        .expect("semeia o blob");

    let linha = lib.import_tone("Meu", &bytes, agora()).expect("importa");
    assert_ne!(linha.id, id_cruzado, "o arquivo novo ganhou id livre");
    assert_eq!(linha.id, format!("{id_cruzado}-2"));
    // E o arquivo que já estava lá continua com os bytes dele.
    assert_eq!(lib.modelo(&id_cruzado).unwrap().unwrap(), outro);
}

/// O slot é de UM tom só: é um recurso do device (`SnapTone1..5`), e dois
/// registros apontando para o mesmo slot fariam a tela mentir sobre o que está
/// gravado no aparelho.
#[test]
fn um_slot_um_tom() {
    let lib = Library::open_in_memory().expect("banco");
    let a = lib
        .import_tone("Vintage", &modelo(30, 1), agora())
        .expect("A");
    let b = lib
        .import_tone("Moderno", &modelo(30, 2), agora())
        .expect("B");

    lib.atribuir_slot(&a.id, Some(3)).expect("A no 3");
    assert_eq!(lib.dono_do_slot(3).unwrap().as_deref(), Some(a.id.as_str()));

    let e = lib
        .atribuir_slot(&b.id, Some(3))
        .expect_err("o slot 3 já é do Vintage");
    match e {
        LibraryError::SlotOcupado { slot, dono } => {
            assert_eq!(slot, 3);
            assert_eq!(dono, "Vintage", "o erro diz QUEM tem o slot");
        }
        outro => panic!("esperado SlotOcupado, veio {outro:?}"),
    }
    // Reatribuir o MESMO tom ao mesmo slot não é colisão — é ele reescrevendo
    // o próprio registro, e a UI faz isso a cada clique na mesma linha.
    lib.atribuir_slot(&a.id, Some(3)).expect("idempotente");

    // Desligar o slot libera o de cima (o que a UI faz no "remover do slot").
    lib.atribuir_slot(&a.id, None).expect("desliga");
    assert_eq!(lib.dono_do_slot(3).unwrap(), None);
    lib.atribuir_slot(&b.id, Some(3)).expect("agora é do B");
}

/// Os 5 slots do firmware passam; 0, 6 e 255 são erro TIPADO.
#[test]
fn slots_de_1_a_5_passam_e_o_resto_e_erro() {
    assert_eq!(gp100_library::snap_tone::SLOTS, 5, "§5: SnapTone1..5");
    let lib = Library::open_in_memory().expect("banco");
    for i in 0..5u8 {
        let t = lib
            .import_tone(&format!("Tone {i}"), &modelo(20, i), agora())
            .expect("importa");
        lib.atribuir_slot(&t.id, Some(i + 1)).expect("slot válido");
    }
    for slot in [0u8, 6, 200] {
        let t = lib
            .import_tone("Outro", &modelo(20, 77), agora())
            .expect("importa");
        assert!(
            matches!(
                lib.atribuir_slot(&t.id, Some(slot)),
                Err(LibraryError::SlotInvalido(_, _))
            ),
            "slot {slot} deveria ser recusado"
        );
    }
}

/// O tom de um slot vem COM o modelo: é o que a tela de A/B toca e o que o
/// upload envia. Uma função que devolvesse só a linha faria a tela buscar o
/// modelo com uma segunda volta — e as duas metades poderiam divergir.
#[test]
fn o_tom_do_slot_traz_o_modelo() {
    let lib = Library::open_in_memory().expect("banco");
    let bytes = modelo(2700, 5);
    let t = lib.import_tone("Amber", &bytes, agora()).expect("importa");
    lib.atribuir_slot(&t.id, Some(5)).expect("slot 5");

    let pelo_slot = lib.ton_do_slot(5).expect("lê").expect("existe");
    assert_eq!(pelo_slot.model, bytes);
    assert_eq!(pelo_slot.row.id, t.id);
    assert_eq!(lib.ton_do_slot(4).expect("slot vazio"), None);
}

/// Apagar o tom leva o modelo junto. Deixar o blob órfão é o tipo de lixo que
/// só aparece quando o banco enche — e o dono não tem como limpá-lo pela UI.
#[test]
fn apagar_toma_o_blob_junto() {
    let lib = Library::open_in_memory().expect("banco");
    let t = lib
        .import_tone("Temp", &modelo(80, 4), agora())
        .expect("importa");
    assert!(lib.apagar_tone(&t.id).expect("apaga"));
    assert!(!lib.apagar_tone(&t.id).expect("apagar 2x = false"));
    assert_eq!(lib.modelo(&t.id).unwrap(), None, "o blob foi junto");
    assert_eq!(lib.ton(&t.id).unwrap(), None);
    assert_eq!(lib.total_tons().unwrap(), 0);
}

/// A ordem é a que o gestor mostra: atribuídos primeiro pelo número do slot,
/// soltos pelo nome. Ordenar por id (CRC) mostraria "qual é o tom do slot 3"
/// numa linha diferente a cada importação.
#[test]
fn a_ordem_e_por_slot_e_depois_nome() {
    let lib = Library::open_in_memory().expect("banco");
    let a = lib
        .import_tone("Zeta", &modelo(20, 1), agora())
        .expect("Zeta");
    let b = lib
        .import_tone("Alfa", &modelo(20, 2), agora())
        .expect("Alfa");
    let c = lib
        .import_tone("Meio", &modelo(20, 3), agora())
        .expect("Meio");
    lib.atribuir_slot(&c.id, Some(1)).expect("Meio no 1");
    lib.atribuir_slot(&a.id, Some(4)).expect("Zeta no 4");

    let nomes: Vec<String> = lib.tons().unwrap().into_iter().map(|t| t.name).collect();
    assert_eq!(
        nomes,
        vec!["Meio", "Zeta", "Alfa"],
        "slots 1 e 4 pelo número, depois os soltos pelo nome"
    );
    let _ = b;
}

/// Nome vazio e modelo de 0 bytes não viram registro: um tom sem modelo é um
/// tom que o upload não consegue enviar, e a lista mostraria um item que não
/// faz nada.
#[test]
fn conteudo_invalido_e_recusado() {
    let lib = Library::open_in_memory().expect("banco");
    assert!(matches!(
        lib.import_tone("  ", &modelo(10, 1), agora()),
        Err(LibraryError::Tone(_))
    ));
    assert!(matches!(
        lib.import_tone("Vazio", &[], agora()),
        Err(LibraryError::Tone(_))
    ));
    assert_eq!(lib.total_tons().unwrap(), 0, "nada foi gravado");
}

/// Renomear um tom que não existe devolve `false` (a UI recua da lista) e não
/// inventa registro.
#[test]
fn renomear_tone_inexistente_nao_cria_nada() {
    let lib = Library::open_in_memory().expect("banco");
    assert!(!lib.renomear_tone("tdeadbeef", "Fantasma").expect("executa"));
    assert_eq!(lib.total_tons().unwrap(), 0);
    assert!(
        lib.renomear_tone("tdeadbeef", "  ").is_err(),
        "nome vazio é erro"
    );
    let t = lib
        .import_tone("Antigo", &modelo(10, 1), agora())
        .expect("importa");
    assert!(lib.renomear_tone(&t.id, "  Novo  ").expect("renomeia"));
    assert_eq!(
        lib.ton(&t.id).unwrap().unwrap().name,
        "Novo",
        "o nome é aparado"
    );
}

/// Atribuir slot a um tom que não existe é erro, não um `UPDATE` silencioso
/// que não muda nada e devolve sucesso — que é como a UI mostraria "atribuído"
/// para um tom que o dono acabou de apagar em outra janela.
#[test]
fn atribuir_slot_a_tom_inexistente_e_erro() {
    let lib = Library::open_in_memory().expect("banco");
    assert!(matches!(
        lib.atribuir_slot("tdeadbeef", Some(1)),
        Err(LibraryError::Tone(_))
    ));
    assert_eq!(lib.dono_do_slot(1).unwrap(), None);
}
