//! a EXECUCAO dos subcomandos: o despacho do `run` e os tres fluxos de
//! escrita (`set-param`, `save`, `upload-ir`), com a trava do ADR-5.

use std::path::PathBuf;

use gp100_core::session::Session;
use gp100_core::transport::mock::MockState;
use gp100_core::transport::DeviceTransport;

use super::{Args, Command};
use super::{EXIT_POLICY_BLOCKED, EXIT_PROTOCOL_ERROR};

/// Executa o subcomando e devolve o código de saída.
pub(super) fn run<T: DeviceTransport>(
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
        Command::ReadState5 => match session.read_state5() {
            // HEX CRU de propósito: a semântica de 12001012 ainda não é
            // decode, é medição (R1: não inventar layout). O `--log` guarda
            // a captura para o `build_golden`/juiz.
            Ok(entradas) => {
                println!(
                    "12001012 — {} entradas (44B esperado cada):",
                    entradas.len()
                );
                for (i, p) in entradas.iter().enumerate() {
                    let hex: String = p.iter().map(|b| format!("{b:02x}")).collect();
                    println!("  [{i}] {}B: {hex}", p.len());
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
pub(super) fn run_set_param<T: DeviceTransport>(
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
pub(super) fn run_save<T: DeviceTransport>(
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
pub(super) fn escrita_verificada() -> bool {
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
