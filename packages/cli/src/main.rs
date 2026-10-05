//! gp100-cli — interface de linha de comando do GP-100 NextGen Editor.
//!
//! **Papel:** binário de operação/demonstração do [gp100-core](../../core).
//! Por padrão fala SOMENTE com o **MockDevice** (ADR-4/ADR-5 de
//! [docs/DECISIONS.md](../../docs/DECISIONS.md)): nenhum byte sai
//! para hardware com o mock. O modo `--real` (USB-MIDI real via midir/WinMM/ALSA)
//! só se torna plausível após o gate H (H1–H3) e exige **dupla
//! confirmação** — política de hardware da VISION §7: "nenhum fluxo de escrita
//! sai sem captura validada".
//!
//! **Subcomandos (todos contra o mock por default):**
//!   `info`, `list-user-irs`, `dump-preset <pp>`, `set-param --dry-run`,
//!   `save --dry-run`, `upload-ir --dry-run`. A escrita EFETIVA no device real
//!   só existe num binário compilado com a feature `write-verified` (ADR-5),
//!   e mesmo assim só depois do gate H2 — ver o USAGE para as duas camadas.
//!
//! **`--log` (requisito do `docs/H1_CHECKLIST.md`):** grava TODOS os frames de
//! fio (OUT e IN) no MESMO schema das fixtures P4 —
//! `{"s","dir","func","addr","data"}` em hex, `data` = payload puro sem
//! envelope — para `decode_wire.py` e os replays consumirem sem adaptação.
//!
//! **Parser de args:** parser mínimo zero-dep neste arquivo (superfície
//! pequena e estável: 5 subcomandos e 4 flags; manter o binário enxuto na
//! toolchain gnu). Se a superfície crescer, reabrir a decisão com ADR-lite.
//!
//! **Rodar:** `cargo run -p gp100-cli -- info` (gates em
//! `.agents/skills/rust-practices/SKILL.md`).

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use gp100_core::session::Session;
use gp100_core::transport::mock::{MockDevice, MockState};
use gp100_core::transport::{DeviceTransport, TransportError, WireKind};

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
  upload-ir <slot> <arquivo> blob de User IR (§13.7) — exige --dry-run

OPÇÕES
  --dry-run                   imprime os frames e NÃO envia (escritas)
  --log <arquivo>             grava todos os frames (out/in) no schema P4
  --real                      modo hardware (requer --i-know-what-im-doing
                              E o build com --features real-device)
  --i-know-what-im-doing      dupla confirmação do --real (não contorna o gate)
  --help                      esta mensagem

POLÍTICA DE HARDWARE (VISION §7)
  DUAS camadas: (1) --real exige --i-know-what-im-doing; (2) o build precisa
  da feature `real-device` (default OFF). No build de campo, H1 = leitura
  real (roteiro do docs/H1_CHECKLIST.md; NENHUMA escrita).

  ESCRITA REAL = gate H2, e ela é uma TERCEIRA camada, no TRANSPORTE
  (ADR-5): um frame mutante num device real só passa num binário compilado
  com a feature `write-verified` (que depende de `real-device`).

      H1 (leitura)   cargo build --release -p gp100-cli --features real-device
      H2 (escrita)   cargo build --release -p gp100-cli \
                       --features real-device,write-verified

  Sem essa feature a escrita é IMPOSSÍVEL — não há flag, env ou argumento
  que destrave. E o keepalive de boot (12/00020001) é escrita, então o
  boot completo (B5 do H1) exige a feature do H2: o boot não é só leitura.
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
    /// Gate H2, 3º fluxo: upload de User IR (§13.7).
    UploadIr {
        /// Slot de destino do User IR (0..=19) — NAO confundir com o slot da
        /// cadeia de efeito (1..=9 de `set-param`; rev.2 do ADR-6).
        slot: u8,
        /// Caminho do blob binario do IR (multiplo de 15B; §13.7 rev.2
        /// REJEITA o resto, nao padroniza).
        path: PathBuf,
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
    // smoke test pegou o caso `set-param … --dry-run` no fim):
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
    // Passo 2 — GUARDA de política (DUAS camadas):
    // (a) `--real` exige `--i-know-what-im-doing` (dupla confirmação);
    // (b) o build precisa da feature `real-device` (default OFF — ADR-4/ADR-5).
    // Com as duas, o binário de campo abre o RealDevice: LEITURA real = gate
    // H1 (roteiro do H1_CHECKLIST; só leitura); ESCRITA real é uma TERCEIRA
    // camada, no transporte (ADR-5, feature `write-verified`) — e ainda exige
    // o gate H2. independente de mock.
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
        // H2 3º fluxo: upload de User IR. O slot é 0..=19 (§13.7) e o blob
        // tem de ser múltiplo de 15B — a FSM valida o resto (rev.2: rejeita,
        // não padroniza); aqui só o caminho do arquivo.
        "upload-ir" => {
            if pos.len() != 2 {
                return Err(UsageError(
                    "upload-ir exige <slot 0..=19> <arquivo-do-blob>".into(),
                ));
            }
            let path = pos.pop().expect("len == 2 garantido acima");
            let slot = parse_u8(&pos.pop().expect("len >= 1 garantido"))
                .map_err(|e| UsageError(format!("upload-ir slot: {e}")))?;
            if slot >= 20 {
                return Err(UsageError(format!(
                    "upload-ir: slot {slot} fora de 0..=19 (§13.7 — os 20 User IRs)"
                )));
            }
            Command::UploadIr {
                slot,
                path: PathBuf::from(path),
            }
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
// `{"s","t","dir","func","addr","data"}`, `data` = payload puro (sem envelope)
// em hex minúsculo — consumível por `analysis/wirelog.py` (normalizador) e
// pelos replays.
//
// **`t` entrou na v1.1 do H3 (#23) e antes disso nao existia.** Sem relogio,
// o `build_golden.py` nao consegue NADA do que faz com uma captura do
// gp100-core: nem segmentar em fases (gap > 30 s), nem casar OUT→IN com a
// janela de 3 s do ADR-3. O golden inteiro e derivado de tempo, entao um log
// sem `t` nao alimenta o pipeline de spec — que era exatamente o DoD do H3.
// `t` e ms desde a abertura do log, com 1 casa decimal, igual ao
// `make_fixtures.py`, para que as duas capturas vivam na mesma escala.

/// Logger de fio no schema P4.
struct WireLogger {
    file: std::fs::File,
    /// Instante de abertura (relógio monotônico) — origem do `t`.
    t0: std::time::Instant,
}

impl WireLogger {
    /// Cria/trunca o arquivo de log e zera o relógio.
    fn create(path: &std::path::Path) -> Result<Self, String> {
        Ok(Self {
            file: std::fs::File::create(path).map_err(|e| e.to_string())?,
            t0: std::time::Instant::now(),
        })
    }

    /// ms desde a abertura, com 1 casa decimal (escala do `make_fixtures.py`).
    fn t_ms(&self) -> f64 {
        (self.t0.elapsed().as_secs_f64() * 1000.0 * 10.0).round() / 10.0
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
        let t = self.t_ms();
        let line = format!(
            "{{\"s\":\"H3\",\"t\":{t},\"dir\":\"{dir}\",\"func\":\"{func:02x}\",\"addr\":\"{addr_hex}\",\"data\":\"{data}\"}}"
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
    /// Transparente em TUDO, inclusive na POLICY: repassa o `kind` para o
    /// transporte interno. Se o wrapper decidisse o `kind`, ele seria uma
    /// segunda fonte de politica de escrita — e a trava do ADR-5 valeria
    /// só para quem não passa por aqui (D8: o logger nao filtra nada).
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
            // completo é o script do SUITE — o replay o prova).
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
        Command::SetParam { .. } | Command::Save { .. } | Command::UploadIr { .. } => {
            // A política de escrita tem DUAS camadas independentes, e a
            // segunda é a que realmente segura o hardware:
            //
            //   1. `--dry-run` (aqui): imprime os frames e NÃO envia nada.
            //   2. `WRITE_VERIFIED` (o TRANSPORTE, ADR-5): um frame
            //      mutante num device real so passa num binário compilado
            //      com `--features real-device,write-verified`.
            //
            // Por que a segunda não é redundante: o `if !args.dry_run` e
            // uma linha do CLI. Quem viesse falar com o `RealDevice` sem
            // passar pelo `run` — um subcomando novo, um teste de campo, um
            // dia a UI — não encontraria essa linha. O ADR-5 pede a trava
            // no transporte, e a 2 é a que sobrevive ao próximo subcomando.
            if !args.dry_run && !escrita_verificada() {
                eprintln!(
                    "[!] escrita exige --dry-run neste binário.\n    \
                     O dry-run imprime os frames e NÃO envia nada ao device.\n    \
                     Escrita REAL é o gate H2 e depende da feature de \
                     compilação (ADR-5):\n      cargo build --release -p gp100-cli \
                     --features real-device,write-verified"
                );
                return EXIT_POLICY_BLOCKED;
            }
            match &args.command {
                Command::SetParam {
                    slot,
                    code,
                    ctrl,
                    value,
                } => run_set_param(session, *slot, *code, *ctrl, *value, args),
                Command::Save { pp, pp_type, name } => run_save(session, *pp, *pp_type, name, args),
                Command::UploadIr { slot, path } => run_upload_ir(session, *slot, path, args),
                _ => unreachable!("o match externo já filtrou estes comandos"),
            }
        }
    }
}

/// `set-param`: imprime o frame §13.11 e o payload decodificado de volta
/// (prova de ida e volta do codec, como no `set_param_matches_all_fixture_writes`).
///
/// **O `--dry-run` e o que segura o F1 do H2 (#22).** Sem ele, este
/// subcomando ENVIARIA um frame mutante a um preset que o dono escolheu —
/// e a trava de verdade continua sendo a do transporte (ADR-5), que ja foi
/// conferida antes de chegar aqui.
///
/// Sem read-back: o `set_param` e fire-and-forget e a captura nao tem
/// resposta (D4). Por isso a última linha manda olhar o DISPLAY, e nao
/// promete confirmacao que o fio nao pode dar.
fn run_set_param<T: DeviceTransport>(
    session: &mut Session<T>,
    slot: u8,
    code: u32,
    ctrl: u8,
    value: f32,
    args: &Args,
) -> i32 {
    // A validação de slot (1..=9) é da CODEC (§13.11) — o shape não é
    // revalidado aqui (R1: uma fonte de verdade só).
    let frame = match gp100_core::codec::set_param(slot, code, ctrl, value) {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    let hex: String = frame.iter().map(|b| format!("{b:02x}")).collect();
    let rotulo = if args.dry_run { "dry-run " } else { "" };
    println!("{rotulo}set-param (§13.11): {} bytes", frame.len());
    println!("  {hex}");
    match gp100_core::codec::set_param_parse(&frame[13..33]) {
        Ok((c, ctrl2, v)) => {
            println!("  decodificado: code=0x{c:08x} ctrl={ctrl2} value={v}")
        }
        Err(e) => return fail(&e),
    }
    if args.dry_run {
        println!("  (nada enviado — dry-run)");
        return 0;
    }
    if let Err(e) = session.set_param(slot, code, ctrl, value) {
        return fail(&e);
    }
    println!("  ENVIADO ao device (1 frame OUT, ZERO IN — D4, sem read-back).");
    println!("  CONFIRME NO DISPLAY: o valor {value} tem que aparecer no knob.");
    0
}

/// `save --dry-run`: imprime os 9 frames do §13.12 re-derivado (5 meta + ops
/// da S4: op0 ×2 → op1 ×2). ZERO IN esperado (D3: fire-and-forget).
/// `save`: imprime os 9 frames do §13.12 re-derivado (5 meta + ops da S4:
/// op0 ×2 → op1 ×2) e, sem `--dry-run`, ENVIA.
///
/// **Este é o fluxo que PERSISTE** (regra 3.2 do H2_CHECKLIST: só para
/// preset descartável). O save grava o estado ao vivo do preset (§13.12,
/// confirmado na S4), então ele persiste o que o `set-param` acabou de
/// mexer — "salvar para testar" grava o que você mexeu.
///
/// ZERO IN esperado (D3: fire-and-forget). A última linha manda olhar o
/// display DEPOIS de sair e voltar do preset, porque o que se prova aqui é
/// persistência, e não só cache.
fn run_save<T: DeviceTransport>(
    session: &mut Session<T>,
    pp: u16,
    pp_type: u16,
    name: &str,
    args: &Args,
) -> i32 {
    let block = match gp100_core::codec::meta_block(pp, pp_type, name) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };
    let rotulo = if args.dry_run { "dry-run " } else { "" };
    println!("{rotulo}save (§13.12): 9 frames (5 meta + 4 ops); ZERO IN esperado (D3)");
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
    if args.dry_run {
        println!("  (nada enviado — dry-run)");
        return 0;
    }
    if let Err(e) = session.save_preset(pp, pp_type, name) {
        return fail(&e);
    }
    println!("  ENVIADO ao device (9 frames OUT, ZERO IN — D3, sem resposta).");
    println!("  CONFIRME NO DISPLAY: o nome '{name}' tem que aparecer, e continuar");
    println!("  depois de sair do preset e voltar — isso que prova PERSISTENCIA.");
    0
}

/// Erro de protocolo → stderr + exit 3 (tipado, nunca panic — ADR-2).
fn fail(e: &gp100_core::ProtocolError) -> i32 {
    eprintln!("[!] erro de protocolo: {e}");
    EXIT_PROTOCOL_ERROR
}

/// `WRITE_VERIFIED` DESTE binário (ADR-5), por compilação.
///
/// `cfg!` nos dois ramos é o que torna a resposta uma CONSTANTE: não há
/// `env::var`, flag de CLI ou arquivo que a vire. O `--dry-run` acima é a
/// conveniência; isto é a trava.
fn escrita_verificada() -> bool {
    #[cfg(feature = "write-verified")]
    {
        true
    }
    #[cfg(not(feature = "write-verified"))]
    {
        false
    }
}

/// `upload-ir` — gate H2, 3º fluxo (§13.7).
///
/// **O `--dry-run` e honesto sobre o que imprime:** o caminho dos 295
/// chunks (com idx em páginas de 128) e o marcador de fim sao o que o
/// device vai receber, e a FSM e quem monta isso. Aqui o que muda é o
/// destino: no dry-run o blob vai para o log de fio e nada mais.
fn run_upload_ir<T: DeviceTransport>(
    session: &mut Session<T>,
    slot: u8,
    path: &PathBuf,
    args: &Args,
) -> i32 {
    let blob = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[!] não li o blob {path:?}: {e}");
            return EXIT_POLICY_BLOCKED;
        }
    };
    if !blob.len().is_multiple_of(15) {
        // A FSM também rejeita (rev.2 do ADR-6); aqui é para a mensagem
        // dizer o número, que é o que o operador precisa para cortar o WAV.
        eprintln!(
            "[!] blob de {} bytes não é múltiplo de 15 (§13.7 rev.2: a FSM \
             REJEITA, não padroniza — corte ou escolha outro arquivo)",
            blob.len()
        );
        return EXIT_POLICY_BLOCKED;
    }
    let chunks = blob.len() / 15;
    println!(
        "upload-ir slot {slot} ({} bytes = {chunks} chunks de 15B)",
        blob.len()
    );
    if args.dry_run {
        println!("  §13.7: ir_begin -> {chunks} chunks (idx em páginas de 128) -> último chunk 2x");
        println!("  DRY-RUN: frames no log, NADA enviado ao device.");
        return 0;
    }
    match session.upload_ir(slot, &blob) {
        Ok(r) => {
            println!(
                "  enviado: {r} chunks, {acks} ACKs (inclui o dup final §13.7)",
                r = r.chunks,
                acks = r.acks
            );
            println!("  CONFIRME NO DISPLAY antes de considerar o H2 verde.");
            0
        }
        Err(e) => fail(&e),
    }
}

// ═══════════════════════════════════════════════════════════════ testes
//
// Internals do CLI (parser/política/logger) em `#[cfg(test)]` — contratos de
// PROCESSO (exit codes/stdout de binário) ficam p/ M1.x se a UI precisar.

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Transporte que CONTA os frames de escrita e delega no `MockDevice`.
    ///
    /// Ele existe porque o teste anterior (que so olhava o codigo de saida)
    /// passava com o defeito de volta: o `set-param` devolvia 0 sem ter
    /// mandado nada, e um `assert_eq!(codigo, 0)` nao ve diferenca entre
    /// "enviou" e "imprimiu e esqueceu". Aqui a contagem e a prova — e o
    /// mesmo espelho que o `--log` faz em campo.
    ///
    /// A contagem mora num `Rc<Cell>` porque o campo `transport` da
    /// `Session` e privado: o teste precisa ler o que saiu sem fazer o core
    /// expor o transporte so por causa de um teste.
    struct Contador {
        inner: MockDevice,
        writes: Rc<Cell<usize>>,
        ultimos: Rc<RefCell<Vec<Vec<u8>>>>,
    }

    /// O que o `Contador` entrega ao teste: a contagem de writes e a lista
    /// de frames que sairam.
    type Celulas = (Rc<Cell<usize>>, Rc<RefCell<Vec<Vec<u8>>>>);

    impl Contador {
        fn new() -> (Self, Celulas) {
            let mut inner = MockDevice::new().expect("mock");
            inner.open().expect("mock.open");
            let writes = Rc::new(Cell::new(0));
            let ultimos = Rc::new(RefCell::new(Vec::new()));
            (
                Self {
                    inner,
                    writes: Rc::clone(&writes),
                    ultimos: Rc::clone(&ultimos),
                },
                (writes, ultimos),
            )
        }
    }

    impl DeviceTransport for Contador {
        fn open(&mut self) -> Result<(), TransportError> {
            self.inner.open()
        }
        fn close(&mut self) -> Result<(), TransportError> {
            self.inner.close()
        }
        fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
            if kind == WireKind::Write {
                self.writes.set(self.writes.get() + 1);
                self.ultimos.borrow_mut().push(data.to_vec());
            }
            self.inner.send_raw(data, kind)
        }
        fn recv_raw(&mut self, timeout: std::time::Duration) -> Result<Vec<u8>, TransportError> {
            self.inner.recv_raw(timeout)
        }
    }

    fn nao_dry_run() -> Args {
        Args {
            command: Command::Info,
            dry_run: false,
            log: None,
            real: false,
        }
    }

    /// O BUG que o gate H2 (#22) estava esperando: `set-param` e `save`
    /// imprimiam o frame e diziam "(nada enviado — dry-run)" MESMO sem o
    /// `--dry-run`, porque nenhum dos dois chamava a `Session`. No ensaio
    /// contra o mock passava despercebido (o `--log` ficava vazio e o juiz
    /// via "sem evidencia"), e em campo o operador teria acreditado que
    /// gravou.
    ///
    /// A prova e a CONTAGEM de writes no transporte: 1 para o set-param,
    /// 9 para o save (5 do meta + 4 ops, D3).
    #[test]
    fn set_param_sem_dry_run_manda_o_frame_ao_transporte() {
        let (t, (writes, ultimos)) = Contador::new();
        let mut s = Session::new(t);
        let codigo = run_set_param(&mut s, 1, 0x0700_006e, 0, 99.5, &nao_dry_run());
        assert_eq!(codigo, 0, "set-param de campo nao pode falhar");
        assert_eq!(
            writes.get(),
            1,
            "o §13.11 e UM frame de escrita; sem isto o knob nao girou"
        );
        // E o frame exato que a §13.11 manda, byte a byte.
        assert_eq!(
            ultimos.borrow()[0],
            gp100_core::codec::set_param(1, 0x0700_006e, 0, 99.5).unwrap()
        );
    }

    /// O `save` e a escrita que PERSISTE (regra 3.2 do H2_CHECKLIST). Um
    /// `save` que so imprime e o pior defeito possivel neste gate: o
    /// operador le "enviado" no terminal, acredita, e o preset nunca foi
    /// gravado — e nao ha como desfazer a suposicao.
    #[test]
    fn save_sem_dry_run_manda_os_9_frames() {
        let (t, (writes, _ultimos)) = Contador::new();
        let mut s = Session::new(t);
        let codigo = run_save(&mut s, 0, 4, "H2 TESTE", &nao_dry_run());
        assert_eq!(codigo, 0, "save de campo nao pode falhar");
        assert_eq!(
            writes.get(),
            9,
            "o §13.12 re-derivado e 5 writes de meta + 4 ops (D3)"
        );
    }

    /// O `--dry-run` continua sendo o que NAO envia: os dois subcomandos
    /// impressos antes do conserto e a trava de conveniencia do ADR-5 nao
    /// podem ter sumido junto com o defeito.
    #[test]
    fn dry_run_continua_imprimindo_sem_mandar() {
        let dry = Args {
            command: Command::Info,
            dry_run: true,
            log: None,
            real: false,
        };
        let (t, (writes, _ultimos)) = Contador::new();
        let mut s = Session::new(t);
        assert_eq!(run_set_param(&mut s, 1, 0x0700_006e, 0, 99.5, &dry), 0);
        assert_eq!(
            writes.get(),
            0,
            "dry-run nao pode mandar nada — e o que segura o F1 de campo"
        );
        assert_eq!(run_save(&mut s, 0, 4, "H2 TESTE", &dry), 0);
        assert_eq!(writes.get(), 0, "save em dry-run nao grava nada");
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
            "upload-ir",
        ] {
            assert!(USAGE.contains(term), "USAGE não documenta {term}");
        }
    }

    /// **A constante do ADR-5 reflete a feature de compilação.** É o teste
    /// que garante que o binário de campo do H1 (compilado SEM a feature)
    /// não se declara verificado — e que o do H2 se declara.
    ///
    /// Os dois ramos são `cfg!` de propósito: com a feature ligada o teste
    /// continua rodando e confere o outro valor.
    #[test]
    fn escrita_verificada_segue_a_feature() {
        #[cfg(not(feature = "write-verified"))]
        assert!(
            !escrita_verificada(),
            "build sem `write-verified` NÃO pode se declarar verificado — é o \
             que segura o H1 (leitura) de escrever"
        );
        #[cfg(feature = "write-verified")]
        assert!(
            escrita_verificada(),
            "build com `write-verified` se declara verificado (gate H2)"
        );
    }

    /// O USAGE diz COMO destravar a escrita. Sem isso, quem pega a recusa
    /// "escrita bloqueada" em campo conclui que o binário está quebrado.
    #[test]
    fn o_uso_diz_como_destravar_a_escrita() {
        assert!(
            USAGE.contains("write-verified"),
            "o USAGE precisa nomear a feature que destrava a escrita (ADR-5)"
        );
        assert!(
            USAGE.contains("upload-ir"),
            "o 3º fluxo do H2 precisa estar no USAGE"
        );
    }

    /// `upload-ir` valida o slot no parser: 20 é o primeiro inválido (os 20
    /// User IRs são 0..=19, §13.12). Um slot 20 aceito aqui viraria um
    /// `begin` para um slot que o device não tem.
    #[test]
    fn upload_ir_recusa_slot_fora_de_0_a_19() {
        for slot in ["0", "19"] {
            assert!(
                parse_args(args(&["upload-ir", slot, "blob.bin"]).into_iter()).is_ok(),
                "slot {slot} é válido"
            );
        }
        let err = parse_args(args(&["upload-ir", "20", "blob.bin"]).into_iter())
            .expect_err("slot 20 não existe");
        assert!(err.0.contains("0..=19"), "{}", err.0);

        let err = parse_args(args(&["upload-ir", "0x14", "blob.bin"]).into_iter())
            .expect_err("0x14 = 20, também inválido");
        assert!(err.0.contains("0..=19"), "{}", err.0);
    }

    /// `upload-ir` sem os 2 posicionais é erro de uso, não um caminho que
    /// chegue a ler arquivo.
    #[test]
    fn upload_ir_exige_slot_e_arquivo() {
        assert!(parse_args(args(&["upload-ir"]).into_iter()).is_err());
        assert!(parse_args(args(&["upload-ir", "0"]).into_iter()).is_err());
    }

    /// O nome tem espaço: `upload-ir` recebe o caminho por `pos`, não por
    /// flag, então o arquivo não pode ser confundido com o slot.
    #[test]
    fn upload_ir_guarda_o_caminho_do_blob() {
        let parsed =
            parse_args(args(&["upload-ir", "3", "C:/meus/test ir.bin"]).into_iter()).expect("ok");
        match parsed.command {
            Command::UploadIr { slot, path } => {
                assert_eq!(slot, 3);
                assert_eq!(path, PathBuf::from("C:/meus/test ir.bin"));
            }
            _ => panic!("comando errado"),
        }
    }

    /// Logger P4: linha no schema `{"s","t","dir","func","addr","data"}` com
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
        assert_eq!(v["s"], "H3");
        assert_eq!(v["dir"], "out");
        assert_eq!(v["func"], "12");
        assert_eq!(v["addr"], "10030002");
        // payload puro: 20B nibble-exp = 40 chars hex (envelope fora).
        assert_eq!(v["data"].as_str().map(|s| s.len()), Some(40));
        // `t` é o que faltava: sem relogio o build_golden não segmenta nem
        // casa OUT→IN numa captura do core (o DoD do H3).
        let t = v["t"].as_f64().expect("t numerico");
        assert!((0.0..5_000.0).contains(&t), "t em ms desde a abertura: {t}");
    }

    /// O `t` é MONOTÔNICO: um pipeline que fatia a sessão em fases por gap
    /// (30 s) e casa OUT→IN por janela (3 s) depende de ordem + relogio.
    #[test]
    fn o_relogio_do_log_avanca() {
        let dir = std::env::temp_dir().join("gp100_cli_test_log_t");
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let path = dir.join("t.jsonl");
        let mut log = WireLogger::create(&path).expect("arquivo");
        let frame = gp100_core::codec::set_param(3, 0x0700_006e, 0, 15.0).expect("vetor");
        log.record("out", &frame);
        std::thread::sleep(std::time::Duration::from_millis(25));
        log.record("in", &frame);
        let content = std::fs::read_to_string(&path).expect("conteúdo");
        let ts: Vec<f64> = content
            .lines()
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(l).expect("JSONL")["t"]
                    .as_f64()
                    .expect("t")
            })
            .collect();
        assert_eq!(ts.len(), 2, "2 frames gravados");
        assert!(ts[1] > ts[0], "t avancou entre os frames: {ts:?}");
        assert!(ts[1] - ts[0] >= 20.0, "delta cobre o sleep: {ts:?}");
    }
}
