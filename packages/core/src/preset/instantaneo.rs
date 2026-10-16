//! o INSTANTÂNEO do layout (o `.prst` como DADO) e a reconstrução por ele.

use super::elemento::Attr;
use super::shape;
use super::Document;
use super::{Element, LineBreak};
use crate::ProtocolError;

// ------------------------------------------------------- instantâneo (layout)
//
// **Por que existe este seam.** O layout do `.prst` é DADO, não algoritmo (ver
// o topo do módulo): o Suite quebra linha em colunas que **não** seguem um
// limite de largura — medido nos 3 arquivos reais, há linhas de 82 a 91
// colunas que quebram e linhas da mesma faixa que não quebram. Derivar a
// quebra seria CHUTAR, e chute viola o R1.
//
// Consequência para a exportação em JSON (#114): um JSON puramente semântico
// (cadeia de 9 slots + knobs) **não** consegue reproduzir os bytes de volta,
// porque não carrega a quebra. O instantâneo abaixo é o que dá ao JSON o
// direito de prometer round-trip byte-idêntico sem adivinhar nada: ele expõe o
// layout como dado explícito, mantendo os campos de [`Element`] privados (o
// instantâneo é a ÚNICA porta, e construir por ele não pode violar invariante).

/// Uma quebra de linha registrada: o atributo de índice `at_attr` começa uma
/// linha nova indentada com `cont_indent` espaços.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quebra {
    /// Índice do atributo que inicia a linha (>= 1; o 1º fica na linha do `<`).
    pub at_attr: usize,
    /// Espaços de indentação da linha de continuação.
    pub cont_indent: usize,
}

/// Instantâneo COMPLETO de um elemento, **layout incluído**.
///
/// Espelha [`Element`] campo a campo e é a representação que a exportação
/// serializa. Não é um "modelo paralelo": é uma projeção fiel, e reconstruir
/// por ele devolve os mesmos bytes (provado nos 3 arquivos de fábrica).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementSnapshot {
    /// Nome da tag (`GP-100`, `presets`, `Effect`, `ppIRInfo0`…).
    pub name: String,
    /// Whitespace bruto antes do `<` (inclui `\r\n` e o indent).
    pub pre_ws: String,
    /// Atributos na ORDEM do arquivo — a ordem é layout, não conveniência.
    pub attrs: Vec<(String, String)>,
    /// Quebras de linha registradas, em ordem crescente de `at_attr`.
    pub quebras: Vec<Quebra>,
    /// Elemento fechado na própria tag (`<x .../>`).
    pub self_closing: bool,
    /// Whitespace antes de `</nome>`; vazio quando `self_closing`.
    pub close_pre_ws: String,
    /// Filhos diretos, em ordem.
    pub children: Vec<ElementSnapshot>,
}

/// Instantâneo do documento inteiro: declaração, raiz e whitespace final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSnapshot {
    /// Bytes crus de `<?xml …?>`.
    pub decl: String,
    /// Raiz `<GP-100>` (o `pre_ws` dela inclui a linha em branco pós-declaração).
    pub root: ElementSnapshot,
    /// Whitespace após `</GP-100>`.
    pub trailing: String,
}

impl Element {
    /// Instantâneo fiel deste elemento, layout incluído (recursivo).
    pub fn snapshot(&self) -> ElementSnapshot {
        ElementSnapshot {
            name: self.name.clone(),
            pre_ws: String::from_utf8_lossy(&self.pre_ws).into_owned(),
            attrs: self
                .attrs
                .iter()
                .map(|a| (a.name.clone(), a.value.clone()))
                .collect(),
            quebras: self
                .breaks
                .iter()
                .map(|b| Quebra {
                    at_attr: b.at_attr,
                    cont_indent: b.cont_indent,
                })
                .collect(),
            self_closing: self.self_closing,
            close_pre_ws: String::from_utf8_lossy(&self.close_pre_ws).into_owned(),
            children: self.children.iter().map(Self::snapshot).collect(),
        }
    }
}

impl Document {
    /// Instantâneo fiel deste documento (declaração + árvore + trailing).
    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            decl: String::from_utf8_lossy(&self.decl).into_owned(),
            root: self.root.snapshot(),
            trailing: String::from_utf8_lossy(&self.trailing).into_owned(),
        }
    }

    /// Reconstrói um documento a partir de um instantâneo.
    ///
    /// **Só existe UM writer, e é o [`Element::write`].** A reconstrução monta
    /// as structs privadas e serializa por [`Document::to_bytes`] — escrever os
    /// bytes direto do instantâneo seria um SEGUNDO writer para o mesmo
    /// formato, e dois writers divergem: o `to_bytes` é o que o teste do R4
    /// fiscaliza, então um caminho paralelo poderia ficar errado em silêncio.
    ///
    /// **A validação é a prova, não uma checagem de forma.** Os bytes
    /// reconstruídos são REPARSEADOS com [`Document::parse`] — o mesmo parser
    /// strict que lê os arquivos do Suite. Assim o que é aceito aqui é, por
    /// construção, algo que este módulo sabe reler; um instantâneo com quebra
    /// no primeiro atributo, indent com tab ou `pre_ws` inventado vira erro em
    /// vez de um `.prst` malformado em disco.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] quando o instantâneo descreve um
    /// documento fora do dialecto (o `got` traz a reclamação do parser).
    pub fn from_snapshot(snap: &DocumentSnapshot) -> Result<Self, ProtocolError> {
        let doc = Document {
            decl: snap.decl.as_bytes().to_vec(),
            root: element_from(&snap.root),
            trailing: snap.trailing.as_bytes().to_vec(),
        };
        // Reparsear é o gate: o dialecto tem regras (indent só com espaços,
        // quebra do 1º atributo proibida, raiz `<GP-100>`) que uma checagem
        // campo a campo reimplementaria pior — e pior, sem provar que o
        // documento resultante é relível.
        Document::parse(&doc.to_bytes()).map_err(|e| {
            shape(
                "instantâneo que descreve um .prst válido",
                format!("o parser recusou os bytes reconstruídos: {e}"),
            )
        })
    }
}

/// Constrói um [`Element`] a partir do instantâneo (o inverso de
/// [`Element::snapshot`]).
fn element_from(s: &ElementSnapshot) -> Element {
    Element {
        name: s.name.clone(),
        pre_ws: s.pre_ws.as_bytes().to_vec(),
        attrs: s
            .attrs
            .iter()
            .map(|(name, value)| Attr {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
        breaks: s
            .quebras
            .iter()
            .map(|q| LineBreak {
                at_attr: q.at_attr,
                cont_indent: q.cont_indent,
            })
            .collect(),
        self_closing: s.self_closing,
        children: s.children.iter().map(element_from).collect(),
        close_pre_ws: s.close_pre_ws.as_bytes().to_vec(),
    }
}
