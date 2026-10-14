//! o PARSER da linha de comando: a superficie (subcomando/flags) e a
//! POLITICA do `--real`. Os TIPOS e o `USAGE` ficam no handle.

use std::path::PathBuf;

use super::{Args, Command, UsageError, USAGE};

/// Parseia a linha de comando. A política `--real` é verificada AQUI (não
/// precisa de estado externo) — recusada sempre, com ou sem confirmação.
pub(super) fn parse_args<I: Iterator<Item = String>>(args: I) -> Result<Args, UsageError> {
    // Passo 1 — flags globais em QUALQUER posição (antes/depois do subcomando;
    // smoke test pegou o caso `set-param … --dry-run` no fim):
    // primeiro separa flags de posicionais, depois resolve o subcomando.
    let mut dry_run = false;
    let mut log: Option<PathBuf> = None;
    let mut real = false;
    let mut acknowledge = false;
    let mut mock_device = false;
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
            "--mock-device" => mock_device = true,
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
    if real && mock_device {
        return Err(UsageError(
            "--real e --mock-device são exclusivos: um abre o APARELHO, o outro \
             sela o MOCK (teste/offline)"
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
        "read-state5" => Command::ReadState5,
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
        mock_device,
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
