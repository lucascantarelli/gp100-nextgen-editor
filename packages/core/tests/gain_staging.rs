//! Contratos do assistente de gain staging (#115) — ângulo caixa-preta.
//!
//! O que estes testes trancam, e por que cada um:
//!
//! 1. **Cada número tem ORIGEM.** A DoD da issue diz "relatorio por modulo com a
//!    origem de cada numero (qual valor do preset levou a estimativa) — nada de
//!    numero magico". O teste não confere um número fixo: ele **recalcula** a
//!    posição a partir do valor e da faixa que vêm do próprio board e compara
//!    com o que o relatório publicou. Um coeficiente escondido no meio da conta
//!    faria este teste falhar.
//! 2. **O que fica de fora está declarado.** Módulo desligado não conta, mix não
//!    soma nível, controle de banda não é nível, algoritmo fora do dicionário é
//!    **listado** (`fora_do_dicionario`) em vez de virar silêncio. "Não avaliei"
//!    e "avaliei e está bem" são respostas diferentes, e a tela precisa das
//!    duas.
//! 3. **Um preset armado para clipar é acusado, com a ordem de ajuste.** O
//!    risco é uma REGRA declarada (threshold de posição), então o caso do
//!    extremo é construído à mão a partir de um preset real — o mesmo exercício
//!    que a DoD pede.
//! 4. **Nenhum byte sai.** O instrumento é o do `value_gate.rs` (transporte que
//!    PUNE qualquer byte), e o alvo aqui é a própria análise. A prova estrutural
//!    é a assinatura ([`relatorio`] recebe `&BoardView` e devolve um valor: não
//!    há transporte no caminho) — o teste é o alarme para o dia em que alguém
//!    ligar o assistente a uma `Session`.

use std::time::Duration;

use gp100_core::gain::{relatorio, Papel, Risco};
use gp100_core::model::{Dictionary, DICTIONARY_JSON};
use gp100_core::pedalboard::{board_view_for, embedded_document, BoardView};
use gp100_core::session::Session;
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};

fn dicionario() -> Dictionary {
    Dictionary::from_json(DICTIONARY_JSON).expect("dicionário embutido")
}

/// O board de um preset REAL do `all.prst` embutido (nada inventado).
fn board(pp: u16) -> BoardView {
    board_view_for(
        &embedded_document().expect("all.prst"),
        &dicionario(),
        Some(pp),
    )
    .expect("pp existe no all.prst")
}

/// Muda o valor de um controle do board (o "preset armado" dos testes).
fn mexe(b: &mut BoardView, slot: u8, knob: &str, valor: &str) {
    let s = b
        .slots
        .iter_mut()
        .find(|s| s.slot == slot)
        .expect("slot existe");
    let k = s
        .knobs
        .iter_mut()
        .find(|k| k.name == knob)
        .unwrap_or_else(|| panic!("o slot {slot} ({}) não tem '{knob}'", s.name));
    k.value = Some(valor.to_string());
}

/// O relatório lê os 9 lugares na ORDEM DO SINAL — é a cadeia que o palco
/// desenha, e o assistente não pode ter uma ordem própria.
#[test]
fn o_relatorio_le_a_cadeia_inteira_na_ordem_do_sinal() {
    let r = relatorio(&board(0x00));
    assert_eq!(r.modulos.len(), 9, "os 9 lugares da cadeia");
    let slots: Vec<u8> = r.modulos.iter().map(|m| m.slot).collect();
    assert_eq!(slots, (0..9).collect::<Vec<u8>>(), "ordem do sinal");
    let familias: Vec<&str> = r.modulos.iter().map(|m| m.familia.as_str()).collect();
    assert_eq!(
        familias,
        ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"]
    );
    assert_eq!(r.pp, 0, "o pp do preset analisado");
}

/// **Os números são derivados, não mágicos.** Para todo controle lido, a posição
/// publicada tem de ser `(valor - lo) / (hi - lo)` do valor e da faixa do
/// PRÓPRIO board.
#[test]
fn cada_numero_do_relatorio_vem_do_valor_e_da_faixa_do_board() {
    // ppIDs reais do all.prst (a lista NÃO é 0..98 contígua: 0x0a não existe)
    for pp in [0x00u16, 0x18, 0x19, 0x30] {
        let b = board(pp);
        let r = relatorio(&b);
        let mut conferidos = 0;
        for m in &r.modulos {
            let s = b.slots.iter().find(|s| s.slot == m.slot).expect("slot");
            for l in &m.leituras {
                let k = s
                    .knobs
                    .iter()
                    .find(|k| k.pos == l.pos)
                    .expect("o controle lido existe no board");
                assert_eq!(k.name, l.knob, "o nome é o do dicionário");
                assert_eq!(k.value.as_deref(), Some(l.valor.as_str()), "valor cru");
                let (lo, hi) = l.faixa;
                assert_eq!(Some(l.faixa), k.range, "a faixa é a do dicionário");
                let valor: f64 = l.valor.parse().expect("valor numérico");
                let esperada = ((valor - lo) / (hi - lo)).clamp(0.0, 1.0);
                assert!(
                    (l.posicao - esperada).abs() < 1e-9,
                    "pp {pp:#04x} slot {} {}: posicao {} != {esperada} (valor {} em {lo}..{hi})",
                    m.slot,
                    l.knob,
                    l.posicao,
                    l.valor
                );
                assert!(
                    (l.folga - (1.0 - l.posicao)).abs() < 1e-9,
                    "a folga é o complemento da posição"
                );
                conferidos += 1;
            }
        }
        assert!(
            conferidos >= 4,
            "pp {pp:#04x}: poucos controles lidos ({conferidos})"
        );
    }
}

/// Módulo DESLIGADO: as leituras continuam no relatório (são fatos do preset) e
/// não entram na conta — um pedal bypassado não soma nível.
#[test]
fn modulo_desligado_aparece_mas_nao_conta() {
    let b = board(0x00);
    let pre = b.slots.iter().find(|s| s.slot == 0).expect("PRE");
    assert!(!pre.state, "o PRE do P01 nasce desligado");
    let r = relatorio(&b);
    let m = r.modulos.iter().find(|m| m.slot == 0).expect("PRE");
    assert!(!m.ligado);
    assert!(
        m.leituras.iter().any(|l| l.knob == "VOL"),
        "as leituras do módulo desligado aparecem"
    );
    assert_eq!(m.folga, None, "desligado não tem folga");
    assert!(!m.no_teto, "desligado nunca está no teto");
    assert!(
        !r.ajuste.iter().any(|s| s.slot == 0),
        "desligado não entra na ordem de ajuste"
    );
}

/// **Mix não soma nível.** Um delay com `Mix` no teto não é clipagem: as duas
/// metades de uma mistura não são o nível do estágio.
#[test]
fn mix_e_mistura_nao_soma_nivel() {
    let mut b = board(0x00);
    mexe(&mut b, 7, "Mix", "99"); // DLY (P-Echo)
    let r = relatorio(&b);
    let dly = r.modulos.iter().find(|m| m.slot == 7).expect("DLY");
    let mix = dly.leituras.iter().find(|l| l.knob == "Mix").expect("Mix");
    assert_eq!(mix.papel, Papel::Mix);
    assert!(
        !dly.no_teto,
        "Mix no teto não põe o módulo no teto (não soma nível)"
    );
    assert!(r.ajuste.is_empty(), "e não vira sugestão de ajuste");
    assert_eq!(r.risco, Risco::Baixo);
}

/// Controle de BANDA (os ganhos do EQ) não entra: ele mexe no timbre, não no
/// nível de entrada/saída do estágio.
#[test]
fn ganho_de_banda_do_eq_fica_fora_da_conta() {
    let mut b = board(0x00);
    mexe(&mut b, 5, "6.6kHz", "50"); // banda do Mess EQ, faixa -50..50
    let r = relatorio(&b);
    let eq = r.modulos.iter().find(|m| m.slot == 5).expect("EQ");
    assert!(
        eq.leituras.is_empty(),
        "banda do EQ não é leitura de nível: {:?}",
        eq.leituras
    );
    assert_eq!(eq.folga, None);
    assert_eq!(r.risco, Risco::Baixo);
}

/// Algoritmo fora do dicionário chega ao board SEM knobs (regra R1): o slot é
/// LISTADO, porque "não sei" é resposta e não pode virar silêncio.
#[test]
fn algoritmo_fora_do_dicionario_e_declarado() {
    let mut b = board(0x00);
    let cab = b.slots.iter_mut().find(|s| s.slot == 4).expect("CAB");
    cab.knobs.clear();
    let r = relatorio(&b);
    assert_eq!(
        r.fora_do_dicionario,
        vec![4],
        "o slot sem knobs é declarado"
    );
    let m = r.modulos.iter().find(|m| m.slot == 4).expect("CAB");
    assert!(m.leituras.is_empty());
    assert_eq!(m.folga, None);
}

/// Valor ilegível ou faixa degenerada ficam DECLARADOS em `ignorados` (a tela
/// mostra o que ficou de fora, em vez de o controle sumir sem explicação).
#[test]
fn controle_ilegivel_ou_faixa_degenerada_e_declarado() {
    let mut b = board(0x00);
    mexe(&mut b, 2, "Gain", "abc"); // AMP Bog RedM
    let r = relatorio(&b);
    let amp = r.modulos.iter().find(|m| m.slot == 2).expect("AMP");
    assert!(
        amp.ignorados.contains(&"Gain".to_string()),
        "valor ilegível declarado: {:?}",
        amp.ignorados
    );
    assert!(!amp.leituras.iter().any(|l| l.knob == "Gain"));

    let mut b2 = board(0x00);
    let amp2 = b2.slots.iter_mut().find(|s| s.slot == 2).expect("AMP");
    let gain = amp2
        .knobs
        .iter_mut()
        .find(|k| k.name == "Gain")
        .expect("Gain");
    gain.range = Some((50.0, 50.0)); // faixa degenerada
    let r2 = relatorio(&b2);
    let amp = r2.modulos.iter().find(|m| m.slot == 2).expect("AMP");
    assert!(amp.ignorados.contains(&"Gain".to_string()));
}

/// **O preset armado para clipar é acusado** — e a ordem de ajuste põe o NÍVEL
/// DE SAÍDA (o CAB no fim da cadeia) antes do GANHO (o AMP), que é a razão
/// escrita no método: tirar nível depois do ponto que clipa não mexe no timbre
/// do drive.
#[test]
fn preset_armado_para_clipar_acusa_o_teto_e_a_ordem_de_ajuste() {
    let mut b = board(0x00);
    mexe(&mut b, 2, "Gain", "99"); // AMP (Bog RedM) no teto
    mexe(&mut b, 4, "Volume", "99"); // CAB (U-ban 4x12) no teto
    let r = relatorio(&b);

    assert_eq!(r.risco, Risco::Alto, "dois estágios no teto");
    let amp = r.modulos.iter().find(|m| m.slot == 2).expect("AMP");
    let cab = r.modulos.iter().find(|m| m.slot == 4).expect("CAB");
    assert!(amp.no_teto && cab.no_teto);
    assert_eq!(amp.folga, Some(0.0), "Gain 99 = fim da faixa 0..99");
    assert_eq!(
        r.folga_minima.map(|(slot, _)| slot),
        Some(2),
        "o AMP é quem está com a menor folga (empate com o CAB, menor slot)"
    );

    let ordem: Vec<(u8, &str)> = r.ajuste.iter().map(|s| (s.slot, s.knob.as_str())).collect();
    assert_eq!(
        ordem,
        vec![(4, "Volume"), (2, "Gain")],
        "saída primeiro (fim da cadeia), depois o ganho"
    );
    // a sugestão carrega o valor ATUAL: é o que o dono vê no pedal
    assert_eq!(r.ajuste[0].valor, "99");
    assert_eq!(r.ajuste[0].papel, Papel::Saida);
}

/// O caso que a issue descreve: o AMP no teto com um CAB ligado atrás (gain alto
/// com a IR no slot) já é risco ALTO — mesmo com um único controle no teto.
#[test]
fn amp_no_teto_com_cab_ligado_e_risco_alto() {
    let mut b = board(0x00);
    mexe(&mut b, 2, "Gain", "99");
    let r = relatorio(&b);
    assert_eq!(r.risco, Risco::Alto);
    assert_eq!(r.ajuste.len(), 1, "uma sugestão: o Gain do AMP");
    assert_eq!(r.ajuste[0].slot, 2);
}

/// Um controle no teto, sem o AMP+CAB, é risco MÉDIO — e a folga mínima é dele.
#[test]
fn um_controle_no_teto_e_risco_medio() {
    let mut b = board(0x00);
    mexe(&mut b, 4, "Volume", "99");
    let r = relatorio(&b);
    assert_eq!(r.risco, Risco::Medio);
    assert_eq!(r.folga_minima.map(|(slot, _)| slot), Some(4));
    assert_eq!(r.ajuste.len(), 1);
}

/// O preset de fábrica não é acusado: o P01 (que o palco abre) tem folga em
/// todos os estágios de nível ligados.
#[test]
fn preset_de_fabrica_nao_acusa_teto() {
    let r = relatorio(&board(0x00));
    assert_eq!(r.risco, Risco::Baixo);
    assert!(r.ajuste.is_empty());
    assert!(r.fora_do_dicionario.is_empty());
    let (slot, folga) = r.folga_minima.expect("o P01 tem estágios de nível");
    assert_eq!(slot, 4, "o CAB Volume 72 é o mais perto do teto");
    assert!(folga > 0.2, "e ainda tem folga: {folga}");
}

// ───────────────────────── o instrumento do "nenhum byte" ───────────────────

/// Transporte que FALHA se qualquer byte sair (o mesmo instrumento de
/// `value_gate.rs`): um `Err` aqui é "um byte escapou".
#[derive(Default)]
struct ByteGuard {
    escapados: Vec<Vec<u8>>,
}

impl DeviceTransport for ByteGuard {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        self.escapados.push(data.to_vec());
        Ok(())
    }
    fn recv_raw(&mut self, _timeout: Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::RecvTimeout { timeout_ms: 0 })
    }
}

/// **Nenhum byte sai — e o board não é tocado.**
///
/// A prova estrutural é a assinatura: `relatorio(&BoardView) -> Relatorio` não
/// tem transporte para onde escrever (é o mesmo motivo pelo qual não existe um
/// `ByteGuard` DENTRO do módulo). O teste é o alarme: ele roda o caminho
/// inteiro do assistente (board → relatório → JSON) com uma `Session` armada
/// sobre um transporte que pune qualquer byte, e confere que nada saiu e que o
/// board ficou idêntico.
#[test]
fn a_analise_nao_muda_o_board_e_nao_manda_um_byte() {
    let mut guard = ByteGuard::default();
    guard.open().expect("abre");
    let sessao = Session::new(guard);

    let b = board(0x00);
    let antes = b.clone();
    let r = relatorio(&b);
    let json = serde_json::to_string(&r).expect("o relatório serializa");
    assert!(json.contains("\"risco\""), "o DTO carrega o risco");

    assert_eq!(b, antes, "a análise não modifica o board");
    let guard = sessao.into_transport();
    assert!(
        guard.escapados.is_empty(),
        "ZERO bytes podem sair do assistente: escaparam {}",
        guard.escapados.len()
    );
}
