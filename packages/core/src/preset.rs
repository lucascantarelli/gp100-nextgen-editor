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
