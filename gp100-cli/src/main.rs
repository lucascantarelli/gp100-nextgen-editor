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
//! **Subcomandos planejados para o M0.7** (todos contra o mock por default):
//! `info`, `list-user-irs`, `dump-preset <pp>`, `set-param --dry-run`,
//! `save --dry-run`. O parser de argumentos (clap?) é escolhido no M0.7 —
//! este esqueleto só implementa a guarda de política.
//!
//! **Rodar:** `cargo run -p gp100-cli` (toolchain windows-gnu pinada; ver
//! skill `.agents/skills/rust-practices` para os gates de qualidade).

/// Código de saída para violação da política de hardware.
///
/// Convenção estável para scripts/CI: `2` = erro de USO/política (distinto de
/// pânicos/aborts). Qualquer script que automatize o CLI pode confiar nele.
const EXIT_POLICY_BLOCKED: i32 = 2;

fn main() {
    // Passo 1 — coletar argumentos crus.
    // O esqueleto não usa parser de args: o M0.7 introduzirá subcomandos
    // (info/list-user-irs/...) e SÓ ENTÃO a flag --i-know-what-im-doing
    // passa a ser aceita (e exigida) junto de --real.
    let args: Vec<String> = std::env::args().collect();

    // Passo 2 — GUARDA de política de hardware (não negociável).
    // `--real` sozinho é bloqueado SEMPRE neste estágio: o transporte real
    // ainda não existe (M0.5) e o gate H (H1–H3) não rodou. Mesmo depois do
    // M0.7, a regra permanece: sem `--i-know-what-im-doing` junto, exit 2.
    if args.iter().any(|a| a == "--real") {
        // Erros de política vão para STDERR (nunca stdout — stdout é para
        // dados consumíveis por pipes; convenção POSIX).
        eprintln!("[!] modo --real exige também --i-know-what-im-doing");
        eprintln!("    (política de hardware: ROADMAP H1-H2; só 3 fluxos capturados)");
        std::process::exit(EXIT_POLICY_BLOCKED);
    }

    // Passo 3 — modo mock (único caminho ativo até o gate H passar).
    // A mensagem é um lembrete de escopo: os subcomandos reais chegam no M0.7.
    println!("gp100-cli (mock) — M0.7 entregará info/list-user-irs/dump-preset/set-param/save");

    // Passo 4 — manter o link com o core vivo no esqueleto.
    // Sem nenhum uso real, a dependência `gp100-core` não gera aviso, mas o
    // toque explícito documenta a intenção; REMOVER no M0.3, quando o CLI
    // consumir o golden-file de verdade (Template::build_request/matches).
    let _ = gp100_core::SYSEX_EOX;
}
