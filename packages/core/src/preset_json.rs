//! preset_json — o `.prst` em JSON **versionado** e de ida e volta (issue #114).
//!
//! **O que o JSON resolve.** `.prst` é um container XML de 4 KB com um preset
//! inteiro achatado em atributos (`params_0`..`params_14` intercalados com
//! `x`/`y`). Dá para o aparelho usar e é impossível para um humano comparar:
//! `git diff` de dois `.prst` mostra uma linha gigante reescrita, sem dizer
//! qual knob mudou. Este módulo transforma o mesmo conteúdo em algo que diffa
//! por linha — um atributo por linha, um elemento por bloco.
//!
//! **Por que o JSON carrega LAYOUT, e não só a cadeia.** O `DoD` da #114 pede
//! `.prst → JSON → .prst` **byte-idêntico**. Um JSON puramente semântico
//! (cadeia de 9 slots + knobs) **não consegue** cumprir isso, e o motivo está
//! medido: o writer do Suite quebra linha em colunas que não seguem um limite
//! de largura — nos 3 arquivos de fábrica há linhas de 82 a 91 colunas que
//! quebram e linhas da mesma faixa que **não** quebram (`files/patches/*.prst`).
//! Não existe `L` tal que "quebre acima de L" explique todas as quebras; a
//! janela de aceitação é não-vazia, então **derivar a quebra é adivinhar** — e
//! adivinhar viola o R1.
//!
//! A saída não é escolher entre legível e fiel: é declarar as duas camadas. O
//! parser do `.prst` já trata layout como DADO (topo de [`crate::preset`]), e
//! o JSON apenas **publica esse dado** em vez de escondê-lo. O envelope tem,
//! então, duas leituras do mesmo conteúdo:
//!
//! - `raiz` — a árvore fiel (nome + atributos na ordem + quebras + filhos).
//!   É ela que reconstrói os bytes, e a ÚNICA coisa que a importação lê;
//! - as chaves são nomeadas e um atributo por linha, que é o que faz o diff
//!   por knob funcionar.
//!
//! O que a árvore deliberadamente **não** é: um DOM genérico serializado. Ela
//! não tem namespace, não tem schema XML e não tenta ser editável por
//! ferramenta externa. É o dialecto do `.prst`decomposto, e o `version` do
//! envelope é o que autoriza recusar.
//!
//! **A `version` recusa o futuro, e é o ponto.** Sem ela, um JSON de um build
//! mais novo seria importado por um mais velho, e cada campo que o velho não
//! conhece viraria uma perda SILENCIOSA — o tipo de bug que só aparece quando o
//! dono nota que o preset voltou diferente. [`from_json`] rejeita formato
//! alheio e versão diferente ANTES de olhar a árvore, com mensagem que diz qual
//! versão veio e qual este build lê.
//!
//! **Determinismo.** `serde_json` escreve os campos na ordem de declaração e os
//! vetores na ordem do arquivo, então `to_json` é puro: o mesmo documento dá o
//! mesmo texto. É isso que faz `.prst → JSON → .prst → JSON` ser idêntico sem
//! nenhuma ordenação artificial de chaves.

use serde::Deserialize;

use crate::preset::{Document, DocumentSnapshot, ElementSnapshot, Quebra};
use crate::ProtocolError;

/// Identificador do formato — se mudar, é outro tipo de arquivo.
pub const FORMATO: &str = "gp100.preset";

/// Versão DESTE envelope. Incrementar quando a forma do JSON mudar.
///
/// Não é a versão do `.prst` nem do firmware: é a do contrato de exportação.
/// O `.prst` não tem número de versão próprio (o `<preset_info>` traz firmware
/// e software, mas eles não descrevem o layout), então quem protege a
/// compatibilidade é esta constante.
pub const VERSAO: u32 = 1;

/// O envelope exportado — `format`/`version` primeiro, para o cabeçalho ser
/// legível sem descer a árvore.
///
/// Só `Deserialize`: a ESCRITA passa pelo writer controlado de [`to_json`], e
/// derivar `Serialize` aqui daria dois escritores para o mesmo formato —
/// exatamente o tipo de divergência silenciosa que o round-trip canônico
/// existe para não ter.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEnvelope {
    /// Sempre [`FORMATO`].
    pub format: String,
    /// Versão do envelope ([`VERSAO`]).
    pub version: u32,
    /// Bytes crus da declaração `<?xml …?>` do arquivo de origem.
    pub declaracao: String,
    /// A árvore fiel (layout incluído).
    pub raiz: NoJson,
    /// Whitespace após `</GP-100>` (o `\r\n` final do arquivo).
    pub depois: String,
}

/// Um elemento do `.prst` em JSON.
///
/// Os campos opcionais são OMITIDOS na escrita quando vazios (`quebras`,
/// `vazio`, `fechaAntes`, `filhos`) porque nos 3 arquivos de fábrica a maioria
/// dos nós é auto-fechada — sem isso haveria três campos vazios por elemento, e
/// a legibilidade que este módulo existe para dar sumiria no ruído. Na leitura
/// todos têm default, então omitir é sempre aceito.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoJson {
    /// Nome da tag (`GP-100`, `presets`, `Effect`, `ppIRInfo0`…).
    pub nome: String,
    /// Whitespace antes do `<` (o indent da profundidade, e o `\r\n`).
    pub antes: String,
    /// Atributos na ordem do arquivo: `[["ppName", "Blink OD"], …]`.
    ///
    /// Par e não objeto de propósito: um objeto JSON não tem ordem garantida,
    /// e a ordem dos atributos é layout (o writer reimprime nela).
    #[serde(default)]
    pub atributos: Vec<(String, String)>,
    /// Quebras de linha: `[[atributo, recuo], …]`, em ordem crescente.
    ///
    /// Compacto e não nomeado porque é a parte que só a máquina lê — está aqui
    /// porque é justamente o que NÃO se pode derivar (ver o topo do módulo).
    #[serde(default)]
    pub quebras: Vec<[usize; 2]>,
    /// `true` quando o elemento fecha na própria tag (`<x …/>`).
    #[serde(default)]
    pub vazio: bool,
    /// Whitespace antes de `</nome>` (ausente em elemento auto-fechado).
    #[serde(default)]
    pub fecha_antes: String,
    /// Filhos diretos, em ordem.
    #[serde(default)]
    pub filhos: Vec<NoJson>,
}

/// Escapa uma string como literal JSON (delega o escape ao `serde_json`, que é
/// quem conhece as regras — aspas, `\\`, controles, `\uXXXX`).
fn jstr(s: &str) -> String {
    // Serializar uma `&str` não falha em `serde_json` (o único erro possível é
    // de I/O, e um `String` na memória não tem). O fallback existe para não
    // deixar um panic de produção aqui se isso mudar em outra versão.
    serde_json::to_string(s).unwrap_or_else(|_| String::from("\"\""))
}

/// Serializa um documento `.prst` no JSON do envelope, com layout CONTROLADO.
///
/// **Por que não `to_string_pretty`, que já existe.** Ele quebra linha dentro de
/// todo array, então um par de atributo — a unidade de leitura deste formato —
/// virava quatro linhas:
///
/// ```text
///           [
///             "ppName",
///             "Blink OD"
///           ],
/// ```
///
/// Com 9 slots × ~20 atributos isso multiplica por quatro o tamanho do arquivo e
/// destrói a única coisa que a #114 pede do JSON: um `git diff` que mostra QUAL
/// knob mudou, em uma linha. Daí o writer daqui.
///
/// A forma é estável e determinística (nenhum `HashMap` no caminho):
///
/// - um par de atributo por linha — `["params_0", "23"],`;
/// - `quebras` em uma linha só, porque é traço de máquina;
/// - campos vazios omitidos, para o ruído não esconder o conteúdo.
///
/// O texto sai com indentação de 2 espaços e quebra `\n`; o `\r\n` do `.prst`
/// vive DENTRO dos valores (`antes`/`fechaAntes`/`depois`), como dado, e nunca
/// como formatação do JSON.
pub fn to_json(doc: &Document) -> Result<String, ProtocolError> {
    let snap = doc.snapshot();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"format\": {},\n", jstr(FORMATO)));
    out.push_str(&format!("  \"version\": {VERSAO},\n"));
    out.push_str(&format!("  \"declaracao\": {},\n", jstr(&snap.decl)));
    out.push_str(&format!("  \"depois\": {},\n", jstr(&snap.trailing)));
    out.push_str("  \"raiz\": ");
    escreve_no(&mut out, &no_from(&snap.root), 1);
    out.push('\n');
    out.push_str("}\n");
    Ok(out)
}

/// Escreve um nó no `out`, começando na posição ATUAL do cursor (o valor é
/// colocado por quem chama — é o que faz `"raiz": {` sair com um espaço só).
///
/// `nivel` é a coluna do PAR de chaves: os campos ficam em `nivel + 1` e os
/// itens de um array, em `nivel + 2` — é o que faz o recorte do bloco de um
/// `<Effect>` no JSON coincidir com o recorte do bloco no arquivo.
fn escreve_no(out: &mut String, no: &NoJson, nivel: usize) {
    let pad = "  ".repeat(nivel);
    let pad1 = "  ".repeat(nivel + 1);
    let pad2 = "  ".repeat(nivel + 2);

    // Campos na ordem em que se lê um elemento: identidade, posição no arquivo,
    // conteúdo, e por último os filhos.
    let mut campos: Vec<String> = Vec::new();
    campos.push(format!("\"nome\": {}", jstr(&no.nome)));
    campos.push(format!("\"antes\": {}", jstr(&no.antes)));
    if !no.atributos.is_empty() {
        let itens: Vec<String> = no
            .atributos
            .iter()
            .map(|(k, v)| format!("{pad2}[{}, {}]", jstr(k), jstr(v)))
            .collect();
        campos.push(format!("\"atributos\": [\n{}\n{pad1}]", itens.join(",\n")));
    }
    if !no.quebras.is_empty() {
        let itens: Vec<String> = no
            .quebras
            .iter()
            .map(|q| format!("[{}, {}]", q[0], q[1]))
            .collect();
        campos.push(format!("\"quebras\": [{}]", itens.join(", ")));
    }
    if no.vazio {
        campos.push("\"vazio\": true".to_string());
    }
    if !no.fecha_antes.is_empty() {
        campos.push(format!("\"fechaAntes\": {}", jstr(&no.fecha_antes)));
    }
    if !no.filhos.is_empty() {
        let mut itens: Vec<String> = Vec::with_capacity(no.filhos.len());
        for c in &no.filhos {
            // o item de array vive em `nivel + 2`: o recuo vai AQUI, porque
            // `escreve_no` escreve a partir do cursor.
            let mut s = String::new();
            s.push_str(&pad2);
            escreve_no(&mut s, c, nivel + 2);
            itens.push(s);
        }
        campos.push(format!("\"filhos\": [\n{}\n{pad1}]", itens.join(",\n")));
    }

    out.push_str("{\n");
    for (i, campo) in campos.iter().enumerate() {
        out.push_str(&pad1);
        out.push_str(campo);
        if i + 1 < campos.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&pad);
    out.push('}');
}

/// Lê o JSON do envelope e devolve o documento `.prst` correspondente.
///
/// Recusa **antes** de tocar a árvore:
/// - formato diferente de [`FORMATO`] (o JSON não é um preset nosso);
/// - `version` diferente de [`VERSAO`] (versão futura ou anterior).
///
/// # Erros
/// [`ProtocolError::InvalidShape`] com mensagem que diz o que veio e o que este
/// build lê. JSON que não é objeto, ou sem o cabeçalho, cai no mesmo erro — a
/// mensagem muda para não acusar "campo faltando" quando o arquivo é outro.
pub fn from_json(json: &str) -> Result<Document, ProtocolError> {
    let valor: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ProtocolError::InvalidShape {
            expected: "JSON válido do envelope gp100.preset".into(),
            got: e.to_string(),
        })?;

    // Cabeçalho PRIMEIRO: sem ele não se sabe nem se o arquivo é nosso. Ler o
    // envelope inteiro de uma vez daria "faltou o campo raiz" para qualquer
    // JSON que o dono arrastasse para a janela — mensagem errada para o caso
    // mais comum (o arquivo é outro).
    let formato = valor.get("format").and_then(|v| v.as_str());
    let versao = valor.get("version").and_then(|v| v.as_u64());
    match (formato, versao) {
        (Some(f), _) if f != FORMATO => {
            return Err(ProtocolError::InvalidShape {
                expected: format!("envelope com format = \"{FORMATO}\""),
                got: format!("format = \"{f}\""),
            })
        }
        (None, _) => {
            return Err(ProtocolError::InvalidShape {
                expected: format!("envelope do preset (format = \"{FORMATO}\")"),
                got: "JSON sem o campo format".into(),
            })
        }
        (_, None) => {
            return Err(ProtocolError::InvalidShape {
                expected: "envelope do preset (version)".into(),
                got: "JSON sem o campo version".into(),
            })
        }
        (Some(_), Some(v)) if v != VERSAO as u64 => {
            return Err(ProtocolError::InvalidShape {
                expected: format!("version = {VERSAO} (a que este build le)"),
                got: format!("version = {v}"),
            })
        }
        _ => {}
    }

    let envelope: PresetEnvelope =
        serde_json::from_value(valor).map_err(|e| ProtocolError::InvalidShape {
            expected: format!("envelope v{VERSAO} completo (declaracao, raiz, depois)"),
            got: e.to_string(),
        })?;

    Document::from_snapshot(&snapshot_from(envelope))
}

fn no_from(el: &ElementSnapshot) -> NoJson {
    NoJson {
        nome: el.name.clone(),
        antes: el.pre_ws.clone(),
        atributos: el.attrs.clone(),
        quebras: el
            .quebras
            .iter()
            .map(|q| [q.at_attr, q.cont_indent])
            .collect(),
        vazio: el.self_closing,
        fecha_antes: el.close_pre_ws.clone(),
        filhos: el.children.iter().map(no_from).collect(),
    }
}

/// Reconstrói o instantâneo a partir do envelope lido.
fn snapshot_from(env: PresetEnvelope) -> DocumentSnapshot {
    DocumentSnapshot {
        decl: env.declaracao,
        root: no_to(&env.raiz),
        trailing: env.depois,
    }
}

fn no_to(no: &NoJson) -> ElementSnapshot {
    ElementSnapshot {
        name: no.nome.clone(),
        pre_ws: no.antes.clone(),
        attrs: no.atributos.clone(),
        quebras: no
            .quebras
            .iter()
            .map(|q| Quebra {
                at_attr: q[0],
                cont_indent: q[1],
            })
            .collect(),
        self_closing: no.vazio,
        close_pre_ws: no.fecha_antes.clone(),
        children: no.filhos.iter().map(no_to).collect(),
    }
}
