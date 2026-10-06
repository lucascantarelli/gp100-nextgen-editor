//! gain — o assistente de gain staging: headroom e clipagem **estimada** por
//! módulo (#115).
//!
//! **O que este módulo é, e o que ele NÃO é.** Ele lê o MESMO board que o palco
//! desenha (`pedalboard::BoardView` — a projeção do preset pelo dicionário) e
//! diz, por módulo da cadeia, onde os controles de ganho/nível estão na faixa
//! declarada e onde isso se acumula. Ele **não mede áudio** e **não escreve**:
//! não há device, nem transporte, nem fio neste caminho — a assinatura
//! [`relatorio`] recebe `&BoardView` e devolve um valor, então o compilador já
//! proíbe o assistente de mandar byte para o aparelho (é o que a DoD da #115
//! pede, e o porquê de não existir o `ByteGuard` do `value_gate.rs` aqui: não há
//! por onde um byte sair).
//!
//! **Por que a estimativa é POSIÇÃO, e não decibel.** O dicionário
//! (`analysis/parameters.json`) traz, de cada controle, a FAIXA física
//! (`min`/`max`) e o valor atual — nunca a resposta em dB do pedal. Um
//! "Gain: 70" quer dizer coisas diferentes num overdrive e num amp, e o quanto
//! cada estágio ganha não está medido em lugar nenhum. Escrever "+12 dB" seria
//! inventar precisão — o oposto do que a issue pede ("os coeficientes são
//! estimativas declaradas, com o método escrito; um número que parece preciso e
//! não é é pior do que um número que só avisa").
//!
//! O número que este módulo publica é, então, a **posição do controle na
//! própria faixa** (`0.0` = piso, `1.0` = teto), que é aritmética sobre o
//! dicionário e o preset — auditável, sem coeficiente nenhum. A "folga"
//! (*headroom*) é `1 - posição`: quanto ainda sobra daquele controle. E a
//! **limitação** viaja junto do relatório (campo [`Metodo::limitacao`]) para
//! aparecer na tela, não só no documento: posição no controle não é nível de
//! sinal medido, e o assistente não conhece nem a guitarra nem a IR.
//!
//! **Quem entra na conta, e quem fica de fora** (tudo declarado em
//! [`Metodo`], para a tela poder mostrar):
//! - controles de **Ganho** (`Gain`, `EQ Gain`) — somam nível;
//! - controles de **Saída** (`Volume`, `Master`, `Output`, `Level`) — dosam o
//!   nível de saída; no teto, não têm mais para onde ir;
//! - controles de **Mix** (`Mix`, `Mix A`, `Mix B`, `Blend`) — mistura
//!   seco/molhado, que NÃO soma nível: ficam de fora, e o relatório diz que
//!   ficaram (é a diferença entre "não avaliei" e "avaliei e está bem");
//! - **switch/combox** e valores não numéricos — ficam de fora (não têm
//!   posição contínua);
//! - módulos **desligados** — o preset os tem, mas um pedal bypassado não soma
//!   nível: as leituras aparecem no relatório e não entram nem no risco nem na
//!   ordem de ajuste;
//! - algoritmos **fora do dicionário** — o board chega sem knobs (regra R1 do
//!   `pedalboard.rs`): o slot é listado em [`Relatorio::fora_do_dicionario`],
//!   porque "não sei" é uma resposta e não pode virar silêncio.

use serde::Serialize;

use crate::pedalboard::{BoardView, Family};

/// A partir de que posição o controle está "no teto" (declarado).
///
/// 95% e não 100%: o valor de fio é inteiro numa faixa 0..99, e um Gain em 99
/// já é o fim da linha — exigir o exato 99 faria o aviso sumir por um clique.
pub const LIMIAR_TETO: f64 = 0.95;

/// O papel de um controle na conta de nível.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Papel {
    /// Ganho de entrada do estágio (soma nível).
    Ganho,
    /// Nível de saída do estágio (dosa o que sai dele).
    Saida,
    /// Mistura seco/molhado — NÃO soma nível; fica de fora da conta.
    Mix,
}

/// Um controle do preset que entrou na conta, com a origem do número.
///
/// `origem` é literal: o nome do controle, a posição no envelope, o valor cru
/// do preset e a faixa de onde a posição foi calculada. É a resposta para
/// "qual valor do preset levou a esta estimativa" (DoD da #115).
///
/// Os DTOs deste módulo são `camelCase` no fio, como os de `preset_json` — o
/// contrato com a UI é o mesmo, e o `serde` fica no core (a api é casca fina).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leitura {
    /// Nome do controle no dicionário (ex.: "Gain").
    pub knob: String,
    /// Posição no envelope (`params_N` do `.prst`).
    pub pos: u8,
    /// Valor atual cru do preset (a representação do arquivo).
    pub valor: String,
    /// Papel declarado na conta.
    pub papel: Papel,
    /// Faixa do dicionário de onde a posição foi calculada (`lo <= hi`).
    pub faixa: (f64, f64),
    /// Posição na faixa, `0.0`..`= 1.0`.
    pub posicao: f64,
    /// `1 - posicao` — o quanto ainda sobra do controle.
    pub folga: f64,
}

/// Um módulo da cadeia (TODOS os 9, ligados ou não).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Modulo {
    /// Posição na cadeia (`x` do arquivo, 0..8 = ordem do sinal).
    pub slot: u8,
    /// Rótulo da família (PRE/DST/AMP/NR/CAB/EQ/MOD/DLY/RVB).
    pub familia: String,
    /// Nome do algoritmo (ex.: "Bog RedM").
    pub nome: String,
    /// Ligado? Desligado não conta para risco nem para a ordem.
    pub ligado: bool,
    /// Leituras de ganho/saída/mix DESTE módulo (na ordem do `pos`).
    pub leituras: Vec<Leitura>,
    /// Controles de ganho/saída ignorados (switch/combox ou valor ilegível).
    pub ignorados: Vec<String>,
    /// A MENOR folga entre os controles de ganho/saída — o controle mais perto
    /// do teto é quem manda no módulo. `None` = sem controle de ganho, ou
    /// módulo desligado.
    pub folga: Option<f64>,
    /// Algum controle de ganho/saída está no teto ([`LIMIAR_TETO`])?
    pub no_teto: bool,
}

/// Risco declarado da cadeia inteira.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risco {
    /// Nenhum controle de ganho/saída no teto.
    Baixo,
    /// Um módulo no teto.
    Medio,
    /// Dois ou mais — ou o AMP no teto com um CAB ligado atrás (o caso que a
    /// issue descreve: gain alto com a IR no slot).
    Alto,
}

/// Um passo sugerido de ajuste — o que mexer, e POR QUE ele vem nesta ordem.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sugestao {
    /// Módulo da cadeia.
    pub slot: u8,
    /// Rótulo da família.
    pub familia: String,
    /// Nome do controle.
    pub knob: String,
    /// Posição do controle.
    pub pos: u8,
    /// Valor atual (o que está no preset).
    pub valor: String,
    /// Papel do controle (decide quem vem primeiro — ver [`Metodo::ordem`]).
    pub papel: Papel,
}

/// O método, DECLARADO — o que a tela mostra junto do relatório.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metodo {
    /// Nomes declarados como ganho de entrada.
    pub nomes_de_ganho: Vec<&'static str>,
    /// Nomes declarados como nível de saída.
    pub nomes_de_saida: Vec<&'static str>,
    /// Nomes declarados como mistura (fora da conta).
    pub nomes_de_mix: Vec<&'static str>,
    /// A partir de que posição o controle conta como no teto.
    pub limiar_teto: f64,
    /// A ORDEM do ajuste, escrita.
    pub ordem: &'static str,
    /// A LIMITAÇÃO, escrita — o que o número não é.
    pub limitacao: &'static str,
}

/// O relatório do assistente para um preset.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relatorio {
    /// pp do preset analisado.
    pub pp: u16,
    /// Nome do preset.
    pub nome: String,
    /// Os 9 módulos, na ordem do sinal.
    pub modulos: Vec<Modulo>,
    /// Slots cujo algoritmo não está no dicionário (board sem knobs — R1).
    pub fora_do_dicionario: Vec<u8>,
    /// Risco declarado da cadeia.
    pub risco: Risco,
    /// A MENOR folga da cadeia (e de quem ela é) — onde falta headroom.
    pub folga_minima: Option<(u8, f64)>,
    /// A ordem de ajuste sugerida (vazia = nada no teto).
    pub ajuste: Vec<Sugestao>,
    /// O método e a limitação, para a tela.
    pub metodo: Metodo,
}

/// Os nomes declarados como ganho de entrada.
///
/// A lista saiu do DICIONÁRIO, não da imaginação: `analysis/parameters.json`
/// tem 105 nomes distintos, e só `Gain` (57 algoritmos) é ganho de entrada sem
/// ambiguidade. `Sustain` (compressor) e `Fuzz`/`Bias` (caráter de clipagem)
/// ficam de fora: mexem no timbre, não no nível que chega ao estágio seguinte.
const NOMES_DE_GANHO: &[&str] = &["Gain"];

/// Os nomes declarados como nível de saída.
///
/// **`VOL` e `Volume` estão os dois na lista de propósito** — o dicionário usa
/// as duas grafias (42 + 2 + 2 + 2 controles `VOL`/`VOL `/`VOL 1`/`VOL 2`
/// contra 65 `Volume`), e `H-Vol`/`L-Vol` são os níveis das duas vozes de um
/// pedal de harmonia: são a SAÍDA daquele estágio, mesmo nomeadas por voz.
const NOMES_DE_SAIDA: &[&str] = &[
    "VOL", "VOL 1", "VOL 2", "Volume", "Master", "Output", "Level", "H-Vol", "L-Vol",
];

/// Os nomes declarados como mistura seco/molhado (fora da conta).
///
/// `Wet`/`Dry` entram aqui pelo mesmo motivo de `Mix`: são as duas metades de
/// uma mistura, e mistura não soma nível.
const NOMES_DE_MIX: &[&str] = &["Mix", "Mix A", "Mix B", "Blend", "Wet", "Dry"];

/// A ordem do ajuste, escrita (vai no relatório e na tela).
const ORDEM: &str = "Primeiro os NÍVEIS de saída que estão no teto, do fim da cadeia para o começo (tirar nível depois do ponto que clipa não mexe no timbre do drive); depois os GANHOS que estão no teto, do começo para o fim (o ganho de entrada é o que os estágios seguintes amplificam).";

/// A limitação, escrita (vai no relatório e na tela).
const LIMITACAO: &str = "A estimativa é a POSIÇÃO de cada controle na faixa do dicionário — não é nível de sinal medido. O assistente não conhece o nível da guitarra, a resposta em dB de cada pedal, nem a IR no slot: ele aponta onde o preset está no teto e onde isso se acumula na cadeia. Controles de banda (os ganhos do EQ) também ficam fora: eles mexem no timbre, não no nível de entrada/saída do estágio.";

/// O papel declarado de um controle pelo nome (case-insensitive).
fn papel_de(nome: &str) -> Option<Papel> {
    let n = nome.trim();
    if NOMES_DE_GANHO.iter().any(|g| g.eq_ignore_ascii_case(n)) {
        return Some(Papel::Ganho);
    }
    if NOMES_DE_SAIDA.iter().any(|g| g.eq_ignore_ascii_case(n)) {
        return Some(Papel::Saida);
    }
    if NOMES_DE_MIX.iter().any(|g| g.eq_ignore_ascii_case(n)) {
        return Some(Papel::Mix);
    }
    None
}

/// Lê os controles de um módulo. Devolve as leituras (ordem do `pos`) e os
/// nomes ignorados — switch/combox e valor ilegível ficam de fora, declarados.
fn le_controles(slot: &crate::pedalboard::SlotSpec) -> (Vec<Leitura>, Vec<String>) {
    let mut leituras = Vec::new();
    let mut ignorados = Vec::new();
    for k in &slot.knobs {
        let Some(papel) = papel_de(&k.name) else {
            continue; // não é controle de nível: nem entra na lista de ignorados
        };
        // Só `knob` contínuo tem posição; switch/combox de mesmo nome (não há
        // hoje, mas o dicionário é insumo) não teriam.
        if k.kind != "knob" {
            ignorados.push(k.name.clone());
            continue;
        }
        let (Some(faixa), Some(valor)) = (k.range, k.value.as_deref()) else {
            ignorados.push(k.name.clone());
            continue;
        };
        let Ok(v) = valor.trim().parse::<f64>() else {
            ignorados.push(k.name.clone());
            continue;
        };
        let (lo, hi) = faixa;
        if hi <= lo {
            // Faixa degenerada: sem posição possível (mesma regra do
            // `Control::range` do dicionário, que já normaliza lo <= hi).
            ignorados.push(k.name.clone());
            continue;
        }
        let posicao = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
        leituras.push(Leitura {
            knob: k.name.clone(),
            pos: k.pos,
            valor: valor.to_string(),
            papel,
            faixa,
            posicao,
            folga: 1.0 - posicao,
        });
    }
    leituras.sort_by_key(|l| l.pos);
    (leituras, ignorados)
}

/// O relatório do assistente para o board indicado.
///
/// **Puro por construção:** recebe `&BoardView` e devolve um valor. Não há
/// transporte, sessão ou device neste caminho — nenhum byte pode sair daqui.
pub fn relatorio(board: &BoardView) -> Relatorio {
    let mut modulos = Vec::with_capacity(board.slots.len());
    let mut fora = Vec::new();

    for s in &board.slots {
        let tem_knobs = !s.knobs.is_empty();
        if !tem_knobs {
            fora.push(s.slot);
        }
        let (leituras, ignorados) = le_controles(s);
        // Só módulo LIGADO conta — e só quem tem controle de ganho/saída
        // define uma folga (mix não soma nível).
        let de_nivel: Vec<&Leitura> = leituras.iter().filter(|l| l.papel != Papel::Mix).collect();
        let folga = if s.state && !de_nivel.is_empty() {
            Some(
                de_nivel
                    .iter()
                    .map(|l| l.folga)
                    .fold(f64::INFINITY, f64::min),
            )
        } else {
            None
        };
        let no_teto = folga.is_some_and(|f| 1.0 - f >= LIMIAR_TETO);
        modulos.push(Modulo {
            slot: s.slot,
            familia: rotulo(s.family),
            nome: s.name.clone(),
            ligado: s.state,
            leituras,
            ignorados,
            folga,
            no_teto,
        });
    }
    modulos.sort_by_key(|m| m.slot);

    // Onde falta headroom: a menor folga da cadeia e de quem ela é.
    let folga_minima = modulos
        .iter()
        .filter_map(|m| m.folga.map(|f| (m.slot, f)))
        .min_by(|a, b| a.1.total_cmp(&b.1));

    // Quem está no teto, separado por papel.
    let no_teto: Vec<&Leitura> = modulos
        .iter()
        .filter(|m| m.no_teto)
        .flat_map(|m| m.leituras.iter().filter(|l| l.papel != Papel::Mix))
        .filter(|l| 1.0 - l.folga >= LIMIAR_TETO)
        .collect();
    let ganhos_no_teto = no_teto.iter().filter(|l| l.papel == Papel::Ganho).count();

    // O caso da issue: o AMP no teto com um CAB ligado atrás (o gain alto com a
    // IR no slot). `AMP` = família Amp; `CAB` = família Cab.
    let amp_no_teto = modulos.iter().any(|m| {
        m.no_teto && m.familia == "AMP" && m.leituras.iter().any(|l| l.papel == Papel::Ganho)
    });
    let cab_ligado = board
        .slots
        .iter()
        .any(|s| s.state && matches!(s.family, Family::Cab));

    let risco = if ganhos_no_teto >= 2 || (amp_no_teto && cab_ligado) {
        Risco::Alto
    } else if !no_teto.is_empty() {
        Risco::Medio
    } else {
        Risco::Baixo
    };

    let ajuste = ordem_de_ajuste(&modulos);

    Relatorio {
        pp: board.pp,
        nome: board.name.clone(),
        modulos,
        fora_do_dicionario: fora,
        risco,
        folga_minima,
        ajuste,
        metodo: Metodo {
            nomes_de_ganho: NOMES_DE_GANHO.to_vec(),
            nomes_de_saida: NOMES_DE_SAIDA.to_vec(),
            nomes_de_mix: NOMES_DE_MIX.to_vec(),
            limiar_teto: LIMIAR_TETO,
            ordem: ORDEM,
            limitacao: LIMITACAO,
        },
    }
}

/// A ordem sugerida: saída no teto (do fim da cadeia para o começo) e depois
/// ganho no teto (do começo para o fim) — a razão está em [`ORDEM`].
fn ordem_de_ajuste(modulos: &[Modulo]) -> Vec<Sugestao> {
    let mut saida: Vec<Sugestao> = Vec::new();
    let mut ganho: Vec<Sugestao> = Vec::new();
    for m in modulos.iter().filter(|m| m.no_teto) {
        for l in m.leituras.iter().filter(|l| 1.0 - l.folga >= LIMIAR_TETO) {
            let s = Sugestao {
                slot: m.slot,
                familia: m.familia.clone(),
                knob: l.knob.clone(),
                pos: l.pos,
                valor: l.valor.clone(),
                papel: l.papel,
            };
            match l.papel {
                Papel::Saida => saida.push(s),
                Papel::Ganho => ganho.push(s),
                Papel::Mix => {}
            }
        }
    }
    // Saída: do FIM da cadeia para o começo. Ganho: do começo para o fim.
    saida.sort_by_key(|s| std::cmp::Reverse(s.slot));
    ganho.sort_by_key(|s| s.slot);
    saida.extend(ganho);
    saida
}

/// Rótulo da família (mesma string do `rename_all = "UPPERCASE"` do enum).
fn rotulo(f: Family) -> String {
    match f {
        Family::Pre => "PRE",
        Family::Dst => "DST",
        Family::Amp => "AMP",
        Family::Nr => "NR",
        Family::Cab => "CAB",
        Family::Eq => "EQ",
        Family::Mod => "MOD",
        Family::Dly => "DLY",
        Family::Rvb => "RVB",
    }
    .to_string()
}
