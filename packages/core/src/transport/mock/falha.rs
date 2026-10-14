//! o PLANO DE FALHA do test double (device que cai, USB instavel) e o ritmo opt-in.

use std::time::Duration;

use super::MockDevice;
use crate::transport::TransportError;

/// Plano de FALHA do mock (test double): o device pode CAIR no meio da
/// sessão — o cenário para o qual [`TransportError::DeviceGone`] existe.
///
/// O mock é o test double declarado do projeto; a falha entra AQUI (e não num
/// `FailingTransport` novo) para o caminho testado ser EXATAMENTE o de
/// produção: transporte → FSM → actor/command → UI. NUNCA afeta o
/// `RealDevice`: a política de hardware (ADR-4/ADR-5) segue intocada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFault {
    /// Depois de `n` transmissões (`send_raw`) o device desaparece: todo
    /// `send_raw`/`recv_raw`/`open` seguinte devolve
    /// [`TransportError::DeviceGone`]. `0` = já saiu antes da 1ª transação.
    DieAfter(u32),

    /// USB **instável** (intermitente): falhas que NÃO matam o device — o
    /// outro lado do moeda do `DieAfter`. Tudo contra-contado, sem
    /// aleatoriedade (mesma determinismo do golden).
    ///
    /// - `send_every`: a cada n-ésima `send_raw` o envio falha UMA vez com
    ///   [`TransportError::SendFailed`] transitório — a tentativa seguinte
    ///   passa (o contador saiu do múltiplo) e o device segue ABERTO. Como
    ///   a falha precede o parse, o device "não recebeu os bytes": não há
    ///   resposta a enfileirar. `0` desliga.
    /// - `drop_every`: a cada n-ésima mensagem que sairia no `recv_raw` o
    ///   mock a **perde no fio** — consome da fila e devolve
    ///   [`TransportError::RecvTimeout`] com o device saudável (a mensagem
    ///   não reaparece; a seguinte chega). `0` desliga.
    UsbFlaky {
        /// Período do envio: a cada n-ésima `send_raw` falha UMA vez
        /// (múltiplos de n); `0` desliga.
        send_every: u32,
        /// Período da resposta: a cada n-ésima mensagem que sairia no
        /// `recv_raw` o mock a perde no fio; `0` desliga.
        drop_every: u32,
    },
}

impl MockDevice {
    /// Arma um plano de falha: o device CAI depois de `n` transmissões.
    ///
    /// Consumidor típico: testes do shell (actor) e o smoke Tauri, que arma
    /// por env (`GP100_DEBUG_FAULT=die-after:<n>`) — sempre no backend MOCK.
    #[must_use]
    pub fn with_fault(mut self, fault: MockFault) -> Self {
        self.fault = Some(fault);
        self
    }

    /// Transmissões tentadas até agora (diagnóstico/asserções de teste).
    pub fn transactions(&self) -> u32 {
        self.sent
    }

    /// Falhas transitórias de USB emitidas até aqui ([`MockFault::UsbFlaky`]).
    pub fn transient_failures(&self) -> u32 {
        self.transient
    }

    /// Ativa a latência ida/volta com jitter na faixa `[min, max)` —
    /// **opt-in**: fora daqui o mock continua instantâneo (default da suíte).
    ///
    /// - `send_raw` dorme o jitter ANTES de entregar os bytes (escrita +
    ///   processamento no hardware); `Closed`/`DeviceGone` continuam
    ///   precedendo;
    /// - `recv_raw` com mensagem enfileirada dorme o jitter antes do pop
    ///   (chegada ao host);
    /// - `recv_raw` VAZIO espelha o `real.rs`: espera a JANELA inteira do
    ///   `timeout` e só então acusa o silêncio — é o falso positivo de tempo
    ///   (timeout instantâneo com janela de 3s) que este opt-in fecha.
    ///
    /// O jitter vem de um LCG semeado: determinístico, sem dep de `rand`.
    /// Granularidade de milissegundo; `max <= min` devolve sempre `min`.
    #[must_use]
    pub fn with_latency(mut self, min: Duration, max: Duration) -> Self {
        self.latency = Some((min, max));
        self
    }

    /// Próximo valor do jitter (LCG de 64b, constantes MMIX): mesma semente,
    /// mesma sequência, em qualquer máquina.
    pub(super) fn proximo_jitter(&mut self) -> Duration {
        self.jitter = self
            .jitter
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let (min, max) = self.latency.unwrap_or((Duration::ZERO, Duration::ZERO));
        let span = max.as_millis().saturating_sub(min.as_millis()) as u64;
        if span == 0 {
            return min;
        }
        let pick = (self.jitter >> 33) % span;
        Duration::from_millis(min.as_millis() as u64 + pick)
    }

    /// O device já caiu? (o plano `DieAfter` foi consumido)
    pub(super) fn gone(&self) -> bool {
        matches!(self.fault, Some(MockFault::DieAfter(n)) if self.sent > n)
    }

    /// Erro de device ausente (fio cortado), com o motivo do plano de falha.
    pub(super) fn gone_err(&self) -> TransportError {
        TransportError::DeviceGone {
            why: format!(
                "mock: device caiu depois de {} transmissões (MockFault::DieAfter)",
                self.sent
            ),
        }
    }
}
