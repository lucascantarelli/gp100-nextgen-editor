//! o ELEMENTO: o COMPORTAMENTO do `Attr` e do `Element` — os acessores,
//! o writer e o escape. As DECLARACOES ficam no handle (os campos nao abrem).

use super::shape;
use super::Element;
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

/// Atributo: nome + valor BRUTO (na forma escapada do arquivo — o writer
/// reimprime sem re-escapar; `escape_value` existe para valores novos).
#[derive(Debug, Clone)]
pub struct Attr {
    /// Nome do atributo (ex.: `ppName`, `params_0`).
    pub name: String,
    /// Valor na forma crua do arquivo (escapes preservados).
    pub value: String,
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
    pub(super) fn write(&self, out: &mut Vec<u8>) {
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
