//! gp100-cli — interface de linha de comando do GP-100 NextGen Editor (M0.7).
//!
//! **Papel:** binário de operação/demonstração do [gp100-core](../../gp100-core).
//! Por padrão fala SOMENTE com o **MockDevice** (ADR-4/ADR-5 de
//! [docs/DECISIONS.md](../../docs/DECISIONS.md); ROADMAP M0.5): nenhum byte sai
//! para hardware na Fase M0. O modo `--real` (USB-MIDI real via midir/WinMM/ALSA)
//! só se torna plausível após o gate H (ROADMAP H1–H3) e exige **dupla
//! confirmação** — política de hardware da VISION §7: "nenhum fluxo de escrita
//! sai sem captura validada".
//!
//! **Subcomandos (todos contra o mock por default; ROADMAP M0.7):**
//!   `info`, `list-user-irs`, `dump-preset <pp>`, `set-param --dry-run`,
//!   `save --dry-run`. Escrita EFETIVA não existe nesta fase (DoD da issue
//!   manda dry-run; escrita real só no gate H2, `WRITE_VERIFIED`).
//!
//! **`--log` (requisito do `docs/H1_CHECKLIST.md`):** grava TODOS os frames de
//! fio (OUT e IN) no MESMO schema das fixtures P4 —
//! `{"s","dir","func","addr","data"}` em hex, `data` = payload puro sem
//! envelope — para `decode_wire.py` e os replays consumirem sem adaptação.
//!
//! **Parser de args:** decisão da issue era "clap?" — resolvido por parser
//! mínimo zero-dep neste arquivo (superfície pequena e estável: 5 subcomandos
//! e 4 flags; manter o binário enxuto na toolchain gnu). Se a superfície
//! crescer na M1, reabrir a decisão ali com ADR-lite.
//!
//! **Rodar:** `cargo run -p gp100-cli -- info` (gates em
//! `.agents/skills/rust-practices/SKILL.md`).

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use gp100_core::session::Session;
use gp100_core::transport::mock::{MockDevice, MockState};
use gp100_core::transport::{DeviceTransport, TransportError};

/// Código de saída para violação de uso/política de hardware.
///
/// Convenção estável para scripts/CI: `2` = erro de USO/política (distinto de
/// pânicos/aborts). Qualquer script que automatize o CLI pode confiar nele.
const EXIT_POLICY_BLOCKED: i32 = 2;

/// Código de saída para erro de protocolo/transação (`ProtocolError`).
const EXIT_PROTOCOL_ERROR: i32 = 3;

/// Uso documentado (impresso por `--help` e por erro de uso).
const USAGE: &str = "\
gp100-cli — operação do GP-100 NextGen Editor (mock por default)

USO: gp100-cli [opções] <subcomando> [args]

SUBCOMANDOS
  info                        estado do mock (pp, nome, tipo, nº de presets)
  list-user-irs               tabela dos 20 User IRs (nome por slot)
  dump-preset <pp>            select + 9 páginas de estado do preset (hex)
  set-param <slot> <code> <ctrl> <value>
                              knob da cadeia (§13.11) — exige --dry-run
  save <pp> <pp-type> <name>  metadados + ops (§13.12) — exige --dry-run

OPÇÕES
  --dry-run                   imprime os frames e NÃO envia (escritas)
  --log <arquivo>             grava todos os frames (out/in) no schema P4
  --real                      modo hardware (requer --i-know-what-im-doing
                              E o build com --features real-device)
  --i-know-what-im-doing      dupla confirmação do --real (não contorna o gate)
  --help                      esta mensagem

POLÍTICA DE HARDWARE (VISION §7 / ROADMAP H1–H2 / core-dev regra 8)
  DUAS camadas: (1) --real exige --i-know-what-im-doing; (2) o build precisa
  da feature `real-device` (default OFF). No build de campo, H1 = leitura
  real (roteiro do docs/H1_CHECKLIST.md; NENHUMA escrita); escrita real só
  pós-H2 com WRITE_VERIFIED — a guarda de --dry-run permanece sempre.
  Números aceitam hex (0x0100) ou decimal.
";

/// Comando requisitado na linha de comando.
#[derive(Debug)]
enum Command {
    Info,
    ListUserIrs,
    DumpPreset {
        /// pp do preset (u16 BE no fio; hex `0x0100` ou decimal).
        pp: u16,
    },
    SetParam {
        /// Posição na CADEIA 1..=9 (§13.11) — não é o slot de IR.
        slot: u8,
        /// effectCode `(nibble<<24)|index` (u32; hex/dec).
        code: u32,
        /// Índice do controle no `controls[]` do dicionário.
        ctrl: u8,
        /// Valor físico (float32 LE no fio).
        value: f32,
    },
    Save {
        pp: u16,
        pp_type: u16,
        name: String,
    },
}

/// Linha de comando parseada.
#[derive(Debug)]
struct Args {
    command: Command,
    dry_run: bool,
    log: Option<PathBuf>,
    /// `--real` JÁ VALIDADO: exige `--i-know-what-im-doing` e um build com
    /// a feature `real-device` (recusas feitas no parser — política).
    real: bool,
}

/// Erro de parse (impresso com usage).
#[derive(Debug)]
struct UsageError(String);

/// Parseia a linha de comando. A política `--real` é verificada AQUI (não
/// precisa de estado externo) — recusada sempre, com ou sem confirmação.
fn parse_args<I: Iterator<Item = String>>(args: I) -> Result<Args, UsageError> {
    // Passo 1 — flags globais em QUALQUER posição (antes/depois do subcomando;
    // smoke test da M0.7 pegou o caso `set-param … --dry-run` no fim):
    // primeiro separa flags de posicionais, depois resolve o subcomando.
    let mut dry_run = false;
    let mut log: Option<PathBuf> = None;
    let mut real = false;
    let mut acknowledge = false;
    let mut positional: Vec<String> = Vec::new();
    let mut it = args.into_iter().peekable();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--dry-run" => dry_run = true,
            "--log" => {
                let path = it
                    .next()
                    .ok_or_else(|| UsageError("--log exige um caminho".into()))?;
                log = Some(PathBuf::from(path));
            }
            "--help" | "-h" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--real" => real = true,
            "--i-know-what-im-doing" => acknowledge = true,
            _ => positional.push(a),
        }
    }
    // Passo 2 — GUARDA de política (regra 8 do core-dev, DUAS camadas):
    // (a) `--real` exige `--i-know-what-im-doing` (dupla confirmação);
    // (b) o build precisa da feature `real-device` (default OFF — ADR-4/ADR-5).
    // Com as duas, o binário de campo abre o RealDevice: LEITURA real = gate
    // H1 (roteiro do H1_CHECKLIST; só leitura); ESCRITA real segue bloqueada
    // pela guarda de dry-run (pós-H2 WRITE_VERIFIED) — independente de mock.
    if real && !acknowledge {
        return Err(UsageError(
            "modo --real exige também --i-know-what-im-doing (dupla confirmação; \
             política de hardware VISION §7)"
                .into(),
        ));
    }
    #[cfg(not(feature = "real-device"))]
    if real {
        return Err(UsageError(
            "modo --real BLOQUEADO nesta build: compilada SEM a feature \
             `real-device` (default OFF — política ADR-4/ADR-5). O binário de \
             campo compila com: cargo build --release -p gp100-cli \
             --features real-device"
                .into(),
        ));
    }
    // Passo 3 — subcomando = 1º posicional; o resto são os argumentos dele.
    let mut pos = positional.into_iter();
    let sub = pos
        .next()
        .ok_or_else(|| UsageError("faltou o subcomando (veja --help)".into()))?;
    let mut pos: Vec<String> = pos.collect();
    let command = match sub.as_str() {
        "info" => Command::Info,
        "list-user-irs" => Command::ListUserIrs,
        "dump-preset" => {
            let pp_raw = pos
                .pop()
                .ok_or_else(|| UsageError("dump-preset exige <pp>".into()))?;
            let pp = parse_u16(&pp_raw).map_err(|e| UsageError(format!("dump-preset: {e}")))?;
            Command::DumpPreset { pp }
        }
        "set-param" => {
            if pos.len() != 4 {
                return Err(UsageError(
                    "set-param exige <slot 1..=9> <code> <ctrl> <value>".into(),
                ));
            }
            let value: f32 = pos[3]
                .parse()
                .map_err(|_| UsageError(format!("value inválido: {}", pos[3])))?;
            Command::SetParam {
                slot: parse_u8(&pos[0]).map_err(|e| UsageError(format!("slot: {e}")))?,
                code: parse_u32(&pos[1]).map_err(|e| UsageError(format!("code: {e}")))?,
                ctrl: parse_u8(&pos[2]).map_err(|e| UsageError(format!("ctrl: {e}")))?,
                value,
            }
        }
        "save" => {
            if pos.len() != 3 {
                return Err(UsageError("save exige <pp> <pp-type> <name>".into()));
            }
            let name = pos.pop().expect("len == 3 garantido acima");
            let pp_type = parse_u16(&pos.pop().expect("len >= 2 garantido"))
                .map_err(|e| UsageError(format!("pp-type: {e}")))?;
            let pp = parse_u16(&pos.pop().expect("len >= 1 garantido"))
                .map_err(|e| UsageError(format!("pp: {e}")))?;
            Command::Save { pp, pp_type, name }
        }
        other => {
            return Err(UsageError(format!(
                "subcomando desconhecido: {other} (veja --help)"
            )))
        }
    };
    Ok(Args {
        command,
        dry_run,
        log,
        real,
    })
}

/// u8 dec ou hex (`0x..`).
fn parse_u8(s: &str) -> Result<u8, String> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => {
            u8::from_str_radix(hex, 16).map_err(|e: std::num::ParseIntError| e.to_string())
        }
        None => s
            .parse::<u8>()
            .map_err(|e: std::num::ParseIntError| e.to_string()),
    }
}

/// u16 dec ou hex (`0x..`).
fn parse_u16(s: &str) -> Result<u16, String> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => {
            u16::from_str_radix(hex, 16).map_err(|e: std::num::ParseIntError| e.to_string())
        }
        None => s
            .parse::<u16>()
            .map_err(|e: std::num::ParseIntError| e.to_string()),
    }
}

/// u32 dec ou hex (`0x..`).
fn parse_u32(s: &str) -> Result<u32, String> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => {
            u32::from_str_radix(hex, 16).map_err(|e: std::num::ParseIntError| e.to_string())
        }
        None => s
            .parse::<u32>()
            .map_err(|e: std::num::ParseIntError| e.to_string()),
    }
}

// ═════════════════════════════════════════════════════════════ logger P4
//
// Mesmo schema das fixtures P4 (make_fixtures.py): 1 linha JSON por msg,
// `{"s","dir","func","addr","data"}`, `data` = payload puro (sem envelope)
// em hex minúsculo — consumível por `decode_wire.py` e pelos replays.

/// Logger de fio no schema P4.
struct WireLogger {
    file: std::fs::File,
}

impl WireLogger {
    /// Cria/trunca o arquivo de log.
    fn create(path: &std::path::Path) -> Result<Self, String> {
        Ok(Self {
            file: std::fs::File::create(path).map_err(|e| e.to_string())?,
        })
    }

    /// Registra um frame completo (envelope SysEx cru). Frame sem envelope
    /// NUNCA derruba a sessão (truncamento SEM-HDR é dado conhecido do
    /// proxy) — avisa no stderr e segue.
    fn record(&mut self, dir: &str, frame: &[u8]) {
        let (func, addr, payload) = match gp100_core::golden::decode_envelope(frame) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[!] frame sem envelope ({e}); não gravado no log");
                return;
            }
        };
        let data: String = payload.iter().map(|b| format!("{b:02x}")).collect();
        let addr_hex: String = addr.iter().map(|b| format!("{b:02x}")).collect();
        let line = format!(
            "{{\"s\":\"H1\",\"dir\":\"{dir}\",\"func\":\"{func:02x}\",\"addr\":\"{addr_hex}\",\"data\":\"{data}\"}}"
        );
        if let Err(e) = writeln!(self.file, "{line}") {
            eprintln!("[!] falha ao gravar log: {e}");
        }
    }
}

/// Transporte transparente que registra cada frame OUT/IN no logger — o log
/// vê exatamente o que o "fio" vê, sem tocar na Session (D8: consumidor
/// único continua sendo a Session; o logger não lê nem filtra nada).
struct LoggingTransport<T: DeviceTransport> {
    inner: T,
    logger: Option<WireLogger>,
}

impl<T: DeviceTransport> DeviceTransport for LoggingTransport<T> {
    fn open(&mut self) -> Result<(), TransportError> {
        self.inner.open()
    }
    fn close(&mut self) -> Result<(), TransportError> {
        self.inner.close()
    }
    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if let Some(l) = self.logger.as_mut() {
            l.record("out", data);
        }
        self.inner.send_raw(data)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        let msg = self.inner.recv_raw(timeout)?;
        if let Some(l) = self.logger.as_mut() {
            l.record("in", &msg);
        }
        Ok(msg)
    }
}

// ═══════════════════════════════════════════════════════════════ execução

fn main() {
    // Passo 1 — parse (uso/política = exit 2 com usage no stderr).
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(UsageError(msg)) => {
            eprintln!("[!] {msg}\n\n{USAGE}");
            std::process::exit(EXIT_POLICY_BLOCKED);
        }
    };

    // Passo 2 — transporte: MOCK por default (ADR-4/ADR-5) ou REAL com a
    // feature + dupla confirmação (validadas no parser). Dispatch por trait
    // object (a Session continua dona única do stream, D8). O snapshot de
    // estado só existe no mock (o device real não tem estado local).
    #[cfg_attr(not(feature = "real-device"), allow(unused_mut))]
    let (mut transport, snapshot): (
        LoggingTransport<Box<dyn DeviceTransport>>,
        Option<MockState>,
    ) = if args.real {
        #[cfg(feature = "real-device")]
        {
            use gp100_core::transport::real::RealDevice;
            let real = match RealDevice::new() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[!] device real não abriu: {e}");
                    std::process::exit(EXIT_PROTOCOL_ERROR);
                }
            };
            (
                LoggingTransport {
                    inner: Box::new(real),
                    logger: None,
                },
                None,
            )
        }
        #[cfg(not(feature = "real-device"))]
        {
            unreachable!("--real sem feature é recusado no parser")
        }
    } else {
        let mut mock = match MockDevice::new() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[!] mock não inicializou (embedado corrompido?): {e}");
                std::process::exit(EXIT_PROTOCOL_ERROR);
            }
        };
        if let Err(e) = mock.open() {
            eprintln!("[!] mock.open() falhou: {e}");
            std::process::exit(EXIT_PROTOCOL_ERROR);
        }
        let snapshot = mock.state().clone();
        (
            LoggingTransport {
                inner: Box::new(mock),
                logger: None,
            },
            Some(snapshot),
        )
    };

    // Passo 3 — logger envolve o transporte (o log vê o que o "fio" vê,
    // idêntico no mock e no real); Session sobre o transporte (D1–D8).
    transport.logger = match &args.log {
        Some(path) => match WireLogger::create(path) {
            Ok(l) => Some(l),
            Err(e) => {
                eprintln!("[!] --log {path:?}: {e}");
                std::process::exit(EXIT_POLICY_BLOCKED);
            }
        },
        None => None,
    };
    let mut session = Session::new(&mut transport);

    // Passo 4 — executa o subcomando (protocolo = exit 3).
    let code = run(&mut session, snapshot.as_ref(), &args);
    std::process::exit(code);
}

/// Executa o subcomando e devolve o código de saída.
fn run<T: DeviceTransport>(
    session: &mut Session<T>,
    snapshot: Option<&MockState>,
    args: &Args,
) -> i32 {
    match &args.command {
        Command::Info => {
            // MOCK: `info` = estado local sem tráfego de fio (o boot/scan
            // completo é o script do SUITE — o replay da M0.6 o prova).
            // REAL: `info` = a leitura de campo B1 do H1_CHECKLIST (a leitura
            // de verdade acontece nos comandos; aqui só identificamos).
            match snapshot {
                Some(st) => {
                    println!("device : MockDevice (default — política ADR-4/ADR-5)");
                    println!("presets: {}", st.preset_count);
                    println!("pp     : 0x{:04x}", st.current_pp);
                    println!("nome   : {}", st.current_name);
                    println!("tipo   : {}", st.current_pp_type);
                    let ocupados = st.ir_crcs.iter().filter(|&&c| c != 0).count();
                    println!("IRs    : {ocupados}/20 slots com CRC de fábrica");
                }
                None => {
                    println!("device : RealDevice (gate H1 — SÓ LEITURA; escrita = H2)");
                    println!("(o estado real vem das leituras: list-user-irs / dump-preset)");
                }
            }
            0
        }
        Command::ListUserIrs => match session.list_user_irs() {
            Ok(t) => {
                println!("slot  nome");
                println!("----  ------");
                for (slot, name) in &t.slots {
                    println!(
                        "{slot:4}  {}",
                        if name.is_empty() { "(vazio)" } else { name }
                    );
                }
                0
            }
            Err(e) => fail(&e),
        },
        Command::DumpPreset { pp } => {
            // dump-preset = select + 9 páginas: as MESMAS rotas do §13.10
            // provadas pelo replay do boot (D1; meta6 do select vem no
            // endpoint de resposta provado).
            if let Err(e) = session.select_preset(*pp) {
                return fail(&e);
            }
            println!("preset 0x{pp:04x}:");
            for page in 0u8..=8u8 {
                match session.state_page(page) {
                    Ok(p) => {
                        let hex: String = p.raw.iter().map(|b| format!("{b:02x}")).collect();
                        println!("  pág {page} ({}B): {hex}", p.raw.len());
                    }
                    Err(e) => return fail(&e),
                }
            }
            0
        }
        Command::SetParam { .. } | Command::Save { .. } => {
            // Política da M0.7 (DoD): escrita real NÃO existe nesta fase —
            // só --dry-run (imprime frames). Efetiva = pós-H2 WRITE_VERIFIED.
            if !args.dry_run {
                eprintln!(
                    "[!] escrita exige --dry-run nesta fase (escrita efetiva = \
                     gate H2, WRITE_VERIFIED).\n    o dry-run imprime os frames \
                     e NÃO envia nada ao device."
                );
                return EXIT_POLICY_BLOCKED;
            }
            match &args.command {
                Command::SetParam {
                    slot,
                    code,
                    ctrl,
                    value,
                } => run_set_param(*slot, *code, *ctrl, *value),
                Command::Save { pp, pp_type, name } => run_save(*pp, *pp_type, name),
                _ => unreachable!("o match externo já filtrou estes comandos"),
            }
        }
    }
}

/// `set-param --dry-run`: imprime o frame §13.11 completo e o payload
/// decodificado de volta (prova de ida e volta do codec, como no
/// `set_param_matches_all_fixture_writes`).
fn run_set_param(slot: u8, code: u32, ctrl: u8, value: f32) -> i32 {
    // A validação de slot (1..=9) é da CODEC (§13.11) — o shape não é
    // revalidado aqui (R1: uma fonte de verdade só).
    let frame = match gp100_core::codec::set_param(slot, code, ctrl, value) {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    let hex: String = frame.iter().map(|b| format!("{b:02x}")).collect();
    println!("dry-run set-param (§13.11): {} bytes", frame.len());
    println!("  {hex}");
    match gp100_core::codec::set_param_parse(&frame[13..33]) {
        Ok((c, ctrl2, v)) => {
            println!("  decodificado: code=0x{c:08x} ctrl={ctrl2} value={v}")
        }
        Err(e) => return fail(&e),
    }
    println!("  (nada enviado — dry-run)");
    0
}

/// `save --dry-run`: imprime os 9 frames do §13.12 re-derivado (5 meta + ops
/// da S4: op0 ×2 → op1 ×2). ZERO IN esperado (D3: fire-and-forget).
fn run_save(pp: u16, pp_type: u16, name: &str) -> i32 {
    let block = match gp100_core::codec::meta_block(pp, pp_type, name) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };
    println!("dry-run save (§13.12): 9 frames (5 meta + 4 ops); ZERO IN esperado (D3)");
    // Os frames vêm de codec::write_frame — o PONTO ÚNICO de montagem de
    // envelope (ADR-6: o binário não monta SysEx; imprime o que a lib faz).
    for (addr, payload) in &block {
        let m = gp100_core::codec::write_frame(addr, payload);
        let hex: String = m.iter().map(|b| format!("{b:02x}")).collect();
        let addr_hex: String = addr.iter().map(|b| format!("{b:02x}")).collect();
        println!("  meta {addr_hex} ({}B): {hex}", m.len());
    }
    for op in [0u8, 0, 1, 1] {
        let m = gp100_core::codec::write_frame(
            &[0x00, 0x02, 0x00, 0x00],
            &gp100_core::codec::op_payload(op),
        );
        let hex: String = m.iter().map(|b| format!("{b:02x}")).collect();
        println!("  op   00020000 (op={op}, {}B): {hex}", m.len());
    }
    println!("  (nada enviado — dry-run)");
    0
}

/// Erro de protocolo → stderr + exit 3 (tipado, nunca panic — ADR-2).
fn fail(e: &gp100_core::ProtocolError) -> i32 {
    eprintln!("[!] erro de protocolo: {e}");
    EXIT_PROTOCOL_ERROR
}

// ═══════════════════════════════════════════════════════════════ testes
//
// Internals do CLI (parser/política/logger) em `#[cfg(test)]` — contratos de
// PROCESSO (exit codes/stdout de binário) ficam p/ M1.x se a UI precisar.

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Camada 1 da política: `--real` sem `--i-know-what-im-doing` é recusado
    /// SEMPRE (dupla confirmação é pré-condição, feature ou não).
    #[test]
    fn real_sozinho_e_bloqueado() {
        let err = parse_args(args(&["--real", "info"]).into_iter()).unwrap_err();
        assert!(err.0.contains("--i-know-what-im-doing"));
    }

    /// Camada 2 da política (só no build DEFAULT, sem a feature):
    /// `--real --i-know-what-im-doing` ainda é recusado — a dupla confirmação
    /// é necessária mas NÃO suficiente sem o binário de campo.
    #[cfg(not(feature = "real-device"))]
    #[test]
    fn real_com_ack_sem_feature_e_bloqueado() {
        let parsed = parse_args(args(&["--real", "--i-know-what-im-doing", "info"]).into_iter());
        let err = parsed.unwrap_err();
        assert!(err.0.contains("BLOQUEADO"));
    }

    /// No build de CAMPO (feature on), `--real --i-know-what-im-doing` passa
    /// o parser (o `RealDevice::new()` decide em runtime — device presente).
    #[cfg(feature = "real-device")]
    #[test]
    fn real_com_ack_com_feature_passa_o_parser() {
        let parsed = parse_args(args(&["--real", "--i-know-what-im-doing", "info"]).into_iter());
        assert!(
            parsed.is_ok(),
            "feature on: parser aceita (abertura é runtime)"
        );
    }

    /// O parser ACEITA escrita sem --dry-run (o bloqueio é no `run`, com a
    /// mensagem que explica o gate H2) — aqui só provamos a separação.
    #[test]
    fn parser_aceita_escrita_sem_dry_run() {
        let parsed = parse_args(args(&["set-param", "1", "0x03000001", "0", "42"]).into_iter())
            .expect("parse ok");
        assert!(!parsed.dry_run);
    }

    /// set-param parseia hex e decimal.
    #[test]
    fn set_param_parseia_argumentos() {
        let parsed = parse_args(args(&["set-param", "0x3", "0x0700006e", "0", "15.5"]).into_iter())
            .expect("ok");
        match parsed.command {
            Command::SetParam {
                slot,
                code,
                ctrl,
                value,
            } => {
                assert_eq!(slot, 3);
                assert_eq!(code, 0x0700_006e);
                assert_eq!(ctrl, 0);
                assert!((value - 15.5).abs() < f32::EPSILON);
            }
            _ => panic!("comando errado"),
        }
    }

    /// save parseia os 3 posicionais (pp hex, tipo dec, nome com espaços).
    #[test]
    fn save_parseia_argumentos() {
        let parsed =
            parse_args(args(&["save", "0x0000", "4", "It's GP100"]).into_iter()).expect("ok");
        match parsed.command {
            Command::Save { pp, pp_type, name } => {
                assert_eq!(pp, 0);
                assert_eq!(pp_type, 4);
                assert_eq!(name, "It's GP100");
            }
            _ => panic!("comando errado"),
        }
    }

    /// dump-preset sem argumento = erro de uso.
    #[test]
    fn dump_preset_exige_pp() {
        assert!(parse_args(args(&["dump-preset"]).into_iter()).is_err());
    }

    /// Subcomando desconhecido = erro de uso.
    #[test]
    fn subcomando_desconhecido_e_erro() {
        assert!(parse_args(args(&["fly-to-the-moon"]).into_iter()).is_err());
    }

    /// O USAGE documenta as flags de política e todos os subcomandos (o
    /// `--help` de processo é testado na M1.x, se a UI precisar).
    #[test]
    fn usage_documenta_a_superficie() {
        for term in [
            "--dry-run",
            "--log",
            "--real",
            "--i-know-what-im-doing",
            "info",
            "list-user-irs",
            "dump-preset",
            "set-param",
            "save",
        ] {
            assert!(USAGE.contains(term), "USAGE não documenta {term}");
        }
    }

    /// Logger P4: linha no schema `{"s","dir","func","addr","data"}` com
    /// payload puro (sem envelope) em hex.
    #[test]
    fn logger_grava_schema_p4() {
        let dir = std::env::temp_dir().join("gp100_cli_test_log");
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let path = dir.join("t.jsonl");
        let mut log = WireLogger::create(&path).expect("arquivo");
        // Vetor real da fixture knobs: slot 3, Bog RedM @ 15.0.
        let frame = gp100_core::codec::set_param(3, 0x0700_006e, 0, 15.0).expect("vetor");
        log.record("out", &frame);
        let content = std::fs::read_to_string(&path).expect("conteúdo");
        let v: serde_json::Value =
            serde_json::from_str(content.lines().next().expect("1 linha")).expect("JSONL");
        assert_eq!(v["s"], "H1");
        assert_eq!(v["dir"], "out");
        assert_eq!(v["func"], "12");
        assert_eq!(v["addr"], "10030002");
        // payload puro: 20B nibble-exp = 40 chars hex (envelope fora).
        assert_eq!(v["data"].as_str().map(|s| s.len()), Some(40));
    }
}
