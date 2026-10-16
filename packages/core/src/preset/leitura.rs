//! a LEITURA estrita do dialecto: o `Parser` (o cursor) e o `Document::parse`.

use super::elemento::Attr;
use super::shape;
use super::Document;
use super::{Element, LineBreak};
use crate::ProtocolError;

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
