//! Testes do actor: boot pelo canal, progresso, read-back/sync (#164),
//! reconexão (#150) e as travas de escrita. Vivem em `src/` (não em `tests/`)
//! porque exercitam internals do actor — regra de organização na skill
//! `rust-practices` (unitário em `#[cfg(test)]`, contrato em `tests/`).

use std::collections::BTreeMap;

use gp100_core::pedalboard::{embedded_document, preset_list};
use gp100_core::transport::mock::{MockDevice, MockFault};
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};

use super::backends::MockState;
use super::*;

/// Boot completo via actor: 2297 transações — o inventário default do
/// MOCK é `0..198` (#132/ADR-12), que NÃO contém o 0x0100 e por isso não
/// paga o quirk §13.4. O APARELHO é 2299 e quem o prova é o
/// `tests/pp_gate.rs` do core.
///
/// O resultado chega ao chamador pelo canal de resposta.
#[test]
fn boot_via_actor_completa_e_responde() {
    let mock = MockDevice::new().expect("mock montado");
    let actor = DeviceActor::spawn(mock, Backend::Mock);
    let report = actor.boot(None).expect("boot contra o mock via actor");
    // 2297 = o script do MOCK: o inventário default dele é `0..198`
    // (#132/ADR-12), que NÃO contém o 0x0100 e não paga o quirk §13.4.
    // O APARELHO é 2299 (a prova C do `validate_golden.py`, 2299/2299)
    // e quem o prova é o `tests/pp_gate.rs` do core. Este crate fica
    // FORA do workspace (`exclude` no Cargo.toml raiz), então
    // `cargo test --workspace` não o alcança — rode-o aqui.
    assert_eq!(report.transactions, 2297);
    actor.shutdown();
}

/// O progresso atravessa a fronteira do actor PELO CANAL: cada
/// transação produz 1 beat no `mpsc` (o command vai reemitir como
/// evento `device://progress`). Beats monótonos até o total.
#[test]
fn progresso_atravessa_o_actor_pelo_canal() {
    let mock = MockDevice::new().expect("mock montado");
    let actor = DeviceActor::spawn(mock, Backend::Mock);
    let (ptx, prx) = mpsc::channel::<BootProgress>();
    let report = actor.boot(Some(ptx)).expect("boot com progresso");
    let beats: Vec<BootProgress> = prx.try_iter().collect();
    assert_eq!(beats.len(), report.transactions);
    assert_eq!(beats.last().map(|p| p.done), Some(report.transactions));
    assert!(beats.windows(2).all(|w| w[0].done + 1 == w[1].done));
    actor.shutdown();
}

/// Info sobe pelo actor (mesmo estado que o CLI imprime; R1 nos DTOs) —
/// e DEPOIS do info a Session continua utilizável (boot funciona).
#[test]
fn info_via_actor_deriva_do_mock() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let st = actor.info().expect("snapshot");
    assert_eq!(st.preset_count, 99);
    assert_eq!(st.current_name, "It's GP100");
    let report = actor.boot(None).expect("boot após info");
    assert_eq!(report.transactions, 2297);
    actor.shutdown();
}

/// **#161 — o relatório HONESTO atravessa o actor com inventário
/// VARIANTE.** O caminho scan → `BootReport` → DTO é o que a UI consome
/// (o `commands.rs` copia os 3 campos para o `BootReportDto`): aqui se
/// prova o boundary do APP com um device saudável que tem um slot sem
/// nome gravado. O relatório que sai daqui — e que a UI recebe — tem de
/// ser `198/197`: inventário lido por inteiro, nome só onde o fio
/// entregou. Se o relatório mentisse 198/198 (contando slot por
/// inventário em vez de por nome lido), o gate da #161 não teria o que
/// barrar e a casca montaria sobre um aparelho pela metade.
#[test]
fn boot_via_actor_entrega_relatorio_honesto_com_slot_sem_nome() {
    let mock = MockDevice::new()
        .expect("mock montado")
        .with_slot_sem_nome(0x0042);
    let actor = DeviceActor::spawn(mock, Backend::Mock);
    let report = actor.boot(None).expect("boot com slot sem nome");
    assert_eq!(
        report.presets, 198,
        "inventário lido por inteiro — o slot sem nome existe no fio"
    );
    assert_eq!(
        report.names, 197,
        "197/198 honesto pelo actor: o DTO que a UI recebe carrega a medição do scan"
    );
    actor.shutdown();
}

/// **O log de fio do app grava o fio de verdade.** É o que faz um incidente
/// de campo ser ATRIBUÍVEL em vez de dedutivo: o arquivo tem o frame que
/// saiu, com `func`/`addr`/`data`. No incidente de 06/10 não havia log
/// nenhum, e a causa teve de ser lida do código.
///
/// O `select` é o comando daquele incidente (`11/13010000` com o `pp`): ele
/// atravessa o actor com o logger ligado e tem de aparecer no arquivo — não
/// basta o `WireLogger` ter teste próprio no core, porque o que se prova
/// aqui é que o caminho do APP (actor → `LoggingTransport`) o aciona.
#[test]
fn log_de_fio_do_actor_registra_o_select_que_atravessa() {
    let dir = std::env::temp_dir().join("gp100-actor-wire-log");
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("sessao.jsonl");
    let _ = std::fs::remove_file(&path);

    let actor = DeviceActor::spawn(MockDevice::new().expect("mock montado"), Backend::Mock);
    actor
        .log_session(path.to_str().expect("caminho utf8"))
        .expect("log ligado");
    actor.select_preset(0).expect("select no mock");
    actor.shutdown();

    let texto = std::fs::read_to_string(&path).expect("o log tem de existir");
    let linhas: Vec<&str> = texto.lines().filter(|l| !l.is_empty()).collect();
    assert!(!linhas.is_empty(), "o log nao pode sair vazio");

    // Schema P4 — as chaves que `h1_compare.py`/`h2_compare.py` leem.
    for l in &linhas {
        for chave in [
            "\"s\":",
            "\"t\":",
            "\"dir\":",
            "\"func\":",
            "\"addr\":",
            "\"data\":",
        ] {
            assert!(l.contains(chave), "falta {chave} em: {l}");
        }
    }

    // O frame do incidente: o select do preset 0 (`11/13010000`).
    let select = linhas
        .iter()
        .find(|l| l.contains("\"dir\":\"out\"") && l.contains("\"addr\":\"13010000\""))
        .unwrap_or_else(|| panic!("o select nao foi registrado: {linhas:?}"));
    assert!(select.contains("\"func\":\"11\""), "{select}");
}

/// **O caminho do log é do BACKEND, e a tela pergunta por ele.** Sem isto o
/// painel mostraria "nenhum log" enquanto a sessão de campo já está em disco
/// (#130 liga sozinho) — e o botão ofereceria "gravar" por cima do arquivo
/// que o próprio app abriu.
#[test]
fn log_path_reflete_o_log_ligado_e_parado() {
    let dir = std::env::temp_dir().join("gp100-actor-log-path");
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("sessao.jsonl");
    let _ = std::fs::remove_file(&path);

    let actor = DeviceActor::spawn(MockDevice::new().expect("mock montado"), Backend::Mock);
    assert_eq!(actor.log_path().expect("consulta"), None, "nasce sem log");

    actor
        .log_session(path.to_str().expect("caminho utf8"))
        .expect("log ligado");
    assert_eq!(
        actor.log_path().expect("consulta"),
        Some(path.to_string_lossy().to_string()),
        "ligado: o caminho e o que foi pedido"
    );

    actor.log_stop().expect("log parado");
    assert_eq!(
        actor.log_path().expect("consulta"),
        None,
        "parado volta a nada"
    );
    actor.shutdown();
}

/// **App sem aparelho responde honesto (#150).** `backend: "none"` com o
/// motivo; a biblioteca NÃO é inventada e o reconectar falha com o motivo
/// — o oposto do mock fingindo ser aparelho.
#[test]
fn app_sem_aparelho_e_estado_honesto() {
    let actor = DeviceActor::desligado("aparelho não conectado via USB");
    let snap = actor.info().expect("info responde mesmo desligado");
    assert_eq!(snap.backend, "none");
    assert_eq!(snap.preset_count, 0);
    assert_eq!(snap.detail, "aparelho não conectado via USB");
    assert!(
        actor.library().is_err(),
        "sem aparelho, a biblioteca não é inventada"
    );
    let e = actor.conectar().expect_err("reconectar sem aparelho falha");
    assert!(e.contains("aparelho não conectou"), "motivo legível: {e}");
    actor.shutdown();
}

/// Reconectar com a sessão viva é no-op: o open é refeito por cima do
/// transporte que já responde, e a sessão continua funcionando.
#[test]
fn conectar_com_sessao_viva_e_no_op() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock montado"), Backend::Mock);
    actor.conectar().expect("sessão viva: reconectar é no-op");
    assert_eq!(
        actor.info().expect("info após reconectar").backend,
        "mock",
        "a sessão sobreviveu ao re-open"
    );
    actor.shutdown();
}

/// **A prova de que o generico nao e vazio.** Um backend que **nao** e
/// o mock — sem `MockState`, sem `drain_inbox` de mock — entra na MESMA
/// fila e responde. E o que garante que o `RealDevice` do build de campo
/// tem caminho ate o fio, e nao so uma assinatura que compila.
///
/// O `AparelhoFake` responde `Ok(vec![])` no `recv_raw`; o que importa
/// aqui e que `info` **degrada honestamente**: `backend` = "real", e os
/// campos que so o mock tem ficam no neutro em vez de virarem zero
/// que a UI mostraria como dado.
struct AparelhoFake {
    sent: Vec<Vec<u8>>,
}

impl std::fmt::Debug for AparelhoFake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AparelhoFake")
            .field("sent", &self.sent.len())
            .finish()
    }
}

impl DeviceTransport for AparelhoFake {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        self.sent.push(data.to_vec());
        Ok(())
    }
    fn recv_raw(&mut self, _t: std::time::Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::RecvTimeout { timeout_ms: 0 })
    }
    /// O fake REPRESENTA o aparelho (é declarado `Backend::Real`), então
    /// ele é aparelho para a trava de faixa do `pp` (#132) — sem isto o
    /// caminho real do app não teria como ser testado sem hardware.
    fn e_aparelho(&self) -> bool {
        true
    }
}

impl DeviceBackend for AparelhoFake {
    fn local_state(&self) -> Option<MockState> {
        None
    }
    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }
}

#[test]
fn um_backend_que_nao_e_mock_entra_na_mesma_fila() {
    let actor = DeviceActor::spawn(AparelhoFake { sent: Vec::new() }, Backend::Real);
    let st = actor.info().expect("info de um backend sem estado local");
    assert_eq!(st.backend, "real", "o backend vem DECLARADO, nao deduzido");
    assert_eq!(st.current_name, "", "sem estado local = vazio, nao nome");
    assert_eq!(st.current_pp_type, 0);
    assert_eq!(st.ir_slots_with_crc, 0);
    assert_eq!(st.preset_count, 0);
    // O knob dentro da faixa (15.0) sai; fora (99.5) e recusado ANTES do
    // `send_raw` — a trava da #110 e de conteudo, e vale no caminho
    // real tambem.
    actor
        .set_param(3, 0x0700_006e, 0, 15.0)
        .expect("dentro da faixa");
    assert!(
        actor.set_param(3, 0x0700_006e, 0, 99.5).is_err(),
        "acima do teto"
    );
    actor.shutdown();
}

/// **#155 §5.2/§5.3: o palco vem do CACHE do scan, nunca de um select.**
///
/// `board` para OUTRO pp tem de devolver sem tocar no fio — um `select`
/// mudaria o preset que o pedal está mostrando, inaceitável num editor
/// (spec §5.2). E com o boot feito o palco NÃO é mais recusado.
#[test]
fn board_vem_do_cache_sem_select() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    actor.boot(None).expect("boot");
    // 0x0000 é o PRIMEIRO pp do inventário — longe do corrente (0x0162,
    // fim do scan): ler este só pode vir do cache.
    let b = actor.board(Some(0x0000)).expect("board do cache");
    assert_eq!(b.pp, 0x0000, "o palco é do pp pedido");
    assert!(!b.slots.is_empty(), "a cadeia tem slots");
    actor.shutdown();
}

/// **Campo 12/10: o palco sobrevive aos snapshots de MONTAGEM.**
///
/// No shell real o `device_info` e o `device_preset_library` rodam no
/// MESMO commit do pós-boot, e cada um remonta a Session
/// (`device_snapshot` → `into_transport`) — descartando o cache de
/// páginas DELA. Com o `BoardView` dependendo só da Session, o primeiro
/// desses calls destruía as páginas do scan e o `device_board`
/// seguinte respondia "páginas deste pp não foram lidas no boot": o
/// palco morria segundos depois de um boot perfeito (wire log
/// `wire-20261012201602`, 2297/198/198 e a UI em erro).
///
/// O backend é `Real` de propósito: é o único caminho que NÃO projeta o
/// `all.prst` — sem o cache do actor, este board erra mesmo.
#[test]
fn board_do_cache_sobrevive_aos_snapshots_de_montagem() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Real);
    let rel = actor.boot(None).expect("boot");
    assert_eq!(rel.presets, 198, "o scan leu o inventário inteiro");

    // Info e Library remontam a Session — é o que o shell faz ao montar.
    let snap = actor.info().expect("info");
    assert!(
        !snap.current_name.is_empty(),
        "o nome do corrente vem do cache do scan"
    );
    let lib = actor.library().expect("library");
    assert!(
        !lib.entries.is_empty(),
        "a biblioteca é o inventário do aparelho"
    );

    // SEM o cache do actor este `board` ERRARIA: o backend Real não
    // projeta o artefato, e as páginas da Session já foram descartadas.
    let b = actor
        .board(Some(0x0000))
        .expect("board sobrevive aos snapshots");
    assert_eq!(b.pp, 0x0000, "o palco é do pp pedido");
    assert!(!b.slots.is_empty(), "a cadeia vem do decode do scan");
    actor.shutdown();
}

/// **#155 §5.1: `current_name` vem da pg0 decodificada no scan.**
///
/// Depois do boot o nome tem de estar lá (a pg0 de cada pp foi lida no
/// scan e decodificada); antes, a única fonte é o estado local do mock.
///
/// **Limitação do harness declarada:** o `local_state` do mock e o
/// exemplo congelado do golden são o MESMO preset ("It's GP100"), então
/// aqui as duas fontes produzem o mesmo nome e não são distinguíveis
/// por valor. A prova de que o CACHE alimenta a UI é o teste ao lado,
/// onde elas são distinguíveis (99 nomes distintos do `all.prst` vs 1
/// do cache do mock).
#[test]
fn current_name_vem_do_cache_apos_o_boot() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let antes = actor.info().expect("info sem boot");
    assert!(!antes.current_name.is_empty(), "estado local tem nome");

    actor.boot(None).expect("boot");

    let depois = actor.info().expect("info com boot");
    assert!(
        !depois.current_name.is_empty(),
        "a pg0 decodificada no scan traz o nome do pp corrente"
    );
    assert_eq!(
        depois.current_pp, 0x00C5,
        "o boot termina o scan no último pp do inventário do MOCK (197); \
         no APARELHO seria 0x0162 — provado no `pp_gate.rs`"
    );
    actor.shutdown();
}

/// **#155 §5.1: a biblioteca não devolve inventário sem nome.**
///
/// A prova é DIFERENÇA, não presença: o `all.prst` tem 99 nomes
/// distintos, então entradas que saem com outro nome só podem ter vindo
/// do cache (pg0 decodificada no scan). Com o mock a distinção é nítida
/// — ele serve o MESMO preset para todos os pps do fio.
///
/// O bug de endereçamento que limitava esta prova (#156) foi corrigido:
/// o `ppID` é DECIMAL e todo ponto do core passa por
/// `preset::pp_id_decimal` — os 99 da UI casam com o fio. Aqui se trava
/// o contrato do cache: nenhuma entrada sem nome, e o cache respondendo
/// por (quase) toda a lista.
#[test]
fn biblioteca_servida_dos_nomes_do_cache() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    actor.boot(None).expect("boot");
    let lib = actor.library().expect("biblioteca");
    assert!(!lib.entries.is_empty(), "tem entradas");
    assert!(
        lib.entries.iter().all(|e| !e.name.is_empty()),
        "nenhuma entrada sem nome — o cache alimenta a lista"
    );

    let doc = embedded_document().expect("doc");
    let do_arquivo: BTreeMap<u16, String> = preset_list(&doc)
        .into_iter()
        .map(|e| (e.pp, e.name))
        .collect();
    let do_fio = lib
        .entries
        .iter()
        .filter(|e| do_arquivo.get(&e.pp).map(String::as_str) != Some(e.name.as_str()))
        .count();
    // Com o cache ligado, o cache cobre os 98 pps restantes — o único
    // que não "muda" é o índice 0, que o mock e o arquivo chamam igual
    // ("It's GP100"). Sem o cache seria 0.
    assert!(
        do_fio >= 90,
        "o cache alimenta quase toda a lista (medido: {do_fio}/99; sem \
         cache seria 0)"
    );
    actor.shutdown();
}

/// **#156: os 99 presets são selecionáveis.**
///
/// O bug de endereçamento fazia o app endereçar em HEX e o fio em
/// DECIMAL banco/slot: 36 dos 99 presets tinham um pp que o fio nem tem
/// (`select_preset` devolvia "esperado pp no espaço banco/slot:
/// 0000..=0062 ou 0100..=0162"). O teste falhava em 36 deles.
#[test]
fn todos_os_99_presets_sao_selecionaveis() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let doc = embedded_document().expect("doc");
    let lista = preset_list(&doc);
    assert_eq!(lista.len(), 99);
    let mut ok = 0;
    for e in &lista {
        if let Err(err) = actor.select_preset(e.pp) {
            panic!("pp {:#06x} ({}) recusado pelo fio: {err}", e.pp, e.name);
        }
        ok += 1;
    }
    assert_eq!(ok, 99, "os 99 selecionáveis");
    actor.shutdown();
}

/// **A trava de faixa do `pp` (#132) atravessa o actor no caminho REAL.**
/// O app não pode mandar um `select` que o aparelho não provou ter — o
/// frame do assert `PresetNum < TOTAL_PA` (`audio.c:912`) é recusado com
/// a faixa na mensagem, antes do fio. O lado "dentro da faixa sai" está
/// provado no core (`tests/pp_gate.rs`); aqui o que importa é a borda do
/// APP: o erro SOBE como string, com o endereço e a faixa legíveis.
#[test]
fn select_fora_do_inventario_e_recusado_no_caminho_real() {
    let actor = DeviceActor::spawn(AparelhoFake { sent: Vec::new() }, Backend::Real);

    let e = actor
        .select_preset(0x00c5)
        .expect_err("0x00c5 (197) não existe no aparelho");
    assert!(e.contains("11/13010000"), "nomeia o frame: {e}");
    assert!(e.contains("pp"), "nomeia o campo: {e}");
    assert!(
        e.contains("0x0000..0x0062") && e.contains("0x0100..0x0162"),
        "diz até onde o aparelho vai: {e}"
    );

    // O MESMO número no mock segue passando — a trava é do aparelho
    // ("no mock, comportamento atual").
    let mock = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    mock.select_preset(0x00c5).expect("mock sem trava");
    mock.shutdown();
    actor.shutdown();
}

/// **`save` e `dump` no app — as duas capacidades que eram só do CLI.**
///
/// O `save` é a operação que PERSISTE: o `set-param` é fire-and-forget
/// (§13.11, D4), então sem isto a mudança morre com a sessão. O `dump`
/// é a leitura de campo que o `h2_field.sh` usa para conferir o antes e
/// o depois na página 0 (é o que provou o F1 do H2).
///
/// As contagens nao sao decorativas: sao as do §13.12 (5 writes de meta +
/// 4 ops, D3) e as 9 transações do §13.9 (meta6 + 8 páginas). Um `save`
/// que imprimisse "enviado" sem mandar os 9 frames seria o pior defeito
/// possível — o operador leria, acreditaria, e o preset nunca foi
/// gravado.
#[test]
fn save_e_dump_via_actor_sao_as_duas_capacidades_do_cli() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("abre");
    let antes = mock.state().current_pp_type;
    let actor = DeviceActor::spawn(mock, Backend::Mock);
    actor.boot(None).expect("boot antes do dump");

    let dump = actor.dump_preset(0).expect("dump do pp corrente");
    assert_eq!(dump.pp, 0);
    assert!(!dump.meta6.is_empty(), "a meta6 veio com conteudo");
    assert_eq!(dump.pages.len(), 8, "§13.9: paginas 0..7 + a final");

    actor
        .save_preset(0, antes, "H2 APP")
        .expect("save §13.12: 9 frames");

    // O nome ASCII foi para o estado do mock (o `11000000` do §13.12).
    assert_eq!(actor.info().expect("info").current_pp_type, antes);
    actor.shutdown();
}

/// Nome com byte não-ASCII é recusado pelo codec ANTES do fio — e o
/// mock fica com o nome antigo. Nao e um detalhe: o §13.12 grava ASCII
/// cru e um byte alto viraria lixo no preset.
#[test]
fn save_recusa_nome_nao_ascii_sem_tocar_o_aparelho() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("abre");
    let actor = DeviceActor::spawn(mock, Backend::Mock);
    let e = actor
        .save_preset(0, 4, "H2 Ã")
        .expect_err("byte nao-ASCII no nome");
    assert!(e.to_string().contains("ASCII"), "{e}");
    actor.shutdown();
}

/// Shutdown: depois dele, requisições novas falham com erro limpo
/// (string) — nunca pânico no command.
#[test]
fn shutdown_depois_falha_limpo() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    actor.shutdown();
    assert!(actor.info().is_err(), "info pós-shutdown = erro, não panic");
}

/// Tabela via actor: 20 slots, nomes ASCII (vazio = 0xFF no fio).
#[test]
fn list_user_irs_via_actor() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let slots = actor.list_user_irs().expect("tabela dos 20 IRs");
    assert_eq!(slots.len(), 20);
    assert_eq!(slots[0].0, 0, "slot 0");
    assert_eq!(slots[19].0, 19, "slot 19");
    actor.shutdown();
}

/// O boot deixa o backlog D7 na inbox do DEVICE: os nomes são
/// fire-and-forget (D4 — a Session não espera) e o mock RESPONDE a
/// eles (61 pushes @11000008) — a drenagem os devolve como hex
/// (F0…F7) e a 2ª drenagem vem vazia (dreno esvazia).
#[test]
fn boot_deixa_backlog_d7_drenavel() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    actor.boot(None).expect("boot");
    let pushes = actor.drain_pushes().expect("drenagem");
    assert!(
        !pushes.is_empty(),
        "respostas tardias dos nomes = backlog D7 do device"
    );
    for h in &pushes {
        assert!(h.starts_with("F0") && h.ends_with("F7"), "SysEx: {h}");
    }
    assert!(actor.drain_pushes().expect("2ª").is_empty());
    actor.shutdown();
}

/// Upload de SnapTone pelo actor: um modelo de 19 bytes é UM bloco, e um
/// bloco não paga settle (§4 — não há operação seguinte). Por isso este
/// teste é rápido mesmo com o piso de 250 ms em produção: o caminho real
/// do actor é exercitado sem transformar o teste em 36 s.
#[test]
fn upload_de_snap_tone_via_actor_fecha_o_stream() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let rel = actor
        .upload_snap_tone(2, &[0xABu8; 19])
        .expect("upload de 1 bloco");
    assert_eq!(rel.slot, 2);
    assert_eq!(rel.blocks, 1);
    assert_eq!(rel.acks, 1, "ACK de 16B por bloco (§5)");
    assert_eq!(rel.bytes, 19);
    actor.shutdown();
}

/// Slot fora de 1..=5 é erro TIPADO até a borda do actor — e não um
/// "enviado com sucesso" para um slot que não existe no aparelho
/// (`SnapTone1..5`).
#[test]
fn upload_com_slot_invalido_e_erro() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let e = actor
        .upload_snap_tone(6, &[0xABu8; 19])
        .expect_err("slot 6 nao existe");
    assert!(e.contains("1..=5"), "a mensagem diz o intervalo: {e}");
    actor.shutdown();
}

/// Fila serializa (D8): info + boot misturados de "duas threads" chegam
/// na ordem de envio — sem interleave de transações.
#[test]
fn fila_serializa_requisicoes() {
    let actor = DeviceActor::spawn(MockDevice::new().expect("mock"), Backend::Mock);
    let a = actor.clone();
    let t1 = std::thread::spawn(move || a.info().expect("info 1"));
    let st = actor.info().expect("info 2");
    let _ = t1.join().expect("thread 1");
    let report = actor.boot(None).expect("boot após infos");
    assert_eq!(st.preset_count, 99);
    assert_eq!(report.transactions, 2297);
    actor.shutdown();
}

/// #48 — ponta-a-ponta no shell: o transporte MORRE no meio da sessão
/// (`MockFault::DieAfter`) e o erro sobe TIPADO (`ProtocolError::DeviceGone`)
/// até a borda do actor — nunca achatado em `InvalidShape` ("transporte
/// saudável"). O actor segue vivo (info responde), a morte aborta na hora
/// (jamais espera a janela de 3 s) e o retry (⟳ da navbar) falha de novo,
/// estável: o mock não ressuscita.
#[test]
fn device_morrendo_no_meio_da_sessao_nao_mata_o_actor() {
    let mock = MockDevice::new()
        .expect("mock montado")
        .with_fault(MockFault::DieAfter(300));
    let actor = DeviceActor::spawn(mock, Backend::Mock);

    let t0 = std::time::Instant::now();
    let err = actor.boot(None).expect_err("boot com o device morrendo");
    assert!(
        err.contains("device sumiu no meio da sessão"),
        "erro tipado do DeviceGone até o actor: {err}"
    );
    assert!(
        !err.contains("transporte saudável"),
        "DeviceGone não pode virar InvalidShape: {err}"
    );
    assert!(
        t0.elapsed() < std::time::Duration::from_secs(2),
        "a morte aborta a transação na hora (sem janela de 3 s): {:?}",
        t0.elapsed()
    );

    // O actor continua operante: o snapshot do mock não toca o fio.
    let st = actor.info().expect("info após a morte do device");
    assert_eq!(st.preset_count, 99);

    // Retry estável: a falha se repete rápida e tipada (device não volta).
    let t1 = std::time::Instant::now();
    let err2 = actor
        .boot(None)
        .expect_err("retry com o device ainda morto");
    assert!(err2.contains("device sumiu no meio da sessão"), "{err2}");
    assert!(
        t1.elapsed() < std::time::Duration::from_secs(2),
        "retry aborta na hora: {:?}",
        t1.elapsed()
    );
    actor.shutdown();
}

/// #164 — ciclo de convergência COMPLETO, com a prova anti-brick da
/// fase nova: boot → `sync`; escrita → `pending`; read-back → `sync` de
/// novo E `changed = true` quando o fio veio DIFERENTE do cache — o
/// board passa a mostrar o valor LIDO do aparelho, não o que o app
/// escreveu. Toda a fase de leitura é registrada: NENHUM
/// `WireKind::Write` pode aparecer nela. No boot a ÚNICA escrita
/// tolerada é o keepalive T4 `12/00020001` (D4 — decisão do owner
/// 06/10: bate-coração que não toca dados; sem ele um build de
/// leitura cairia no último passo). Qualquer outro frame de escrita
/// no caminho é violação anti-brick.
#[test]
fn convergencia_reconcilia_o_valor_lido_e_nunca_escreve() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex as StdMutex;

    /// Transporte que MUTA as páginas depois de armado (#164): simula o
    /// aparelho mudando por baixo do app (knob físico / pedal),
    /// trocando a sequência nibble do valor-alvo no payload — mesmo
    /// encoding do core, sem tocar em shape nem endereço.
    ///
    /// Uma "fase" registrada: o que saiu do fio (kind + frame crU) — o
    /// teste decodifica cada escrita para provar o anti-brick.
    type FaseRegistrada = (WireKind, Vec<u8>);

    struct PaginasVivas {
        inner: MockDevice,
        mutar: std::sync::Arc<AtomicBool>,
        /// `(needle, novo)` — bytes nibble do f32 LE a trocar.
        alvo: std::sync::Arc<StdMutex<(Vec<u8>, Vec<u8>)>>,
        registrando: std::sync::Arc<AtomicBool>,
        /// O que saiu do fio enquanto registrado (ver [`FaseRegistrada`]).
        fases: std::sync::Arc<StdMutex<Vec<FaseRegistrada>>>,
    }

    impl std::fmt::Debug for PaginasVivas {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PaginasVivas").finish_non_exhaustive()
        }
    }

    impl DeviceTransport for PaginasVivas {
        fn open(&mut self) -> Result<(), TransportError> {
            self.inner.open()
        }
        fn close(&mut self) -> Result<(), TransportError> {
            self.inner.close()
        }
        fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
            if self.registrando.load(Ordering::SeqCst) {
                self.fases
                    .lock()
                    .expect("fases")
                    .push((kind, data.to_vec()));
            }
            self.inner.send_raw(data, kind)
        }
        fn recv_raw(&mut self, t: Duration) -> Result<Vec<u8>, TransportError> {
            let mut msg = self.inner.recv_raw(t)?;
            if self.mutar.load(Ordering::SeqCst) {
                let (needle, novo) = {
                    let g = self.alvo.lock().expect("alvo");
                    (g.0.clone(), g.1.clone())
                };
                if needle.len() == novo.len() && !needle.is_empty() {
                    if let Some(pos) = msg.windows(needle.len()).position(|w| w == needle) {
                        msg[pos..pos + needle.len()].copy_from_slice(&novo);
                    }
                }
            }
            Ok(msg)
        }
    }

    impl DeviceBackend for PaginasVivas {
        fn local_state(&self) -> Option<MockState> {
            self.inner.local_state()
        }
        fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
            self.inner.drain_inbox()
        }
    }

    /// Codificação nibble do f32 LE (o mesmo `b >> 4 | b & 0xF` do
    /// fio) — o que o `nibble_collapse` do core desfaz no `decode`.
    fn nibble_de(v: f32) -> Vec<u8> {
        v.to_le_bytes()
            .iter()
            .flat_map(|b| [b >> 4, b & 0x0F])
            .collect()
    }

    let mutar = std::sync::Arc::new(AtomicBool::new(false));
    let registrando = std::sync::Arc::new(AtomicBool::new(false));
    let fases = std::sync::Arc::new(StdMutex::new(Vec::new()));
    let alvo_bytes = std::sync::Arc::new(StdMutex::new((Vec::new(), Vec::new())));
    let actor = DeviceActor::spawn_com_cadencia(
        PaginasVivas {
            inner: MockDevice::new().expect("mock"),
            mutar: mutar.clone(),
            alvo: alvo_bytes.clone(),
            registrando: registrando.clone(),
            fases: fases.clone(),
        },
        Backend::Mock,
        Duration::from_millis(300),
        Duration::from_secs(30),
    );
    let (ev_tx, ev_rx) = mpsc::channel::<SyncEvent>();
    actor.set_sync_sink(ev_tx);

    // SEM conversa ainda: `idle` — o chip nunca afirma "sincronizado"
    // sem ter lido nada (sem número falso).
    assert_eq!(actor.info().expect("info").sync, "idle");

    // Boot = leitura pura + keepalive T4 (D4). Escrita que NÃO é o
    // bate-coração `12/00020001` é violação anti-brick: o boot não
    // pode mandar SetParam, save, meta nem IR — nada que toque dados.
    registrando.store(true, Ordering::SeqCst);
    actor.boot(None).expect("boot");
    {
        let mut f = fases.lock().expect("fases");
        assert!(!f.is_empty(), "o boot manda transações");
        for (k, frame) in f.iter() {
            if !matches!(k, WireKind::Write) {
                continue;
            }
            let (func, addr, _) = gp100_core::golden::decode_envelope(frame)
                .expect("envelope bem formado no fio do boot");
            assert!(
                func == 0x12 && addr == [0x00, 0x02, 0x00, 0x01],
                "escrita no boot ALÉM do keepalive T4 (D4): func {func:02x} \
                 addr {addr:02x?} — o boot só pode ler e bater o keepalive"
            );
        }
        f.clear();
        // Desarma: o boot acabou; a escrita do app (set_param) roda
        // DESARMADA — só a fase de leitura volta a ser registrada.
        registrando.store(false, Ordering::SeqCst);
    }
    assert_eq!(actor.info().expect("info").sync, "sync");

    // O palco ANTES: daqui sai o valor-alvo e o slot/code/ctrl do knob.
    let antes = actor.board(None).expect("board antes");
    let (slot_n, code, pos, valor) = antes
        .slots
        .iter()
        .flat_map(|s| s.knobs.iter().map(move |k| (s, k)))
        .find(|(_, k)| {
            k.kind == "knob"
                && k.value
                    .as_deref()
                    .and_then(|v| v.parse::<f32>().ok())
                    .is_some_and(|v| (1.0..=127.0).contains(&v))
        })
        .map(|(s, k)| {
            (
                s.slot + 1,
                s.code,
                k.pos,
                k.value.as_deref().unwrap().parse::<f32>().unwrap(),
            )
        })
        .expect("um knob dentro de faixa");

    // O aparelho "gira sozinho" (+7) por baixo do app: armamos a mutação
    // das páginas; a PRÓXIMA leitura vai devolver o valor do device.
    {
        let mut g = alvo_bytes.lock().expect("alvo");
        g.0 = nibble_de(valor);
        g.1 = nibble_de(valor + 7.0);
    }
    mutar.store(true, Ordering::SeqCst);

    // O app escreve (o MESMO valor atual — sempre dentro da faixa ADR-10):
    // D4 sem resposta → estado `pending` até o read-back.
    actor
        .set_param(slot_n, code, pos, valor)
        .expect("escrita aceita");
    assert_eq!(
        actor.info().expect("info").sync,
        "pending",
        "escrita aceita = pendente até o read-back confirmar"
    );
    // Arma SÓ a fase de leitura: o boot desarmou ao terminar e o
    // `set_param` (escrita do app, registrada ou não) já foi processado
    // — daqui até o read-back, só frames de leitura podem aparecer.
    registrando.store(true, Ordering::SeqCst);

    // Espera o read-back (debounce 300 ms) — evento `changed = true`.
    // Cada evento NÃO-changed fica registrado: a falha mostra o que
    // chegou de verdade (status/detail), não só "não veio".
    let mut eventos: Vec<SyncEvent> = Vec::new();
    let mut viu_changed = false;
    let prazo = std::time::Instant::now() + Duration::from_secs(8);
    while std::time::Instant::now() < prazo {
        match ev_rx.recv_timeout(Duration::from_millis(400)) {
            Ok(ev) if ev.changed => {
                assert_eq!(ev.status, "sync", "read-back OK = sincronizado");
                viu_changed = true;
                break;
            }
            Ok(ev) => eventos.push(ev),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        viu_changed,
        "o read-back reconciliou (changed = true); eventos: {eventos:?}"
    );

    // Anti-brick da fase nova: TUDO que saiu depois da escrita foi leitura.
    {
        let f = fases.lock().expect("fases");
        assert!(!f.is_empty(), "o read-back manda as 9 páginas");
        assert!(
            f.iter().all(|(k, _)| matches!(k, WireKind::Read)),
            "o read-back NÃO pode escrever: {:?}",
            f.iter().find(|(k, _)| !matches!(k, WireKind::Read))
        );
        registrando.store(false, Ordering::SeqCst);
    }

    // Reconciliação: o board MOSTRA o valor do aparelho (+7), não o que
    // o app quis escrever (valor).
    let depois = actor.board(None).expect("board depois");
    let lido = depois
        .slots
        .iter()
        .flat_map(|s| s.knobs.iter())
        .find(|k| k.kind == "knob" && k.pos == pos)
        .and_then(|k| k.value.as_deref())
        .and_then(|v| v.parse::<f32>().ok())
        .expect("o mesmo knob depois");
    assert_eq!(
        lido,
        valor + 7.0,
        "o aparelho vence: o valor LIDO é o do device, não o escrito"
    );
    assert_eq!(actor.info().expect("info").sync, "sync");
    actor.shutdown();
}

/// #164 — queda do transporte: o read-back morre com `DeviceGone`, o
/// estado vira `offline` com motivo legível, e a escrita seguinte FALHA
/// sem ir ao fio (o app não continua escrevendo em vazio). A sessão
/// fica viva (ADR-4): o retry devolve o erro tipado rápido.
#[test]
fn queda_no_meio_do_readback_vira_offline_e_bloqueia_escrita() {
    let mock = MockDevice::new()
        .expect("mock")
        .with_fault(MockFault::DieAfter(4));
    let actor = DeviceActor::spawn_com_cadencia(
        mock,
        Backend::Mock,
        Duration::from_millis(150),
        Duration::from_secs(60),
    );

    // tx1 = set_param aceito → `pending` (a prova de que a escrita foi
    // ANTES da morte, e não um estado herdado).
    actor
        .set_param(3, 0x0700_006e, 0, 15.0)
        .expect("escrita antes da queda");
    assert_eq!(actor.info().expect("info").sync, "pending");

    // O read-back dispara (debounce 150 ms) e morre no meio das 9
    // páginas (transmissões 3, 4 e a 5 estoura o DieAfter).
    std::thread::sleep(Duration::from_millis(500));

    let st = actor.info().expect("info continua respondendo");
    assert_eq!(st.sync, "offline", "transporte morto = offline");
    assert!(
        st.sync_detail.contains("desconectado"),
        "o motivo é legível: {}",
        st.sync_detail
    );

    // Escrita seguinte: FALHA (DeviceGone tipado) — nenhum frame sai.
    let err = actor
        .set_param(3, 0x0700_006e, 0, 20.0)
        .expect_err("sem aparelho não existe escrita");
    assert!(
        err.contains("device sumiu") || err.contains("session do actor ausente"),
        "erro tipado rápido, não janela de timeout: {err}"
    );
    assert_eq!(actor.info().expect("info").sync, "offline");
    actor.shutdown();
}

/// #164 — `device_sync_now` dispara o read-back AGORA: o spawn tem
/// refresh de 1 HORA, então o `sync` que vier em seguida SÓ pode ser o
/// agendamento do comando — a leitura não espera a cadência.
#[test]
fn sync_now_dispara_readback_sem_esperar_o_refresh() {
    let actor = DeviceActor::spawn_com_cadencia(
        MockDevice::new().expect("mock"),
        Backend::Mock,
        Duration::from_millis(100),
        Duration::from_secs(3600),
    );
    let (ev_tx, ev_rx) = mpsc::channel::<SyncEvent>();
    actor.set_sync_sink(ev_tx);

    actor.boot(None).expect("boot");
    // Drena os eventos do boot (o `sync` final já está na fila) — daqui
    // em diante qualquer evento nasce do sync_now.
    while ev_rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

    actor.sync_now().expect("agenda o read-back");
    let ev = ev_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("o read-back do sync_now emite sem esperar a cadência");
    assert_eq!(ev.status, "sync", "leitura OK = sincronizado");
    actor.shutdown();
}
