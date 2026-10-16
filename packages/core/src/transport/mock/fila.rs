//! a FILA IN por endpoint (D1/D7): o que o mock entrega e o que se injeta nele.

use super::respostas::{build_from_example, envelope_of, push_template};
use super::MockDevice;
use crate::golden::GoldenFile;
use crate::ProtocolError;

impl MockDevice {
    /// Drena a inbox de respostas/pushes NÃO consumidos (ordem FIFO global
    /// — a mais antiga de qualquer endpoint primeiro, mesma regra do
    /// `recv_raw`): observação de pushes pendentes pós-operação (D7 na
    /// perspectiva do DEVICE; o dono do `Session` enxerga o seu backlog
    /// via `pending_pushes`). Usado pelo DeviceActor para reemitir
    /// pushes como eventos da UI.
    pub fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            let oldest = self
                .inbox
                .iter()
                .filter(|(_, q)| !q.is_empty())
                .map(|(k, _)| *k)
                .min();
            match oldest {
                Some(k) => {
                    if let Some(q) = self.inbox.get_mut(&k) {
                        if !q.is_empty() {
                            out.push(q.remove(0));
                        }
                    }
                }
                None => break,
            }
        }
        out
    }

    /// Enfileira um push não solicitado com payload VERBATIM (D7) — o
    /// caminho do teste de push intercalado (a FSM filtra via backlog).
    pub fn queue_push(&mut self, func: u8, addr: [u8; 4], payload: &[u8]) {
        let msg = envelope_of(func, addr, payload);
        self.inbox.entry((func, addr)).or_default().push(msg);
    }

    /// Enfileira um push PADRÃO, construído pelo golden (exemplo congelado
    /// do template `push` do endpoint). `desired_len` escolhe o sub-padrão
    /// em respostas `by-len` (ex.: 75 para a tabela de IRs em `12001002`).
    ///
    /// # Erros
    /// [`ProtocolError::InvalidShape`] se não há template push no endpoint
    /// ou a resposta não é buildável (D5: o mock nunca inventa forma).
    pub fn queue_push_template(
        &mut self,
        func: u8,
        addr: [u8; 4],
        desired_len: Option<usize>,
    ) -> Result<(), ProtocolError> {
        let golden = GoldenFile::embedded()?;
        let tpl = push_template(golden, addr)?;
        let msg = build_from_example(tpl, desired_len, None)?;
        self.inbox.entry((func, addr)).or_default().push(msg);
        Ok(())
    }

    /// Enfileira uma mensagem CRUA — fora do envelope do GP-100 (é o que o
    /// transporte devolve para o framing da família, §5).
    ///
    /// A chave da fila é `(0x00, 00000000)`, a MENOR possível: o `recv_raw`
    /// entrega a mensagem mais antiga pela chave mínima e o ACK do SnapTone
    /// tem que sair antes de qualquer push da família GP-100 que esteja
    /// pendente — a resposta é da operação corrente, não do histórico.
    pub fn queue_raw(&mut self, msg: Vec<u8>) {
        self.inbox.entry((0x00, [0u8; 4])).or_default().push(msg);
    }
}
