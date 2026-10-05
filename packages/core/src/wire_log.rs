//! wire_log — o log de fio no schema P4, como **decorador de transporte**.
//!
//! **POR QUE ISSO MORRE AQUI E NAO NO CLI.** O `--log` nasceu no `gp100-cli`
//! como um struct privado de `main.rs`. O `DeviceActor` do app precisa da
//! mesma coisa, e a outra opção seria copiar os 90 linhas — duas
//! implementações do **mesmo schema**, que é justamente o tipo de coisa que
//! diverge em silêncio quando um consome a captura da outra.
//!
//! Ele mora no core porque é um **decorador**: `DeviceTransport` em
//! `DeviceTransport`, bytes entram e bytes saem, e o `kind` da política de
//! escrita é repassado intacto (ver o `send_raw`). Ele **não** alarga a
//! fronteira do ADR-4 — que é sobre *o que atravessa a fronteira*, não
//! sobre quantas camadas a atravessam. A `Session` continua dona única do
//! stream (D8); o logger não lê, não filtra e não decide nada.
//!
//! **O schema P4 é o contrato dos Circuitos.** `scripts/h1_compare.py` e
//! `scripts/h2_compare.py` leem estes arquivos e comparam frame a frame com o
//! golden — e o `build_golden.py` precisa do `t` (ms desde a abertura) para
//! segmentar a captura em fases e casar OUT→IN na janela de 3 s do ADR-3. Um
//! log sem `t` não alimenta o pipeline de spec. Por isso o `t` é obrigatório
//! e nao opcional.
//!
//! **Uso no CLI:** `LoggingTransport { inner, logger }`.
//! **Uso no app:** o mesmo wrapper, ligado por um command de sessão — o
//! operador faz a sessão no editor, exporta o `.jsonl` e roda o juiz.
//!
//! **Robustez:** um frame sem envelope NUNCA derruba a sessão (truncamento
//! SEM-HDR é dado conhecido do proxy, não bug): avisa no `stderr` e segue.
//! Uma falha de escrita no arquivo também só avisa — perder o log é ruim,
//! perder a sessão por causa do log é pior.

use std::io::Write;
use std::time::{Duration, Instant};

use crate::transport::{DeviceTransport, TransportError, WireKind};

/// Rótulo da sessão no campo `s` de cada linha do log.
///
/// `H3` é o rótulo do golden (as quatro capturas de origem); `APP` marca uma
/// sessão feita no editor. O juiz **não** filtra por este campo — ele
///_segmenta por tempo e casa por addr — mas o rótulo torna o arquivo
/// autoexplicativo quando alguém abre a captura no editor de texto.
pub const SESSION_APP: &str = "APP";

/// Logger de fio no schema P4.
#[derive(Debug)]
pub struct WireLogger {
    file: std::fs::File,
    /// Instante de abertura (relógio monotônico) — origem do `t`.
    t0: Instant,
    /// Rótulo da sessão (`s` no log).
    session: &'static str,
}

impl WireLogger {
    /// Cria/trunca o arquivo de log e zera o relógio.
    ///
    /// # Erros
    /// String com o erro do SO se o arquivo não puder ser criado.
    pub fn create(path: &std::path::Path) -> Result<Self, String> {
        Ok(Self {
            file: std::fs::File::create(path).map_err(|e| e.to_string())?,
            t0: Instant::now(),
            session: SESSION_APP,
        })
    }

    /// Cria com rótulo de sessão explícito.
    ///
    /// # Erros
    /// String com o erro do SO se o arquivo não puder ser criado.
    pub fn with_session(path: &std::path::Path, session: &'static str) -> Result<Self, String> {
        Ok(Self {
            file: std::fs::File::create(path).map_err(|e| e.to_string())?,
            t0: Instant::now(),
            session,
        })
    }

    /// ms desde a abertura, com 1 casa decimal (a escala do `make_fixtures.py`
    /// — as duas capturas precisam viver na mesma escala para o juiz casar).
    fn t_ms(&self) -> f64 {
        (self.t0.elapsed().as_secs_f64() * 1000.0 * 10.0).round() / 10.0
    }

    /// Registra um frame completo (envelope SysEx cru).
    ///
    /// Nunca derruba a sessão: um frame sem envelope avisa e segue.
    pub fn record(&mut self, dir: &str, frame: &[u8]) {
        let (func, addr, payload) = match crate::golden::decode_envelope(frame) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[!] frame sem envelope ({e}); não gravado no log");
                return;
            }
        };
        let data: String = payload.iter().map(|b| format!("{b:02x}")).collect();
        let addr_hex: String = addr.iter().map(|b| format!("{b:02x}")).collect();
        let t = self.t_ms();
        let line = format!(
            "{{\"s\":\"{}\",\"t\":{t},\"dir\":\"{dir}\",\"func\":\"{func:02x}\",\
             \"addr\":\"{addr_hex}\",\"data\":\"{data}\"}}",
            self.session
        );
        if let Err(e) = writeln!(self.file, "{line}") {
            eprintln!("[!] falha ao gravar log: {e}");
        }
    }
}

/// Transporte transparente que registra cada frame OUT/IN no logger — o log
/// vê exatamente o que o "fio" vê, sem tocar na `Session` (D8: consumidor
/// único continua sendo a `Session`; o logger não lê nem filtra nada).
#[derive(Debug)]
pub struct LoggingTransport<T: DeviceTransport> {
    /// Transporte real, delegando tudo.
    pub inner: T,
    /// Logger ligado; `None` = sessão sem log (custo zero).
    pub logger: Option<WireLogger>,
}

impl<T: DeviceTransport> LoggingTransport<T> {
    /// Embrulha um transporte sem log ligado (o padrão do CLI).
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            logger: None,
        }
    }

    /// Liga o logger, criando/truncando o arquivo.
    ///
    /// # Erros
    /// String com o erro do SO se o arquivo não puder ser criado — e nesse
    /// caso **o transporte segue sem log** (a sessão é mais importante que o
    /// log; o operator é avisado no `stderr`).
    pub fn enable_log(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.logger = Some(WireLogger::create(path)?);
        Ok(())
    }

    /// Liga o logger com rótulo de sessão.
    ///
    /// # Erros
    /// String com o erro do SO se o arquivo não puder ser criado.
    pub fn enable_log_session(
        &mut self,
        path: &std::path::Path,
        session: &'static str,
    ) -> Result<(), String> {
        self.logger = Some(WireLogger::with_session(path, session)?);
        Ok(())
    }
}

impl<T: DeviceTransport> DeviceTransport for LoggingTransport<T> {
    fn open(&mut self) -> Result<(), TransportError> {
        self.inner.open()
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.inner.close()
    }

    /// Transparente em TUDO, inclusive na POLICY: repassa o `kind` para o
    /// transporte interno. Se o wrapper decidisse o `kind`, ele seria uma
    /// segunda fonte de politica de escrita — e a trava do ADR-5 valeria só
    /// para quem não passa por aqui (D8: o logger nao filtra nada).
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        if let Some(l) = self.logger.as_mut() {
            l.record("out", data);
        }
        self.inner.send_raw(data, kind)
    }

    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        let msg = self.inner.recv_raw(timeout)?;
        if let Some(l) = self.logger.as_mut() {
            l.record("in", &msg);
        }
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::set_param;

    /// Transporte de mentira que registra o que atravessou.
    struct Eco(std::cell::RefCell<Vec<Vec<u8>>>);

    impl DeviceTransport for Eco {
        fn open(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        fn send_raw(&mut self, d: &[u8], _k: WireKind) -> Result<(), TransportError> {
            self.0.borrow_mut().push(d.to_vec());
            Ok(())
        }
        fn recv_raw(&mut self, _t: Duration) -> Result<Vec<u8>, TransportError> {
            Err(TransportError::RecvTimeout { timeout_ms: 0 })
        }
    }

    /// **O contrato com osarmonios** — e este é o teste que importa: o arquivo
    /// gerado tem de ter EXATAMENTE o schema que `h1_compare.py` e
    /// `make_fixtures.py` leem. Um `{ "t": ... }` faltando e o log inteiro
    /// deixa de alimentar o pipeline de spec.
    #[test]
    fn o_log_tem_o_schema_p4_que_o_juiz_le() {
        let dir = std::env::temp_dir().join("gp100-wire-log-p4");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("sessao.jsonl");
        let _ = std::fs::remove_file(&path);

        let mut t = LoggingTransport::new(Eco(Default::default()));
        t.enable_log(&path).expect("log ligado");
        // §13.11 real: slot 3, AMP Gain, 15.0 — o vetor do knobs.jsonl.
        let frame = set_param(3, 0x0700_006e, 0, 15.0).expect("frame");
        t.send_raw(&frame, WireKind::Write).expect("send");

        let txt = std::fs::read_to_string(&path).expect("log existe");
        let linha = txt.lines().next().expect("uma linha");
        let v: serde_json::Value = serde_json::from_str(linha).expect("JSON valido");

        assert_eq!(v["s"], "APP", "rotulo da sessao");
        assert_eq!(v["dir"], "out", "direcao");
        assert_eq!(v["func"], "12", "FUNC em hex, minusculo");
        assert_eq!(v["addr"], "10030002", "addr 4B BE do §13.11");
        assert!(
            v["t"].as_f64().is_some_and(|t| t >= 0.0),
            "o `t` e OBRIGATORIO: sem ele o juiz nao segmenta a captura em fases"
        );
        assert_eq!(
            v["data"].as_str().unwrap_or("").len(),
            40,
            "20B real = 40 nibble"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Sem log ligado, o wrapper e **invisível**: o transporte interno ve a
    /// mesma coisa. Um wrapper que mudasse o comportamento na ausencia de log
    /// seria um bug dificil de achar.
    #[test]
    fn sem_log_o_transporte_e_invisivel() {
        let mut t = LoggingTransport::new(Eco(Default::default()));
        let frame = set_param(3, 0x0700_006e, 0, 15.0).expect("frame");
        t.send_raw(&frame, WireKind::Write).expect("send");
        assert_eq!(t.inner.0.borrow().len(), 1, "o frame chegou ao interno");
    }

    /// A politica do ADR-5 atravessa o wrapper: `WireKind::Write` chega ao
    /// interno como `Write`, e nao reclassificado. Se o wrapper decidisse o
    /// `kind`, a trava valeria so para quem nao passa por aqui.
    #[test]
    fn a_politica_de_escrita_atravessa_o_wrapper_intacta() {
        struct ViuWrite(std::cell::Cell<bool>);

        impl DeviceTransport for ViuWrite {
            fn open(&mut self) -> Result<(), TransportError> {
                Ok(())
            }
            fn close(&mut self) -> Result<(), TransportError> {
                Ok(())
            }
            fn send_raw(&mut self, _d: &[u8], k: WireKind) -> Result<(), TransportError> {
                self.0.set(k == WireKind::Write);
                Ok(())
            }
            fn recv_raw(&mut self, _t: Duration) -> Result<Vec<u8>, TransportError> {
                Err(TransportError::RecvTimeout { timeout_ms: 0 })
            }
        }

        let mut t = LoggingTransport::new(ViuWrite(std::cell::Cell::new(false)));
        let frame = set_param(3, 0x0700_006e, 0, 15.0).expect("frame");
        t.send_raw(&frame, WireKind::Write).expect("send");
        assert!(t.inner.0.get(), "o Write chegou como Write");
    }

    /// Um frame sem envelope NAO derruba a sessao (truncamento SEM-HDR e dado
    /// conhecido do proxy, nao bug do codec): o logger avisa e segue.
    #[test]
    fn frame_sem_envelope_nao_derruba_a_sessao() {
        let dir = std::env::temp_dir().join("gp100-wire-log-semhdr");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("sem.jsonl");
        let _ = std::fs::remove_file(&path);

        let mut t = LoggingTransport::new(Eco(Default::default()));
        t.enable_log(&path).expect("log");
        // SEM-HDR: os fixtures ja excluem, mas o proxy real produz.
        t.send_raw(&[0x01, 0x02, 0x03], WireKind::Read)
            .expect("o send passa; so o log nao grava");
        let txt = std::fs::read_to_string(&path).expect("log existe");
        assert_eq!(txt.lines().count(), 0, "nao grava, mas nao quebra");
        let _ = std::fs::remove_file(&path);
    }
}
