//! o DESPACHO por endpoint: o indice `(func, addr)` e a escolha do template.

use super::shape;
use super::template::Template;
use super::GoldenFile;
use crate::ProtocolError;

/// Addr 4B em hex minúsculo (mensagens de erro).
fn addr_hex(addr: &[u8; 4]) -> String {
    addr.iter().map(|b| format!("{b:02x}")).collect()
}

impl GoldenFile {
    /// O template de REQUEST (`req`/`write`) para o endpoint TIPADO
    /// `(func u8, addr 4B BE)` — a forma que a FSM tem em mãos.
    ///
    /// # Desambiguação
    /// Três endpoints do golden têm DOIS templates (caso CONGELADO do boot —
    /// payload const — e caso GERAL — pp variável). A chave é o nº de vars:
    /// `var_count == vars.len()` escolhe o certo; sem duplicata, 0 vars
    /// devolve o único. Erro se nenhum/nenhum único casar.
    pub fn request_template(
        &self,
        func: u8,
        addr: &[u8; 4],
        vars: usize,
    ) -> Result<&Template, ProtocolError> {
        let idx = self.by_out.get(&(func, *addr)).ok_or_else(|| {
            shape(
                "endpoint de request no golden",
                format!("func={func:02x} addr={}", addr_hex(addr)),
            )
        })?;
        let mut found = idx.iter().filter(|&&i| {
            self.templates[i]
                .request_pattern()
                .and_then(|p| p.var_count())
                == Some(vars)
        });
        let first = found.next().ok_or_else(|| {
            shape(
                format!(
                    "template com {vars} bytes de var em {func:02x}/{}",
                    addr_hex(addr)
                ),
                "nenhum candidato",
            )
        })?;
        if found.next().is_some() {
            return Err(shape(
                "desambiguação única por var_count",
                format!(
                    "2+ templates com {vars} vars em {func:02x}/{}",
                    addr_hex(addr)
                ),
            ));
        }
        Ok(&self.templates[*first])
    }

    /// Atalho ergonômico da FSM: resolve o template pelos TIPOS e já monta
    /// o SysEx completo. Equivale a `request_template(func, addr,
    /// vars.len())?.build_request(vars)`.
    pub fn build_request(
        &self,
        func: u8,
        addr: &[u8; 4],
        vars: &[u8],
    ) -> Result<Vec<u8>, ProtocolError> {
        self.request_template(func, addr, vars.len())?
            .build_request(vars)
    }

    /// TODOS os templates cujo IN é o endpoint TIPADO — respostas podem ter
    /// múltiplos formatos (by-len: ACK 4B × tabela 75B em `12001002`).
    pub fn for_response(&self, func: u8, addr: &[u8; 4]) -> Vec<&Template> {
        self.by_in
            .get(&(func, *addr))
            .map(|ids| ids.iter().map(|&i| &self.templates[i]).collect())
            .unwrap_or_default()
    }

    /// Despacha uma resposta: entre os candidatos do endpoint TIPADO, o
    /// primeiro cujo padrão casa com `data`. `data` é o payload BRUTO
    /// (use [`decode_envelope`] para separar do envelope).
    pub fn match_response<'a>(
        &'a self,
        func: u8,
        addr: &[u8; 4],
        data: &'a [u8],
    ) -> Option<(&'a Template, Vec<&'a [u8]>)> {
        self.for_response(func, addr)
            .into_iter()
            .find_map(|t| t.matches_response(data).map(|vars| (t, vars)))
    }

    /// Despacho por FORMA: entre os templates do endpoint TIPADO, o primeiro
    /// cujo comprimento casa (no `by-len`, um dos comprimentos da tabela).
    /// **Não compara bytes `const`** — ver [`Template::matches_shape`].
    ///
    /// É o que a FSM (`wait_for`) usa para validar respostas do aparelho:
    /// o conteúdo é do hardware e passa pelo decode dos objetos, nunca por
    /// comparação com snapshot de análise (#177).
    pub fn match_shape(&self, func: u8, addr: &[u8; 4], data: &[u8]) -> Option<&Template> {
        self.for_response(func, addr)
            .into_iter()
            .find(|t| t.matches_shape(data))
    }

    /// Todos os templates de REQUEST cujo OUT é o endpoint TIPADO — o
    /// despacho do `MockDevice`: entre os candidatos, vence o PRIMEIRO
    /// (ordem do arquivo, D2/rev.2) cujo `request_pattern` casa com o
    /// payload do frame recebido. Ex.: em `12001002`, o req de CHUNK
    /// (33B var) e o de TABELA (75B nibble-exp) distinguem por len+consts.
    pub fn for_request(&self, func: u8, addr: &[u8; 4]) -> Vec<&Template> {
        self.by_out
            .get(&(func, *addr))
            .map(|ids| ids.iter().map(|&i| &self.templates[i]).collect())
            .unwrap_or_default()
    }

    /// O request recebido casa com ALGUM template cujo OUT é o endpoint?
    /// Devolve o índice do template na ordem do arquivo (para log/erro).
    pub fn match_request(&self, func: u8, addr: &[u8; 4], data: &[u8]) -> Option<usize> {
        self.for_request(func, addr)
            .into_iter()
            .find(|t| {
                t.request_payload
                    .as_ref()
                    .is_some_and(|p| p.matches_data(data))
            })
            .and_then(|t| self.templates.iter().position(|x| std::ptr::eq(x, t)))
    }
}
