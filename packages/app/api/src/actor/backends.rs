//! Backends de transporte do actor: a trait [`DeviceBackend`] (o `MockState` a
//! mais, que a fronteira de BYTES CRUOS do core não expõe — ADR-4) e as três
//! implementações do app: o `MockDevice`, o `DesconectadoDevice` (#150, build
//! sem a feature) e o `ProcuraAparelho` (#150, build de campo que re-enumera as
//! portas a cada reconexão).

use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};
use gp100_core::wire_log::LoggingTransport;

/// Estado snapshot do mock (alias curto; o tipo vive no core).
pub(super) type MockState = gp100_core::transport::mock::MockState;

/// **O tipo de transporte que o build de campo carrega** (feature
/// `real-device`): `Box<dyn DeviceBackend>` — a trait deste crate, que ja
/// estende `DeviceTransport`, entao o `RealDevice` entra pela mesma fila do
/// actor sem que nenhum tipo concrete precise conhecer o outro.
///
/// **(#150)** O app sem aparelho não monta transporte de mentira: o estado é
/// [`Backend::Desligado`](super::tipos::Backend::Desligado) com um transporte
/// honesto — o [`DesconectadoDevice`]
/// no build sem a feature, e o `ProcuraAparelho` (que re-enumera as portas a
/// cada `device_conectar`) no build de campo.
#[cfg(feature = "real-device")]
pub type AppDevice = Box<dyn DeviceBackend + Send>;

/// Fonte do estado local do backend — só o mock tem.
///
/// Existe **neste crate**, e nao como metodo do `DeviceTransport`, para nao
/// alargar a fronteira de bytes do core (ADR-4: a trait trafega SysEx cru e
/// nada mais). O `RealDevice` devolve `None`, e o app degrada com honestidade
/// em vez de mostrar zero como se fosse um dado.
pub trait DeviceBackend: DeviceTransport + Send {
    /// Estado local do mock, quando existir.
    fn local_state(&self) -> Option<MockState>;

    /// Mensagens que chegaram ao device e **ninguém pediu** (a inbox do
    /// `MockDevice`, D7 na perspectiva do device).
    ///
    /// **Limite conhecido do aparelho real:** o `RealDevice` guarda o excedente
    /// numa fila interna de RX e nao expoe peek, entao devolve vazio. O log de
    /// pushes da UI (`pending_pushes`) e por isso **mock-only por enquanto** —
    /// o espelho fiel no aparelho precisa de um `peek` no `real.rs`, e isso
    /// esta anotado em `docs/referencias/REAL_DEVICE_GAP.md` §6.
    fn drain_inbox(&mut self) -> Vec<Vec<u8>>;
}

impl DeviceBackend for MockDevice {
    fn local_state(&self) -> Option<MockState> {
        Some(self.state().clone())
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        MockDevice::drain_inbox(self)
    }
}

/// O `LoggingTransport` (do core) continua sendo um backend: o log e um
/// **decorador transparente**, entao o `DeviceBackend` atravessa por ele
/// sem mudar nada. E o que permite ligar o log no MEIO da sessao, sem
/// reiniciar o device nem reconstruir a fila.
impl<T: DeviceBackend> DeviceBackend for LoggingTransport<T> {
    fn local_state(&self) -> Option<MockState> {
        self.inner.local_state()
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        self.inner.drain_inbox()
    }
}

/// `Box<dyn DeviceBackend>` tambem e um backend — e o que deixa o build de
/// campo passar o `RealDevice` pela MESMA fila do mock, sem duplicar nada.
impl<T: DeviceBackend + ?Sized> DeviceBackend for Box<T> {
    fn local_state(&self) -> Option<MockState> {
        (**self).local_state()
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        (**self).drain_inbox()
    }
}

/// Transporte do app **sem aparelho** (issue #150) para builds sem a feature
/// `real-device`.
///
/// **Não é mock.** Ele não inventa preset, nome, cadeia ou resposta: toda
/// operação de tráfego é recusada, porque não existe fio. Ele existe para o
/// app subir com a janela viva e um estado HONESTO (`backend: "none"` +
/// motivo legível) em vez de derrubar o binário — ou pior, montar o mock e
/// fingir que é o aparelho. No build de campo quem assume este papel é o
/// `ProcuraAparelho`, que tem ação real no reconectar.
///
/// `cfg_attr` de par com o `desligado`: construído só no build SEM a feature.
#[cfg_attr(feature = "real-device", allow(dead_code))]
#[derive(Debug)]
pub struct DesconectadoDevice;

impl DeviceTransport for DesconectadoDevice {
    fn open(&mut self) -> Result<(), TransportError> {
        // OpenFailed, e não DeviceGone: o aparelho está AUSENTE (nunca
        // esteve na sessão) — é a semântica do erro no core.
        Err(TransportError::OpenFailed {
            why: "aparelho não conectado".into(),
        })
    }

    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    fn send_raw(&mut self, _data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        Err(TransportError::Closed)
    }

    fn recv_raw(&mut self, _timeout: std::time::Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::Closed)
    }
}

impl DeviceBackend for DesconectadoDevice {
    fn local_state(&self) -> Option<MockState> {
        // Sem aparelho, sem estado local: o mesmo neutro do `RealDevice`.
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }
}

/// Transporte que **procura o aparelho a cada `open`** (issue #150, build de
/// campo): o app que subiu sem USB mantém o botão "Reconectar" com ação real
/// — o open re-enumera as portas MIDI e monta o `RealDevice` na hora, em vez
/// de desistir para sempre.
#[cfg(feature = "real-device")]
#[derive(Debug, Default)]
pub struct ProcuraAparelho {
    /// O aparelho, quando um open já achou as portas.
    device: Option<gp100_core::transport::real::RealDevice>,
}

#[cfg(feature = "real-device")]
impl DeviceTransport for ProcuraAparelho {
    fn open(&mut self) -> Result<(), TransportError> {
        // Re-enumeração REAL a cada tentativa: ligar o aparelho DEPOIS de
        // abrir o app tem que funcionar sem reiniciar o editor.
        match gp100_core::transport::real::RealDevice::new() {
            Ok(d) => {
                self.device = Some(d);
                Ok(())
            }
            Err(e) => {
                self.device = None;
                Err(e)
            }
        }
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.device = None;
        Ok(())
    }

    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        match self.device.as_mut() {
            Some(d) => d.send_raw(data, kind),
            None => Err(TransportError::Closed),
        }
    }

    fn recv_raw(&mut self, timeout: std::time::Duration) -> Result<Vec<u8>, TransportError> {
        match self.device.as_mut() {
            Some(d) => d.recv_raw(timeout),
            None => Err(TransportError::Closed),
        }
    }

    fn e_aparelho(&self) -> bool {
        // Quando um open achou o aparelho, os frames saem para o REAL —
        // e a trava de faixa do pp (#132) precisa valer.
        self.device.as_ref().is_some()
    }
}

#[cfg(feature = "real-device")]
impl DeviceBackend for ProcuraAparelho {
    fn local_state(&self) -> Option<MockState> {
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        match self.device.as_mut() {
            Some(d) => d.drain_inbox(),
            None => Vec::new(),
        }
    }
}

#[cfg(feature = "real-device")]
impl DeviceBackend for gp100_core::transport::real::RealDevice {
    fn local_state(&self) -> Option<MockState> {
        // O aparelho nao tem estado local: o que ele mostra na tela, a app
        // precisa ler do fio. Encher isto seria a app inventando dado.
        None
    }

    fn drain_inbox(&mut self) -> Vec<Vec<u8>> {
        // Limite conhecido, nao um "nao ha nada": ver a doc da trait.
        Vec::new()
    }
}
