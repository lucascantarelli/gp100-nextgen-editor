//! tone_sheet — a folha de timbre em PDF (issue #114).
//!
//! **Para que serve.** O dono leva para o palco uma folha onde dá para ler a
//! cadeia de 9 slots e os valores dos knobs, anotar com caneta e voltar para o
//! editor. É o outro lado da #114: o JSON resolve *comparar*; o PDF resolve
//! *imprimir e anotar*.
//!
//! **Por que um writer PRÓPRIO, e não uma biblioteca.** A regra do ADR-9 é não
//! pôr dependência nativa na build, e as crates de PDF em Rust ou puxam
//! rasterizador/cripto (custo que este projeto não tem como pagar no build
//! multiplataforma da #17) ou exigem embutir métrica de fonte. O subconjunto
//! que a folha precisa é pequeno e **fechado**: página de tamanho fixo, três
//! fontes base-14, texto, retângulo, linha. Escrever isso são ~40 operadores
//! PDF, e em troca a folha é determinística, não tem I/O e roda em qualquer
//! plataforma que o crate já suporta.
//!
//! **As fontes são base-14 de propósito.** `Helvetica`, `Helvetica-Bold` e
//! `Courier` são obrigatórias em qualquer leitor de PDF (não precisam ser
//! embutidas, não têm licença a distribuir) — e isso mantém o arquivo em ~4 KB.
//! O preço é que texto não-ASCII só sai correto em `WinAnsiEncoding`; ver
//! [`lit`].
//!
//! **A cadeia é a do app.** A fileira de nove caixas, na ordem do sinal
//! (PRE DST AMP NR CAB EQ MOD DLY RVB), é a mesma ordem que o palco desenha —
//! a folha não inventa uma leitura própria, senão o dono leria duas coisas
//! diferentes para a mesma cadeia.
//!
//! **Nada aqui fala com o aparelho.** O PDF nasce do `Document` do `.prst` em
//! memória; nenhum byte vai para o GP-100. É o que faz a folha ser segura de
//! gerar em qualquer build, inclusive no mock.

use crate::model::Dictionary;
use crate::preset::{Document, EffectView};
use crate::ProtocolError;

/// Largura da página em pontos (A4 retrato). A4 e não Letter porque o projeto
/// é pt-BR e o A4 é o que a impressora do dono tem.
const LARGURA: f64 = 595.28;
/// Altura da página em pontos (A4 retrato).
const ALTURA: f64 = 841.89;
/// Margem de todos os lados.
const MARGEM: f64 = 40.0;
/// Altura de uma linha de texto da tabela.
const LINHA: f64 = 11.0;
/// A ordem do sinal — a MESMA do palco (não reordenar aqui).
const MODULOS: [&str; 9] = ["PRE", "DST", "AMP", "NR", "CAB", "EQ", "MOD", "DLY", "RVB"];

/// Escapa um texto para literal de string PDF (`(…)`), em `WinAnsiEncoding`.
///
/// Três decisões, cada uma com motivo:
/// - `(`, `)` e `\` são os únicos bytes que o PDF exige escapar — os outros
///   imprimíveis entram crus e o arquivo fica legível no editor de texto;
/// - caractere fora do ASCII e dentro do Latin-1 vira UM byte (`WinAnsiEncoding`
///   é Latin-1 nessa faixa), que é o certo para nome de preset com acento;
/// - qualquer outra coisa vira `?`. É uma perda VISÍVEL e não silenciosa: a
///   folha mostra que não saberia escrever aquilo, em vez de imprimir lixo.
fn lit(texto: &str) -> String {
    let mut out = String::with_capacity(texto.len() + 2);
    out.push('(');
    for c in texto.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            ' '..='~' => out.push(c),
            '\u{a0}'..='\u{ff}' => {
                // Latin-1: o byte é o próprio ponto de código. Octal sempre:
                // é octal que o PDF define, e evita ambiguidade com dígito.
                out.push_str(&format!("\\{:03o}", c as u32));
            }
            _ => out.push('?'),
        }
    }
    out.push(')');
    out
}

/// Largura aproximada em pontos de `texto` em Courier de `tamanho` pt.
///
/// Courier é monoespaçada e a métrica é EXATA: cada glifo vale 600/1000 do
/// tamanho. Existe para decidir quebra de linha dentro da caixa da cadeia sem
/// embutir a tabela de métricas da fonte — que é justamente o que este módulo
/// evita para não arrastar dependência.
fn largura_courier(texto: &str, tamanho: f64) -> f64 {
    texto.chars().count() as f64 * tamanho * 0.6
}

/// Quebra `texto` em linhas de no máximo `max_chars` caracteres.
///
/// Quebra por PALAVRA quando dá, por caractere quando a palavra sozinha não
/// cabe — sem isso um nome longo viraria uma linha que atravessa a caixa.
fn quebra(texto: &str, max_chars: usize) -> Vec<String> {
    let mut linhas: Vec<String> = Vec::new();
    let mut atual = String::new();
    for palavra in texto.split_whitespace() {
        if palavra.chars().count() > max_chars {
            // palavra maior que a caixa: corta duro, na quantidade que cabe
            if !atual.is_empty() {
                linhas.push(std::mem::take(&mut atual));
            }
            let mut resto: Vec<char> = palavra.chars().collect();
            while resto.len() > max_chars {
                linhas.push(resto.drain(..max_chars).collect());
            }
            atual = resto.into_iter().collect();
            continue;
        }
        let tentativa = if atual.is_empty() {
            palavra.len()
        } else {
            atual.chars().count() + 1 + palavra.chars().count()
        };
        if tentativa > max_chars && !atual.is_empty() {
            linhas.push(std::mem::take(&mut atual));
        }
        if !atual.is_empty() {
            atual.push(' ');
        }
        atual.push_str(palavra);
    }
    if !atual.is_empty() {
        linhas.push(atual);
    }
    linhas
}

/// Uma página em construção: o stream de conteúdo e a posição vertical.
struct Folha {
    /// Operadores do stream de conteúdo.
    conteudo: String,
    /// Cursor vertical (pt), de baixo para cima como o PDF.
    y: f64,
}

impl Folha {
    fn nova() -> Self {
        Folha {
            conteudo: String::new(),
            y: ALTURA - MARGEM,
        }
    }

    /// Há espaço para `altura` pt antes da margem inferior?
    fn cabe(&self, altura: f64) -> bool {
        self.y - altura >= MARGEM
    }

    /// Escreve um texto em `(x, y)` com a fonte dada.
    fn texto(&mut self, x: f64, y: f64, fonte: &str, tamanho: f64, texto: &str) {
        self.conteudo.push_str(&format!(
            "BT /{fonte} {tamanho} Tf {x:.2} {y:.2} Td {} Tj ET\n",
            lit(texto)
        ));
    }

    /// Desenha o contorno de um retângulo.
    fn retangulo(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.conteudo.push_str(&format!(
            "0.45 w 0.55 0.55 0.55 RG {x:.2} {y:.2} {w:.2} {h:.2} re S\n"
        ));
    }

    /// Desenha uma linha horizontal (separador de tabela).
    fn linha(&mut self, y: f64) {
        self.conteudo.push_str(&format!(
            "0.35 w 0.78 0.78 0.78 RG {MARGEM:.2} {y:.2} m {:.2} {y:.2} l S\n",
            LARGURA - MARGEM
        ));
    }

    /// Pinta um retângulo (faixa de cabeçalho).
    fn faixa(&mut self, x: f64, y: f64, w: f64, h: f64, cinza: f64) {
        self.conteudo.push_str(&format!(
            "{cinza:.2} {cinza:.2} {cinza:.2} rg {x:.2} {y:.2} {w:.2} {h:.2} re f\n"
        ));
    }
}

/// Gera a folha de timbre em PDF para TODOS os presets do documento.
///
/// Uma página (ou mais) por preset: um `.prst` de fábrica tem 1, o `all.prst`
/// tem 99. A paginação é feita pelo cursor vertical — quando o próximo bloco
/// não cabe, uma página nova começa e o cabeçalho do preset é repetido, para
/// uma folha solta no palco não ficar sem saber de que preset ela é.
///
/// # Erros
/// [`ProtocolError::InvalidShape`] só se a estrutura do `.prst` não permitir a
/// leitura da cadeia (documento sem `<presets>`, por exemplo).
pub fn tone_sheet_pdf(doc: &Document, dict: &Dictionary) -> Result<Vec<u8>, ProtocolError> {
    let mut folhas: Vec<Folha> = Vec::new();
    let presets: Vec<_> = doc.presets().collect();
    if presets.is_empty() {
        return Err(ProtocolError::InvalidShape {
            expected: "documento com pelo menos um <presets>".into(),
            got: format!("{} blocos <presets>", presets.len()),
        });
    }

    let total = presets.len();
    for (indice, preset) in presets.iter().enumerate() {
        let nome = preset.pp_name().unwrap_or("(sem nome)");
        let mut folha = Folha::nova();
        let mut numero_pagina = 1usize;

        // ── cabeçalho ────────────────────────────────────────────────────
        // O brasão diz QUAL PRESET é a folha (`3/99`), e não "página x de y":
        // é a primeira pergunta de quem pega uma folha solta no palco, e com um
        // `.prst` de 99 presets as duas coisas não são a mesma. Quando um preset
        // ocupa mais de uma página, o sufixo ` p2` entra só a partir da segunda.
        let cabecalho = |f: &mut Folha, pagina: usize| {
            let brasao = if pagina > 1 {
                format!("{}/{total}  p{pagina}", indice + 1)
            } else {
                format!("{}/{total}", indice + 1)
            };
            // faixa de título
            f.faixa(MARGEM, f.y - 26.0, LARGURA - 2.0 * MARGEM, 24.0, 0.94);
            f.texto(MARGEM + 6.0, f.y - 18.0, "F2", 12.0, nome);
            f.texto(
                LARGURA - MARGEM - 6.0 - largura_courier(&brasao, 8.0),
                f.y - 18.0,
                "F3",
                8.0,
                &brasao,
            );
            f.y -= 30.0;
            // metadados do pp* numa linha só
            let meta = format!(
                "banco {} · id {} · tipo {} · volume {} · BPM {} · IR {}",
                preset.pp_bank().unwrap_or("-"),
                preset.pp_id().unwrap_or("-"),
                preset.pp_type_name().unwrap_or("-"),
                preset.element().attr("ppVolume").unwrap_or("-"),
                preset.element().attr("ppBPM").unwrap_or("-"),
                preset.pp_ir_num().unwrap_or("-"),
            );
            f.texto(MARGEM, f.y, "F3", 7.5, &meta);
            f.y -= 16.0;
        };
        cabecalho(&mut folha, numero_pagina);

        // ── diagrama da cadeia: 9 caixas na ordem do sinal ──────────────
        const VAO: f64 = 5.0;
        let caixa_w = (LARGURA - 2.0 * MARGEM - 8.0 * VAO) / 9.0;
        let caixa_h = 34.0;
        folha.texto(MARGEM, folha.y, "F2", 8.0, "CADEIA (ordem do sinal)");
        folha.y -= 10.0;
        let topo = folha.y - caixa_h;
        for (i, modulo) in MODULOS.iter().enumerate() {
            let x = MARGEM + i as f64 * (caixa_w + VAO);
            let efeito = preset.effect(modulo);
            let ligado = efeito.as_ref().and_then(|e| e.state()) != Some("0");
            folha.retangulo(x, topo, caixa_w, caixa_h);
            if !ligado {
                // bypass desenhado como caixa cinza: o estado não fica só no texto
                folha.faixa(x + 1.0, topo + 1.0, caixa_w - 2.0, caixa_h - 2.0, 0.90);
            }
            folha.texto(x + 3.0, topo + caixa_h - 9.0, "F2", 7.0, modulo);
            let nome_alg = efeito
                .as_ref()
                .and_then(|e| e.name())
                .unwrap_or("vazio")
                .trim();
            let max_chars = ((caixa_w - 4.0) / (6.0 * 0.6)).floor().max(4.0) as usize;
            for (li, linha) in quebra(nome_alg, max_chars).iter().take(2).enumerate() {
                folha.texto(
                    x + 3.0,
                    topo + caixa_h - 19.0 - li as f64 * 7.0,
                    "F3",
                    6.0,
                    linha,
                );
            }
        }
        folha.y = topo - 12.0;

        // ── tabela de knobs, um bloco por slot com efeito ───────────────
        for modulo in MODULOS.iter() {
            let Some(efeito) = preset.effect(modulo) else {
                continue;
            };
            let alg_nome = efeito.name().unwrap_or("?").trim().to_string();
            let ligado = efeito.state() != Some("0");
            let linhas = linhas_do_slot(&efeito, dict, modulo);

            // 3 linhas de cabeçalho do bloco + as linhas dos knobs
            let altura_bloco = 15.0 + LINHA * (linhas.len().max(1) as f64);
            if !folha.cabe(altura_bloco) {
                folhas.push(folha);
                numero_pagina += 1;
                folha = Folha::nova();
                cabecalho(&mut folha, numero_pagina);
            }

            folha.faixa(MARGEM, folha.y - 11.0, LARGURA - 2.0 * MARGEM, 12.0, 0.96);
            folha.texto(
                MARGEM + 3.0,
                folha.y - 8.5,
                "F2",
                8.5,
                &format!(
                    "{modulo}  {alg_nome}   ({})",
                    if ligado { "ligado" } else { "bypass" }
                ),
            );
            folha.y -= 15.0;

            if linhas.is_empty() {
                folha.texto(
                    MARGEM + 6.0,
                    folha.y,
                    "F3",
                    7.5,
                    "(algoritmo fora do dicionario: sem nome de knob)",
                );
                folha.y -= LINHA;
            } else {
                // UMA coluna, um knob por linha. A versão de duas colunas foi
                // descartada depois de GERAR a folha e lê-la de volta com um
                // leitor de verdade: o extrator intercalava as colunas (nome de
                // uma ao lado do valor da outra), e o que a folha precisa é ser
                // lida de cima para baixo, na caneta. Um preset normal cabe em
                // uma página assim mesmo.
                for (knob, valor) in linhas.iter() {
                    folha.texto(
                        MARGEM + 6.0,
                        folha.y,
                        "F3",
                        7.5,
                        &format!("{knob:<16} {valor}"),
                    );
                    folha.y -= LINHA;
                }
            }
            folha.y -= 6.0;
        }

        // rodapé: de onde veio a folha
        if folha.cabe(24.0) {
            folha.y -= 4.0;
            folha.linha(folha.y);
            folha.y -= 10.0;
            folha.texto(
                MARGEM,
                folha.y,
                "F3",
                6.5,
                &format!(
                    "GP-100 NextGen Editor · folha de timbre · preset {} de {} · gerada offline",
                    indice + 1,
                    presets.len()
                ),
            );
        }
        folhas.push(folha);
    }

    Ok(monta_pdf(&folhas))
}

/// Os pares `(nome do knob, valor)` de um slot, na ordem do dicionário.
///
/// **Nome pelo DICIONÁRIO, valor pelo `.prst`.** O `params_N` do arquivo não
/// traz nome; quem sabe que `params_0` do `C-Wah` é `Range` é o
/// `parameters.json`. Ler o nome de lá e o valor daqui é o que faz a folha
/// dizer "Range 50" em vez de "params_0 50".
///
/// A ordem é a dos CONTROLES (ordem do `pos`), não a dos atributos do `.prst` —
/// no arquivo `params_N` vem intercalado com `x`/`y`, que são layout.
fn linhas_do_slot(e: &EffectView<'_>, dict: &Dictionary, modulo: &str) -> Vec<(String, String)> {
    let Some(code) = e.code() else {
        return Vec::new();
    };
    let nibble = ((code >> 24) & 0xff) as u8;
    let index = code & 0x00ff_ffff;
    // A identidade do algoritmo é a TRIPLA (module, nibble, index): o mesmo
    // effectCode existe em PRE e DST (Boost/14 Boost), e a primeira ocorrência
    // daria os NOMES de knob do módulo errado.
    let Some(alg) = dict.algorithm_in_module(modulo, nibble, index) else {
        return Vec::new();
    };
    alg.controls
        .iter()
        .map(|c| {
            let valor = e.param(c.pos).unwrap_or("").to_string();
            (c.name.clone(), valor)
        })
        .collect()
}

/// Monta o arquivo PDF (cabeçalho, objetos, streams, xref, trailer).
///
/// O xref é o único lugar onde a ordem importa de verdade: cada entrada é o
/// OFFSET EM BYTES do objeto, então o stream inteiro é construído primeiro e os
/// offsets saem de `buf.len()` no momento em que cada objeto é escrito. Contar
/// offset "na mão" é o jeito clássico de gerar um PDF que só abre no leitor
/// permissivo.
fn monta_pdf(folhas: &[Folha]) -> Vec<u8> {
    let n_paginas = folhas.len();
    // Objetos fixos: 1=Catalog, 2=Pages, 3..5=fontes.
    // Depois vêm, por página: a Page e o Contents (2 objetos cada).
    let obj_catalog = 1usize;
    let obj_pages = 2usize;
    let obj_fonte1 = 3usize;
    let obj_fonte2 = 4usize;
    let obj_fonte3 = 5usize;
    let primeiro = 6usize;

    let id_page = |i: usize| primeiro + i * 2;
    let id_conteudo = |i: usize| primeiro + i * 2 + 1;
    let total = primeiro + n_paginas * 2 - 1;

    let mut buf: Vec<u8> = Vec::new();
    // offsets[numero_do_objeto] = offset em bytes
    let mut offsets: Vec<usize> = vec![0; total + 1];

    buf.extend_from_slice(b"%PDF-1.4\n");
    // Comentário com bytes > 127: é a convenção que marca o arquivo como
    // binário para ferramentas de transferência (evita que um `scp` de modo
    // texto no Windows corrompa o stream comprimido de outra pessoa — aqui não
    // há stream comprimido, mas a marca é barata e é o que os leitores esperam).
    buf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

    let escreve_objeto = |buf: &mut Vec<u8>, offsets: &mut Vec<usize>, num: usize, corpo: &str| {
        offsets[num] = buf.len();
        buf.extend_from_slice(format!("{num} 0 obj\n{corpo}\nendobj\n").as_bytes());
    };

    escreve_objeto(
        &mut buf,
        &mut offsets,
        obj_catalog,
        &format!("<< /Type /Catalog /Pages {obj_pages} 0 R >>"),
    );

    let kids: Vec<String> = (0..n_paginas)
        .map(|i| format!("{} 0 R", id_page(i)))
        .collect();
    escreve_objeto(
        &mut buf,
        &mut offsets,
        obj_pages,
        &format!(
            "<< /Type /Pages /Kids [{}] /Count {n_paginas} >>",
            kids.join(" ")
        ),
    );

    // Fontes base-14: não são embutidas (o leitor as tem por obrigação). O
    // WinAnsiEncoding é o que faz o byte Latin-1 virar o glifo certo.
    for (num, base) in [
        (obj_fonte1, "Helvetica"),
        (obj_fonte2, "Helvetica-Bold"),
        (obj_fonte3, "Courier"),
    ] {
        escreve_objeto(
            &mut buf,
            &mut offsets,
            num,
            &format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>"
            ),
        );
    }

    for (i, folha) in folhas.iter().enumerate() {
        escreve_objeto(
            &mut buf,
            &mut offsets,
            id_page(i),
            &format!(
                "<< /Type /Page /Parent {obj_pages} 0 R \
                 /MediaBox [0 0 {LARGURA:.2} {ALTURA:.2}] \
                 /Resources << /Font << /F1 {obj_fonte1} 0 R /F2 {obj_fonte2} 0 R \
                 /F3 {obj_fonte3} 0 R >> >> /Contents {} 0 R >>",
                id_conteudo(i)
            ),
        );
        let fluxo = &folha.conteudo;
        escreve_objeto(
            &mut buf,
            &mut offsets,
            id_conteudo(i),
            &format!("<< /Length {} >>\nstream\n{fluxo}endstream", fluxo.len()),
        );
    }

    // xref: uma entrada por objeto (0 é a cabeça livre, obrigatória).
    let xref_off = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n", total + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    // `offsets[0]` não entra: a entrada 0 da xref é a cabeça livre, já escrita.
    for offset in offsets.iter().take(total + 1).skip(1) {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root {obj_catalog} 0 R >>\nstartxref\n{xref_off}\n%%EOF\n",
            total + 1
        )
        .as_bytes(),
    );
    buf
}
