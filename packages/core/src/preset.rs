//! preset — modelo do arquivo `.prst` com round-trip **byte-idêntico**
//! (regra R4 é sagrada: abrir→salvar sem editar = mesmos bytes).
//!
//! **Estratégia de layout (por que não um writer XML genérico):** os arquivos
//! foram serializados pelo Suite com quebras de linha DENTRO das tags
//! (ex.: `<preset_info … platform="WINDOWS"\r\n               time="…"/>`)
//! e indentação própria. Reproduzir isso com um writer genérico exigiria
//! adivinhar o algoritmo de wrap — chute proibido (R1). Em vez disso, o
//! parser REGISTRA o layout observado de cada elemento (ordem dos atributos,
//! quais atributos iniciam linha nova e o indent da continuação) e o writer
//! o reproduz. Layout é DADO, não algoritmo.
//!
//! **Dialecto suportado** (evidência: os 3 `.prst` de `files/patches/`):
//! `<?xml …?>` + linha em branco + `<GP-100>`; filhos sem texto livre
//! (só elementos); atributos com `="…"`, valores com `&amp;` etc. na forma
//! ESCAPADA (preservamos os bytes crus — decodificar é trabalho da UI/M1);
//! fim de linha CRLF. Qualquer coisa fora disso (comentário, CDATA, texto)
//! é rejeitada com `ProtocolError::InvalidShape` — strict de propósito.
//!
//! **Camadas:**
//! 1. [`Document`]/[`Element`] — DOM mínimo com layout preservado + writer;
//! 2. Views tipadas ([`PresetView`], [`EffectView`]) para o vocabulário
//!    `pp*`/`Effect`/`params_0..14` (§13.9), SEM esconder atributos
//!    desconhecidos (preservação obrigatória).
//!
//! **Testes:** `gp100-core/tests/roundtrip_prst.rs` — round-trip byte-a-byte
//! dos 3 arquivos reais + edição estável (modelo híbrido da skill
//! rust-practices).

use crate::ProtocolError;

/// Escapa um valor para a forma XML (use antes de [`Element::set_attr`]
/// quando o novo valor vier de texto livre do usuário — o `.prst` guarda
/// `&amp;`, `&lt;`, `&gt;`, `&quot;` na forma escapada).
pub fn escape_value(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Constrói `ProtocolError::InvalidShape` com posição do arquivo (byte offset).
fn shape(expected: impl Into<String>, got: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidShape {
        expected: expected.into(),
        got: got.into(),
    }
}

/// O `pp` de um atributo `ppID` — **DECIMAL**: a base ÚNICA do espaço de
/// `pp` (#132/ADR-12).
///
/// **Por que decimal, e não hex.** Os 99 `ppID` do `all.prst` são as
/// strings `"0".."98"`, e o aparelho numera exatamente assim: a captura S1
/// varre `0x0000..=0x0062` (99 valores contíguos = `0..98` em hex no fio) —
/// hex no `ppID` daria uma lista ESPARSA (`0x0a` não existe, `"98"` →
/// `0x98` = 152, fora de qualquer banco do aparelho). Quem mais lê este
/// atributo em decimal é a semente da biblioteca (`gp100_library::seed`)
/// e o gerador do artefato do front (`analysis/dump_preset_list.py`, que
/// ASSERTA `0..98`): esta função é onde core, biblioteca e front passam a
/// concordar (#132, critério de aceite).
pub fn pp_id_decimal(pp_id: &str) -> Option<u16> {
    pp_id.trim().parse().ok()
}

/// Converte um `pp` do **fio** no índice do **documento**.
///
/// O fio carrega o banco no byte alto: a captura S1 varre `0x0000..=0x0062`
/// E `0x0100..=0x0162` — os mesmos 99 slots nos dois bancos, com o conteúdo
/// idêntico índice a índice (97/99 iguais byte a byte na captura; os 2
/// diferentes são slots editados). O `all.prst` não tem banco, então o
/// índice é o byte baixo: `0x0100` → `0`.
///
/// `pp` com byte alto fora de `0x00`/`0x01` passa intacto — é um número
/// que nenhum aparelho prova ter, e o lookup tem de FALHAR em vez de
/// resolver outro preset por engano (um `pp & 0xFF` cego faria `0x0200`
/// abrir o preset 0).
pub fn indice_do_documento(pp: u16) -> u16 {
    if pp > 0x01FF {
        pp
    } else {
        pp & 0xFF
    }
}

/// Atributo: nome + valor BRUTO (na forma escapada do arquivo — o writer
/// reimprime sem re-escapar; `escape_value` existe para valores novos).
#[derive(Debug, Clone)]
pub struct Attr {
    /// Nome do atributo (ex.: `ppName`, `params_0`).
    pub name: String,
    /// Valor na forma crua do arquivo (escapes preservados).
    pub value: String,
}

/// Quebra de linha registrada pelo parser: o atributo `at_attr` inicia
/// linha nova com `cont_indent` espaços (o writer reproduz exatamente).
#[derive(Debug, Clone)]
struct LineBreak {
    /// Índice do atributo que começa a linha de continuação (>= 1).
    at_attr: usize,
    /// Espaços de indentação da linha de continuação.
    cont_indent: usize,
}

/// Elemento do dialecto `.prst` com layout preservado.
///
/// Campos privados de propósito: o acesso é por [`Element::attr`]/[`Element::set_attr`]
/// etc., para que as invariantes de layout (ordem/quebras) nunca sejam
/// quebradas por quem edita.
#[derive(Debug, Clone)]
pub struct Element {
    name: String,
    /// Whitespace bruto ANTES do `<` (inclui `\r\n` e indent — reproduzido).
    pre_ws: Vec<u8>,
    attrs: Vec<Attr>,
    breaks: Vec<LineBreak>,
    self_closing: bool,
    children: Vec<Element>,
    /// Whitespace antes de `</nome>` (só quando `!self_closing`).
    close_pre_ws: Vec<u8>,
}

impl Element {
    /// Nome da tag (ex.: `presets`, `Effect`, `ppIRInfo0`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Valor BRUTO de um atributo (`None` se ausente). Escapes intactos:
    /// `ppName="Dub&amp;Vibe"` devolve `"Dub&amp;Vibe"`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Valor de atributo parseado como u32 (`effectCode`, `ppID`, …).
    pub fn attr_u32(&self, name: &str) -> Option<u32> {
        self.attr(name).and_then(|v| v.parse().ok())
    }

    /// Lista imutável de atributos (ordem do arquivo).
    pub fn attrs(&self) -> &[Attr] {
        &self.attrs
    }

    /// Substitui o valor de um atributo EXISTENTE.
    ///
    /// Só atributos presentes: criar atributo novo mudaria o layout
    /// (proibido aqui — a preservação de layout é o contrato).
    /// Passe o valor já na forma escapada (ver [`escape_value`]).
    pub fn set_attr(&mut self, name: &str, value: &str) -> Result<(), ProtocolError> {
        // Valor não pode conter aspas/`<`/`>` crus (quebraria o XML); `&`
        // cru também não — usar escape_value (documentado no topo).
        if value.contains(['"', '<', '>']) || contains_raw_ampersand(value) {
            return Err(shape(
                format!("valor escapado para o atributo '{name}'"),
                format!("valor contém caractere cru não escapado: {value:?}"),
            ));
        }
        let a = self
            .attrs
            .iter_mut()
            .find(|a| a.name == name)
            .ok_or_else(|| {
                shape(
                    format!("atributo existente '{name}' em <{}>", self.name),
                    "atributo ausente — criar atributos novos mudaria o layout (R4)",
                )
            })?;
        a.value = value.to_string();
        Ok(())
    }

    /// Filhos diretos (vazios se `self_closing`).
    pub fn children(&self) -> &[Element] {
        &self.children
    }

    /// Primeiro filho com o nome dado (`ppCtrl`, `ppEXP1`, `preset_info`…).
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Primeiro filho com o nome dado, mutável (navegação para edição).
    pub fn child_mut(&mut self, name: &str) -> Option<&mut Element> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    /// Filhos com o nome dado, em ordem (ex.: todos os `<Effect>`).
    pub fn children_named<'s>(&'s self, name: &'s str) -> impl Iterator<Item = &'s Element> + 's {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Serializa o elemento (recursivo) exatamente como foi lido/editado.
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.pre_ws);
        out.push(b'<');
        out.extend_from_slice(self.name.as_bytes());
        for (i, a) in self.attrs.iter().enumerate() {
            if i == 0 {
                out.push(b' ');
            } else if let Some(br) = self.breaks.iter().find(|b| b.at_attr == i) {
                // quebra registrada: CRLF + indent da continuação (sem espaço
                // antes do attr — o indent é o alinhamento sob o 1º atributo)
                out.extend_from_slice(b"\r\n");
                out.extend(std::iter::repeat_n(b' ', br.cont_indent));
            } else {
                out.push(b' ');
            }
            out.extend_from_slice(a.name.as_bytes());
            out.extend_from_slice(b"=\"");
            out.extend_from_slice(a.value.as_bytes());
            out.push(b'"');
        }
        if self.self_closing {
            out.extend_from_slice(b"/>");
            return;
        }
        out.push(b'>');
        for c in &self.children {
            c.write(out);
        }
        out.extend_from_slice(&self.close_pre_ws);
        out.extend_from_slice(b"</");
        out.extend_from_slice(self.name.as_bytes());
        out.push(b'>');
    }
}

/// `true` se `s` tem `&` que NÃO é entidade válida do dialecto (`&amp;`,
/// `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&#…;`). Usado por `set_attr`.
fn contains_raw_ampersand(s: &str) -> bool {
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c != b'&' {
            continue;
        }
        let rest = &s[i + 1..];
        let ok = ["amp;", "lt;", "gt;", "quot;", "apos;"]
            .iter()
            .any(|e| rest.starts_with(e))
            || {
                // referência numérica &#123; ou &#x1F;
                let hex = rest.starts_with('x') || rest.starts_with('X');
                let digits: String = rest[if hex { 1 } else { 0 }..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                !digits.is_empty()
                    && digits.chars().all(|c| {
                        if hex {
                            c.is_ascii_hexdigit()
                        } else {
                            c.is_ascii_digit()
                        }
                    })
                    && rest[if hex { 1 } else { 0 }..]
                        .get(digits.len()..)
                        .map(|tail| tail.starts_with(';'))
                        .unwrap_or(false)
            };
        if !ok {
            return true;
        }
    }
    false
}

/// Documento `.prst` completo: declaração XML + `<GP-100>` + whitespace final.
#[derive(Debug, Clone)]
pub struct Document {
    /// Bytes crus da declaração `<?xml …?>`.
    decl: Vec<u8>,
    /// Raiz `<GP-100>` (pre_ws inclui a linha em branco pós-declaração).
    root: Element,
    /// Whitespace após `</GP-100>` (o `\r\n` final).
    trailing: Vec<u8>,
}

impl Document {
    /// Faz o parse de um `.prst` (strict: dialecto documentado no topo).
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] com offset do byte em caso de estrutura
    /// fora do dialecto (comentário, CDATA, texto livre, tag mismatch…).
    pub fn parse(data: &[u8]) -> Result<Self, ProtocolError> {
        let s = std::str::from_utf8(data).map_err(|e| {
            shape(
                "arquivo .prst em UTF-8 válido",
                format!("UTF-8 inválido: {e}"),
            )
        })?;
        if s.starts_with('\u{FEFF}') {
            return Err(shape("começo do arquivo", "BOM UTF-8 inesperado"));
        }
        let mut p = Parser {
            b: s.as_bytes(),
            pos: 0,
        };

        // Passo 1 — declaração XML crua: `<?xml … ?>` (reproduzida verbatim).
        if !p.rest().starts_with(b"<?xml") {
            return Err(shape("declaração '<?xml …?>' no início", p.snippet(24)));
        }
        let end = p
            .rest()
            .windows(2)
            .position(|w| w == b"?>")
            .ok_or_else(|| shape("fim '?>' da declaração", p.snippet(24)))?;
        let decl = p.b[..p.pos + end + 2].to_vec();
        p.pos += end + 2;

        // Passo 2 — árvore a partir de <GP-100>.
        let root = p.parse_element()?;
        if root.name != "GP-100" {
            return Err(shape("raiz <GP-100>", format!("<{}>", root.name)));
        }

        // Passo 3 — só whitespace pode restar (o \r\n final).
        let trailing = p.take_ws();
        if p.pos != p.b.len() {
            return Err(shape(
                "EOF após </GP-100>",
                format!("conteúdo extra: {}", p.snippet(24)),
            ));
        }

        Ok(Document {
            decl,
            root,
            trailing,
        })
    }

    /// Serializa de volta. Sem edições, o resultado é BYTE-IDÊNTICO ao
    /// original (contrato R4 — provado em `tests/roundtrip_prst.rs`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.root.pre_ws.len() + self.decl.len() * 2);
        out.extend_from_slice(&self.decl);
        self.root.write(&mut out);
        out.extend_from_slice(&self.trailing);
        out
    }

    /// Raiz `<GP-100>` (leitura).
    pub fn root(&self) -> &Element {
        &self.root
    }

    /// Raiz `<GP-100>` (edição).
    pub fn root_mut(&mut self) -> &mut Element {
        &mut self.root
    }

    /// O `<preset_info …/>` (metadados do arquivo: software/firmware/count…).
    pub fn preset_info(&self) -> Option<&Element> {
        self.root.child("preset_info")
    }

    /// Todos os blocos `<presets>` (um por preset; all.prst tem 99).
    pub fn presets(&self) -> impl Iterator<Item = PresetView<'_>> {
        self.root
            .children()
            .iter()
            .filter(|c| c.name == "presets")
            .map(|el| PresetView { el })
    }
}

/// View tipada de um bloco `<presets>` — o vocabulário pp* do §13.9.
#[derive(Debug, Clone, Copy)]
pub struct PresetView<'a> {
    el: &'a Element,
}

impl<'a> PresetView<'a> {
    /// O elemento por baixo da view (para attrs genéricos/edit).
    pub fn element(&self) -> &'a Element {
        self.el
    }

    /// `ppName` — nome do preset (forma crua/escapada).
    pub fn pp_name(&self) -> Option<&'a str> {
        self.el.attr("ppName")
    }
    /// `ppBank` — banco (u8 no fio; aqui cru).
    pub fn pp_bank(&self) -> Option<&'a str> {
        self.el.attr("ppBank")
    }
    /// `ppID` — id do preset (u16 BE no fio; cru aqui).
    pub fn pp_id(&self) -> Option<&'a str> {
        self.el.attr("ppID")
    }
    /// `ppType` — tipo/ícone (u16 BE no fio; cru aqui).
    pub fn pp_type(&self) -> Option<&'a str> {
        self.el.attr("ppType")
    }
    /// `ppTypeName` — rótulo do tipo (ex.: "Pop").
    pub fn pp_type_name(&self) -> Option<&'a str> {
        self.el.attr("ppTypeName")
    }
    /// `ppIRNum` — IR associado (se houver).
    pub fn pp_ir_num(&self) -> Option<&'a str> {
        self.el.attr("ppIRNum")
    }

    /// Os 9 `<Effect>` do preset, na ordem do arquivo (x = 0..8).
    pub fn effects(&self) -> impl Iterator<Item = EffectView<'a>> + 'a {
        self.el
            .children()
            .iter()
            .filter(|c| c.name == "Effect")
            .map(|el| EffectView { el })
    }

    /// O `<Effect>` do módulo da cadeia (PRE/DST/AMP/NR/CAB/EQ/MOD/DLY/RVB).
    pub fn effect(&self, module: &str) -> Option<EffectView<'a>> {
        self.effects().find(|e| e.module() == module)
    }
}

/// View tipada de um `<Effect>` — attrs fixos + `params_0..14` (§13.9).
#[derive(Debug, Clone, Copy)]
pub struct EffectView<'a> {
    el: &'a Element,
}

impl<'a> EffectView<'a> {
    /// O elemento por baixo da view (para attrs genéricos/edit).
    pub fn element(&self) -> &'a Element {
        self.el
    }

    /// `effectModuleName` — slot da cadeia (PRE..RVB).
    pub fn module(&self) -> &'a str {
        self.el.attr("effectModuleName").unwrap_or("")
    }
    /// `effectName` — nome do algoritmo (pode ter espaços de padding).
    pub fn name(&self) -> Option<&'a str> {
        self.el.attr("effectName")
    }
    /// `effectState` — "1" ativo / "0" bypass.
    pub fn state(&self) -> Option<&'a str> {
        self.el.attr("effectState")
    }
    /// `effectCode` parseado — `(nibble << 24) | index` (u32 decimal).
    pub fn code(&self) -> Option<u32> {
        self.el.attr_u32("effectCode")
    }
    /// `params_N` cru (N = 0..14; §13.9: preservar params_N 0..14 — há
    /// controles ocultos fora do parameters.json, ex.: Mic do CAB).
    pub fn param(&self, n: u8) -> Option<&'a str> {
        if n > 14 {
            return None;
        }
        self.el.attr(&format!("params_{n}"))
    }
}

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

impl Document {
    /// Um documento com o `<presets>` escolhido e mais nada de preset.
    ///
    /// **Por que existe.** O app embute o `all.prst` inteiro (99 presets) e
    /// exporta UM. Sem esta função, exportar um preset de fábrica só seria
    /// possível exportando os 99 — e o arquivo que o dono leva deixaria de ser
    /// "o preset que ele escolheu".
    ///
    /// **A forma imita a dos arquivos reais de um preset só.** O `.prst` de
    /// fábrica de um preset tem `preset_info` + `presets`, e **não** tem
    /// `<ppIRInfo>` — essa tabela é do arquivo de biblioteca inteira. Manter a
    /// tabela (ou o `count` do arquivo de origem) descreveria um documento
    /// diferente do que o arquivo é, então ela sai e o `count` passa a ser o
    /// número de presets AQUI (1).
    ///
    /// `pp` casa com o `ppID` do bloco do mesmo jeito que
    /// [`crate::pedalboard::board_view_for`] casa — mesma expressão, para o
    /// "preset 25" da tela e o "preset 25" da exportação nunca divergirem.
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] quando o documento não tem preset ou o
    /// `pp` pedido não existe.
    pub fn apenas_preset(&self, pp: Option<u16>) -> Result<Self, ProtocolError> {
        // O `pp` pedido pode vir do fio com byte de banco (0x0100 = índice 0
        // — #132); o documento só tem o índice. A mensagem de erro continua
        // nomeando o pp PEDIDO (com banco), que é o que o chamador digitou.
        let alvo = pp.map(crate::preset::indice_do_documento);
        let alvo_index = self
            .root
            .children()
            .iter()
            .position(|c| {
                c.name == "presets"
                    && match alvo {
                        None => true,
                        Some(t) => c.attr("ppID").and_then(crate::preset::pp_id_decimal) == Some(t),
                    }
            })
            .ok_or_else(|| {
                shape(
                    match pp {
                        None => "documento com pelo menos um <presets>".to_string(),
                        // As DUAS notações de propósito: o índice do arquivo
                        // é DECIMAL (`"0".."98"`, #156) e o espaço do fio é
                        // hex (banco/slot, §13.4) — a mensagem não pode deixar
                        // dúvida sobre qual número não foi achado.
                        Some(t) => format!("<presets> com ppID {t:#06x} ({t})"),
                    },
                    "nenhum bloco corresponde",
                )
            })?;

        // A raiz fica com a mesma identidade (decl/indent) e recebe só os
        // metadados + o bloco escolhido. `pre_ws` de cada filho já carrega a
        // indentação dele, e o aninhamento não muda — então o layout continua
        // válido sem nenhum ajuste.
        let mut nova = self.root.clone();
        nova.children = Vec::new();
        for (i, c) in self.root.children().iter().enumerate() {
            match c.name.as_str() {
                // os OUTROS blocos de preset
                "presets" if i != alvo_index => continue,
                // a tabela de IRs é do arquivo de biblioteca, não do preset
                "ppIRInfo" => continue,
                _ => {}
            }
            if c.name == "preset_info" {
                let mut info = c.clone();
                // `count` só existe no preset_info; se faltar, o arquivo já
                // mentia antes de nós e não é nosso trabalho inventar.
                let _ = info.set_attr("count", "1");
                nova.children.push(info);
            } else {
                nova.children.push(c.clone());
            }
        }

        let doc = Document {
            decl: self.decl.clone(),
            root: nova,
            trailing: self.trailing.clone(),
        };
        // Mesmo gate do `from_snapshot`: o que sai daqui tem de ser relível
        // pelo parser strict, senão o dono recebe um arquivo que não abre.
        Document::parse(&doc.to_bytes()).map_err(|e| {
            shape(
                "recorte de um preset que continua válido",
                format!("o parser recusou o recorte: {e}"),
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

// ---------------------------------------------------------------- parser
struct Parser<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a [u8] {
        &self.b[self.pos..]
    }

    /// Recorte legível do ponto atual (para mensagens de erro).
    fn snippet(&self, max: usize) -> String {
        let end = (self.pos + max).min(self.b.len());
        String::from_utf8_lossy(&self.b[self.pos..end]).into()
    }

    fn take_ws(&mut self) -> Vec<u8> {
        let start = self.pos;
        while self.pos < self.b.len() && self.b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        self.b[start..self.pos].to_vec()
    }

    /// Whitespace DENTRO de uma tag. Devolve (houve_quebra, indent da última
    /// linha). Strict: indentação de continuação só com espaços.
    fn take_tag_ws(&mut self) -> Result<(bool, usize), ProtocolError> {
        let start = self.pos;
        while self.pos < self.b.len() && self.b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        let ws = &self.b[start..self.pos];
        if ws.is_empty() {
            return Ok((false, 0));
        }
        if ws.contains(&b'\t') {
            return Err(shape(
                "apenas espaços na indentação de atributos",
                self.snippet(24),
            ));
        }
        match ws.iter().rposition(|&c| c == b'\n') {
            None => Ok((false, 0)),
            Some(i) => {
                let indent = ws[i + 1..].iter().filter(|&&c| c == b' ').count();
                if indent != ws[i + 1..].len() {
                    return Err(shape(
                        "linha de continuação de atributos só com espaços",
                        self.snippet(24),
                    ));
                }
                Ok((true, indent))
            }
        }
    }

    /// Nome de tag/atributo: `[A-Za-z0-9_:.=-]` até delimiter (strict).
    fn take_name(&mut self) -> Result<String, ProtocolError> {
        let start = self.pos;
        while self.pos < self.b.len() {
            let c = self.b[self.pos];
            if c.is_ascii_alphanumeric() || matches!(c, b'_' | b':' | b'.' | b'-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(shape("nome de tag/atributo", self.snippet(24)));
        }
        Ok(String::from_utf8_lossy(&self.b[start..self.pos]).into())
    }

    fn expect(&mut self, pat: &[u8]) -> Result<(), ProtocolError> {
        if self.rest().starts_with(pat) {
            self.pos += pat.len();
            Ok(())
        } else {
            Err(shape(
                format!("'{}'", String::from_utf8_lossy(pat)),
                self.snippet(24),
            ))
        }
    }

    /// Consome até `stop` e devolve o trecho (não inclui o stop).
    fn take_until(&mut self, stop: u8) -> Result<String, ProtocolError> {
        let start = self.pos;
        while self.pos < self.b.len() && self.b[self.pos] != stop {
            self.pos += 1;
        }
        if self.pos >= self.b.len() {
            return Err(shape(
                format!("'{}' de fechamento", stop as char),
                "EOF inesperado",
            ));
        }
        Ok(String::from_utf8_lossy(&self.b[start..self.pos]).into())
    }

    /// Parse de um elemento (recursivo; registra quebras de atributos).
    /// O whitespace ANTES do `<` é o `pre_ws` DESTE elemento (consumido aqui
    /// e reproduzido pelo writer — é assim que a indentação sobrevive).
    fn parse_element(&mut self) -> Result<Element, ProtocolError> {
        let pre_ws = self.take_ws();
        self.expect(b"<")?;
        if self.rest().starts_with(b"!--") || self.rest().starts_with(b"![") {
            return Err(shape(
                "só elementos no dialecto .prst (sem comentários/CDATA)",
                self.snippet(24),
            ));
        }
        let name = self.take_name()?;
        let mut el = Element {
            name,
            pre_ws,
            attrs: Vec::new(),
            breaks: Vec::new(),
            self_closing: false,
            children: Vec::new(),
            close_pre_ws: Vec::new(),
        };

        // Atributos / fim da tag de abertura.
        loop {
            let (had_nl, indent) = self.take_tag_ws()?;
            if self.rest().starts_with(b"/>") {
                self.pos += 2;
                el.self_closing = true;
                return Ok(el);
            }
            if self.rest().starts_with(b">") {
                self.pos += 1;
                break; // tag com filhos
            }
            // Atributo (quebra registrada ANTES dele, se houve linha nova).
            if had_nl {
                if el.attrs.is_empty() {
                    return Err(shape(
                        "primeiro atributo na mesma linha do '<'",
                        self.snippet(24),
                    ));
                }
                el.breaks.push(LineBreak {
                    at_attr: el.attrs.len(),
                    cont_indent: indent,
                });
            }
            let aname = self.take_name()?;
            self.expect(b"=\"")?;
            let value = self.take_until(b'"')?;
            self.pos += 1; // consome a aspa de fechamento
            el.attrs.push(Attr { name: aname, value });
        }

        // Filhos até `</nome>` (sem texto livre no dialecto).
        // O whitespace à frente é espiado SEM consumir: se o próximo elemento
        // é o fechamento, ele vira `close_pre_ws`; senão, será o `pre_ws` do
        // filho (que parse_element consome por si — NUNCA consumir antes).
        loop {
            let mut q = self.pos;
            while q < self.b.len() && self.b[q].is_ascii_whitespace() {
                q += 1;
            }
            if q >= self.b.len() {
                return Err(shape(format!("</{}>", el.name), "EOF antes do fechamento"));
            }
            if self.b[q..].starts_with(b"</") {
                el.close_pre_ws = self.b[self.pos..q].to_vec();
                self.pos = q + 2; // consome '</'
                let cname = self.take_name()?;
                if cname != el.name {
                    return Err(shape(format!("</{}>", el.name), format!("</{cname}>")));
                }
                // strict: nada entre o nome e o '>' (ws aqui quebraria o round-trip)
                self.expect(b">")?;
                return Ok(el);
            }
            // novo filho: parse_element consome o pre_ws dele mesmo
            let child = self.parse_element()?;
            el.children.push(child);
        }
    }
}
