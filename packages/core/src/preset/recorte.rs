//! o RECORTE: o documento com o `<presets>` escolhido e nada mais de preset.

use super::shape;
use super::Document;
use crate::ProtocolError;

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
