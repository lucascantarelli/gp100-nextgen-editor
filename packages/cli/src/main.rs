//! gp100-cli — interface de linha de comando do GP-100 NextGen Editor.
//!
//! **Papel:** binário de operação/demonstração do [gp100-core](../../core).
//! Por padrão fala SOMENTE com o **MockDevice** (ADR-4/ADR-5 de
//! [docs/dev/DECISIONS.md](../../docs/dev/DECISIONS.md)): nenhum byte sai
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
//! **`--log` (requisito do `docs/arquivo/H1_CHECKLIST.md`):** grava TODOS os frames de
//! fio (OUT e IN) no MESMO schema das fixtures P4 —
//! `{"s","dir","func","addr","data"}` em hex, `data` = payload puro sem
//! envelope — para `decode_wire.py` e os replays consumirem sem adaptação.
//!
//! **Parser de args:** parser mínimo zero-dep neste arquivo (superfície
//! pequena e estável: 5 subcomandos e 4 flags; manter o binário enxuto na
//! toolchain gnu). Se a superfície crescer, reabrir a decisão com ADR-lite.
//!
//! ── CONGELADO PARA NOVAS CAPACIDADES (decisão do owner, 05/10) ────────────
//! O produto é o **app**. Esta é hoje a decisão do owner: "manter, mas
//! congelar". O que ela significa, sem ambiguidade:
//!
//!   - **Capability nova NÃO entra aqui.** Vai para o `app/api` (command +
//!     porta em `ui/src/ipc/`) ou não existe.
//!   - **O binário continua compilando, testando e no gate.** Ele não está
//!     moribundo: é o executável com que o core é exercitado nos gates H1–H3,
//!     e `--log`/`--dry-run` continuam sendo o insumo do juiz de campo.
//!   - **Correção de bug aqui é bem-vinda; capacidade nova, não.** Se o CLI
//!     divergir do app, a correção entra — divergir para sempre, não.
//!
//! **POR QUE (e por que a decisão pode ser revista).** O CLI era o dono de
//! parte das capacidades do core; o app tinha um subconjunto DIFERENTE. Os
//! passos 4/4b/4c do `docs/referencias/REAL_DEVICE_GAP.md` §6 fecharam isso, e o passo 6b
//! tirou o último motivo técnico para ele existir (era a única cobertura de
//! CoreMIDI do transporte — o `RealDevice` do app agora é compilado para
//! macOS na CI). Hoje o app é um **superset estrito**: o core tem 12
//! capacidades, o CLI expõe 9, o app expõe essas 9 mais o SnapTone.
//!
//! Revisão: quando a sessão de campo (#17) sair assinada pelo aparelho, a
//! pergunta "o CLI ainda serve a alguém que não o app?" tem resposta medida —
//! e essa resposta, e não este arquivo, decide o futuro do crate.
//!
//! **Rodar:** `cargo run -p gp100-cli -- info` (gates em
//! `.agents/skills/rust-practices/SKILL.md`).

mod args;
mod execucao;

use crate::args::parse_args;
use crate::execucao::run;
#[cfg(test)]
use crate::execucao::{escrita_verificada, run_save, run_set_param};

use std::path::PathBuf;

use gp100_core::session::Session;
use gp100_core::transport::mock::{MockDevice, MockState};
// O logger de fio e o MESMO do app: duas implementacoes do mesmo schema
// P4 divergem em silencio, e quem consome a captura (h1_compare.py /
// h2_compare.py) nao distingue as duas.
use gp100_core::transport::DeviceTransport;
use gp100_core::wire_log::LoggingTransport;

/// Código de saída para violação de uso/política de hardware.
///
/// Convenção estável para scripts/CI: `2` = erro de USO/política (distinto de
/// pânicos/aborts). Qualquer script que automatize o CLI pode confiar nele.
const EXIT_POLICY_BLOCKED: i32 = 2;

/// Código de saída para erro de protocolo/transação (`ProtocolError`).
const EXIT_PROTOCOL_ERROR: i32 = 3;

/// Uso documentado (impresso por `--help` e por erro de uso).
const USAGE: &str = "\
gp100-cli — operação do GP-100 NextGen Editor (transporte MOCK por default)

USO: gp100-cli [opções] <subcomando> [args]

SUBCOMANDOS
  info                        estado do mock (pp, nome, tipo, nº de presets)
  list-user-irs               tabela dos 20 User IRs (nome por slot)
  read-state5                 as 5 leituras cruas de 11/12001012 (estado;
                              era chamado de setlist, mas a GP-100 não tem
                              setlist — PROTOCOL §13.3 / GP100_DEVICE §5#1)
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
  --mock-device               SELO do transporte mock (teste/offline; exclui --real)
  --help                      esta mensagem

POLÍTICA DE HARDWARE (VISION §7)
  DUAS camadas: (1) --real exige --i-know-what-im-doing; (2) o build precisa
  da feature `real-device` (default OFF NESTE crate; no APP ela é default ON
  desde a #165 — o CLI é ferramenta de campo e segue explícito). No build de
  campo, H1 = leitura real (roteiro do docs/arquivo/H1_CHECKLIST.md; NENHUMA escrita).
  O harness de teste DECLARA o mock com --mock-device (conflita com --real).

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
    /// As 5 leituras cruas de `11/12001012` (T2 do boot; semântica não
    /// decifrada — medição de campo, PROTOCOL §13.3).
    ReadState5,
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
    /// `--mock-device`: SELO do transporte mock (teste/offline). É o caminho
    /// default do CLI — a flag existe para o harness DECLARAR a intenção e
    /// para impedir `--real` acidental num script (exclusão verificada no
    /// parser). Não muda o comportamento do mock.
    mock_device: bool,
}

/// Erro de parse (impresso com usage).
#[derive(Debug)]
struct UsageError(String);

// ═════════════════════════════════════════════════════════════ logger P4
//
// Mesmo schema das fixtures P4 (make_fixtures.py): 1 linha JSON por msg,
// `{"s","t","dir","func","addr","data"}`, `data` = payload puro (sem envelope)
// em hex minúsculo — consumível pelo wirelog do projeto (normalizador de fio) e
// pelos replays.
//
// **`t` entrou na v1.1 do H3 (#23) e antes disso nao existia.** Sem relogio,
// o `build_golden.py` nao consegue NADA do que faz com uma captura do
// gp100-core: nem segmentar em fases (gap > 30 s), nem casar OUT→IN com a
// janela de 3 s do ADR-3. O golden inteiro e derivado de tempo, entao um log
// sem `t` nao alimenta o pipeline de spec — que era exatamente o DoD do H3.
// `t` e ms desde a abertura do log, com 1 casa decimal, igual ao
// `make_fixtures.py`, para que as duas capturas vivam na mesma escala.

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

    // Passo 2 — transporte: MOCK por default do CLI (motor de TESTE/offline;
    // `--mock-device` sela essa escolha) ou REAL com a feature + dupla
    // confirmação (validadas no parser). No APP o default é o aparelho
    // (#165); aqui a ferramenta de campo segue explícita. Dispatch por trait
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
        if args.mock_device {
            eprintln!("[i] --mock-device: transporte MOCK declarado (teste/offline)");
        }
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
        // O rotulo `H3` e o das capturas de origem: o log do CLI alimenta
        // o mesmo pipeline de spec que elas.
        Some(path) => match gp100_core::wire_log::WireLogger::with_session(path, "H3") {
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

// ═══════════════════════════════════════════════════════════════ testes
//
// Internals do CLI (parser/política/logger) em `#[cfg(test)]` — contratos de
// PROCESSO (exit codes/stdout de binário) ficam p/ M1.x se a UI precisar.

#[cfg(test)]
mod tests {
    use super::*;
    use gp100_core::transport::{TransportError, WireKind};
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
            mock_device: false,
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
    ///
    /// O valor e o vetor REAL do H2 (`knobs.jsonl` linha 1: slot 3, AMP
    /// Gain, 15.0) e nao um numero inventado — o `99.5` que o runbook
    /// mandava esta agora recusado pela trava da #110, e um teste que
    /// voltasse a usa-lo ensinaria o numero que derrubou o aparelho.
    #[test]
    fn set_param_sem_dry_run_manda_o_frame_ao_transporte() {
        let (t, (writes, ultimos)) = Contador::new();
        let mut s = Session::new(t);
        let codigo = run_set_param(&mut s, 3, 0x0700_006e, 0, 15.0, &nao_dry_run());
        assert_eq!(codigo, 0, "set-param de campo nao pode falhar");
        assert_eq!(
            writes.get(),
            1,
            "o §13.11 e UM frame de escrita; sem isto o knob nao girou"
        );
        // E o frame exato que a §13.11 manda, byte a byte.
        assert_eq!(
            ultimos.borrow()[0],
            gp100_core::codec::set_param(3, 0x0700_006e, 0, 15.0).unwrap()
        );
    }

    /// **A trava da #110 no nivel do CLI (#110, ADR-10).** O `99.5` que o
    /// runbook do H2 mandava derrubou o aparelho num assert do firmware
    /// (`audio.c:1828`), sem nenhum aviso no fio. Aqui o comando recusa
    /// ANTES de montar o frame: sai codigo de erro, NAO sai frame, e o
    /// operador le o teto na stderr.
    #[test]
    fn set_param_fora_da_faixa_falha_sem_mandar_nada() {
        let (t, (writes, _ultimos)) = Contador::new();
        let mut s = Session::new(t);
        let codigo = run_set_param(&mut s, 3, 0x0700_006e, 0, 99.5, &nao_dry_run());
        assert_eq!(
            codigo, EXIT_PROTOCOL_ERROR,
            "99.5 acima do teto 99 tem que falhar, nao 'enviado com sucesso'"
        );
        assert_eq!(
            writes.get(),
            0,
            "ZERO frames no transporte — o byte nao pode sair da maquina"
        );
    }

    /// A trava vale tambem no `--dry-run`: o dry-run existe para o operador
    /// inspecionar o frame, e um frame que o firmware rejeitaria nao e
    /// inspecionavel — e nada e impresso como se fosse mandavel.
    #[test]
    fn dry_run_tambem_recusa_valor_fora_da_faixa() {
        let (t, (writes, _ultimos)) = Contador::new();
        let mut s = Session::new(t);
        let dry = Args {
            command: Command::Info,
            dry_run: true,
            log: None,
            real: false,
            mock_device: false,
        };
        assert_eq!(
            run_set_param(&mut s, 3, 0x0700_006e, 0, 99.5, &dry),
            EXIT_PROTOCOL_ERROR
        );
        assert_eq!(writes.get(), 0, "nada sai em dry-run — nem frame");
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
            mock_device: false,
        };
        let (t, (writes, _ultimos)) = Contador::new();
        let mut s = Session::new(t);
        assert_eq!(run_set_param(&mut s, 3, 0x0700_006e, 0, 15.0, &dry), 0);
        assert_eq!(
            writes.get(),
            0,
            "dry-run nao pode mandar nada — e o que segura o F1 de campo"
        );
        assert_eq!(run_save(&mut s, 0, 4, "H2 TESTE", &dry), 0);
        assert_eq!(writes.get(), 0, "save em dry-run nao grava nada");
    }

    /// `read-state5` é LEITURA pura: parseia sozinho (sem travas de escrita) —
    /// o gate de hardware (camada 2, `--real`+feature) é do TRANSPORTE.
    #[test]
    fn read_state5_parseia_como_leitura() {
        let parsed = parse_args(args(&["read-state5"]).into_iter()).expect("parse ok");
        assert!(matches!(parsed.command, Command::ReadState5));
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

    /// `--mock-device` é o SELO do mock de teste (#165): passa no parser e
    /// deixa `real` desligado — o harness declara a intenção sem depender do
    /// default do crate.
    #[test]
    fn mock_device_sela_o_transporte() {
        let parsed = parse_args(args(&["info", "--mock-device"]).into_iter()).expect("parse ok");
        assert!(parsed.mock_device);
        assert!(!parsed.real);
    }

    /// `--real` e `--mock-device` juntos são uso inválido: um abre o aparelho,
    /// o outro sela o mock — aceitar os dois faria o script mentir sobre o
    /// transporte. A recusa é do parser (exit 2), nunca do runtime.
    #[test]
    fn real_com_mock_device_e_recusado() {
        let err = parse_args(
            args(&["--real", "--i-know-what-im-doing", "--mock-device", "info"]).into_iter(),
        )
        .unwrap_err();
        assert!(err.0.contains("exclusivos"), "mensagem: {}", err.0);
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
            "--mock-device",
            "info",
            "list-user-irs",
            "read-state5",
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
        let mut log = gp100_core::wire_log::WireLogger::with_session(&path, "H3").expect("arquivo");
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
        let mut log = gp100_core::wire_log::WireLogger::with_session(&path, "H3").expect("arquivo");
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
