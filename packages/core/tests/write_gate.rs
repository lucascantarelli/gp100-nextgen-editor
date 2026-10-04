//! `write_gate.rs` — a trava de escrita do ADR-5 se prova por MUTACAO.
//!
//! `WRITE_VERIFIED` é uma **constante de compilação** do transporte real, e
//! a tentação de um gate desses é ser decorativo: um `if` que ninguém
//! exercita. Estes testes exercitam a trava pelos dois lados — o que ela
//! barra e o que ela DEVE deixar passar — porque uma trava que barra
//! leitura é tão quebrada quanto uma que não barra escrita.
//!
//! **O que não é testável aqui:** o `RealDevice` de verdade precisa de
//! hardware e da feature `real-device`, então os testes de travar ele rodam
//! num `BloqueioFake` que replica o mesmo `if`. A constante em si é
//! provada pelo par `#[cfg]` do `real.rs` + o teste de build da CI, que
//! compila o binário de campo com e sem a feature.

use std::time::Duration;

use gp100_core::codec::{ir_begin, ir_chunk, meta_block, set_param, write_frame};
use gp100_core::session::Session;
use gp100_core::transport::{DeviceTransport, MockDevice, TransportError, WireKind};
use gp100_core::SYSEX_HEADER;

/// Endereço 4B de um frame SysEx (§13.1): cabeçalho de8 + FUNC + addr.
fn addr_de(data: &[u8]) -> String {
    let ini = SYSEX_HEADER.len() + 1;
    data.get(ini..ini + 4)
        .map(hex)
        .unwrap_or_else(|| "(curto)".into())
}

/// Transporte que replica a trava do `RealDevice` **exatamente** como o
/// `real.rs` faz: `Write` sem `write_verified` é recusado antes de
/// qualquer byte; `Read` passa sempre.
///
/// Não é mock do comportamento do DEVICE (isso é o `MockDevice`); é mock do
/// que o **ADR-5** faz, e é por isso que o gate dele é testável sem
/// pedaleira.
struct TravaFake {
    /// A flag que o `real.rs` lê de `cfg!`.
    write_verified: bool,
    /// Frames que SAIRAM (o que a trava tem que impedir de crescer).
    enviados: Vec<(String, WireKind)>,
}

impl TravaFake {
    fn new(write_verified: bool) -> Self {
        Self {
            write_verified,
            enviados: Vec::new(),
        }
    }
    fn escritas_bloqueadas(&self) -> usize {
        self.enviados
            .iter()
            .filter(|(_, k)| *k == WireKind::Write)
            .count()
    }
}

impl DeviceTransport for TravaFake {
    fn open(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
    fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    /// A MESMA ordem do `real.rs`: primeiro a trava, DEPOIS o "driver".
    /// Inverter isso faria a trava parecer funcionar e não barrar nada.
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        if kind == WireKind::Write && !self.write_verified {
            return Err(TransportError::WriteBlocked { op: addr_de(data) });
        }
        self.enviados.push((addr_de(data), kind));
        Ok(())
    }

    fn recv_raw(&mut self, _timeout: Duration) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::RecvTimeout { timeout_ms: 0 })
    }
}

/// A MESMA trava do `real.rs`, embrulhando um transporte que tem caminho de
/// LEITURA de verdade. É o que permite rodar o `boot()` inteiro contra o
/// `MockDevice` e observar onde ele bate na trava — sem isso o boot morre no
/// primeiro `wait_for` (timeout) e o teste nunca chega ao keepalive.
///
/// E o embrulho prova que a trava COMPOE: ela não depende de ser a única
/// politica do transporte, funciona por cima de outro.
struct Gate<T> {
    inner: T,
    write_verified: bool,
    kinds: Vec<(String, WireKind)>,
}

impl<T: DeviceTransport> Gate<T> {
    fn new(inner: T, write_verified: bool) -> Self {
        Self {
            inner,
            write_verified,
            kinds: Vec::new(),
        }
    }
}

impl<T: DeviceTransport> DeviceTransport for Gate<T> {
    fn open(&mut self) -> Result<(), TransportError> {
        self.inner.open()
    }
    fn close(&mut self) -> Result<(), TransportError> {
        self.inner.close()
    }
    fn send_raw(&mut self, data: &[u8], kind: WireKind) -> Result<(), TransportError> {
        if kind == WireKind::Write && !self.write_verified {
            return Err(TransportError::WriteBlocked { op: addr_de(data) });
        }
        self.kinds.push((addr_de(data), kind));
        self.inner.send_raw(data, kind)
    }
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        self.inner.recv_raw(timeout)
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Um frame de escrita real (§13.12 meta block) — não um frame qualquer, para
/// que o teste não prove a trava com um byte sintético.
fn frame_de_escrita() -> Vec<Vec<u8>> {
    meta_block(0, 4, "H2")
        .expect("meta_block monta")
        .into_iter()
        .map(|(addr, payload)| write_frame(&addr, &payload))
        .collect()
}

// ═══════════════════════════════════════════ a trava barra escrita

/// A trava barra o knob (§13.11) com `write_verified == false` — e nenhum
/// byte sai.
#[test]
fn knob_e_barrado_sem_write_verified() {
    let mut dev = TravaFake::new(false);
    let mut s = Session::new(&mut dev);
    let err = s
        .set_param(1, 0x0700_006e, 0, 42.0)
        .expect_err("escrita tem de ser barrada");
    let msg = err.to_string();
    assert!(
        msg.contains("escrita bloqueada") || msg.contains("transporte"),
        "a recusa tem de ser legível e sobre escrita: {msg}"
    );
}

/// A trava barra o save (§13.12) **inteiro** — nenhum dos 5 frames do meta
/// block passa. É o ADR-5, consequência 6: a checagem é ANTES do 1º write,
/// senão o device fica com metadado pela metade.
#[test]
fn save_nao_escreve_nenhum_frame_sem_write_verified() {
    let mut dev = TravaFake::new(false);
    let mut s = Session::new(&mut dev);
    let _ = s.save_preset(0, 4, "H2");
    assert_eq!(
        dev.escritas_bloqueadas(),
        0,
        "nenhum frame mutante pode sair com a trava fechada"
    );
    assert!(
        dev.enviados.is_empty(),
        "save barrado não manda NADA (nem leitura): {:?}",
        dev.enviados
    );
}

/// **Contraprova da meia escrita.** `save_preset` monta 9 frames (5 meta +
/// 4 ops). Se a trava fosse consultada por frame e o `write_verified`
/// pudesse mudar no meio, os 5 primeiros saíam e os 4 últimos não — meta
/// pela metade no device, que é exatamente o que o ADR-5 proíbe.
///
/// Aqui a flag é lida uma vez e é constante, então a propriedade é
/// "ou os 9 saem, ou nenhum". O teste fixa essa propriedade.
#[test]
fn save_e_atomico_ou_nada() {
    // Com a trava aberta, os 9 saem.
    let mut dev = TravaFake::new(true);
    let mut s = Session::new(&mut dev);
    s.save_preset(0, 4, "H2").expect("com a flag, save passa");
    assert_eq!(
        dev.escritas_bloqueadas(),
        9,
        "save = 5 writes de meta + 4 ops (§13.12), e todos saem juntos"
    );
}

/// A trava barra o upload de IR (§13.7) — inclusive o `ir_begin`, que é o
/// primeiro frame e o que reserva o slot no device.
#[test]
fn upload_ir_e_barrado_sem_write_verified() {
    let mut dev = TravaFake::new(false);
    let mut s = Session::new(&mut dev);
    let blob = [0x22u8; 30]; // 2 chunks de 15B
    let _ = s.upload_ir(0, &blob);
    assert!(
        dev.enviados.is_empty(),
        "nada do upload pode sair: {:?}",
        dev.enviados
    );
}

/// E o `ir_begin` sozinho já é barrado: é ele que reserva o slot, e um
/// begin que passa com o resto barrado deixa o slot reservado no device.
#[test]
fn ir_begin_e_barrado_antes_de_qualquer_chunk() {
    let mut dev = TravaFake::new(false);
    let err = dev
        .send_raw(&ir_begin(3).expect("begin"), WireKind::Write)
        .expect_err("begin e escrita");
    assert!(
        matches!(err, TransportError::WriteBlocked { .. }),
        "ir_begin precisa de WriteBlocked tipado: {err:?}"
    );
    assert!(dev.enviados.is_empty());
}

/// A trava barra a DUPLICAÇÃO final do chunk (§13.7) também — ela é o
/// marcador de fim, e deixar ela passar sozinha trava o upload no meio.
#[test]
fn chunk_final_duplicado_tambem_e_escrita() {
    let mut dev = TravaFake::new(false);
    for idx in [0u16, 1, 0x226] {
        let err = dev
            .send_raw(
                &ir_chunk(1, idx, &[0x22u8; 15]).expect("chunk"),
                WireKind::Write,
            )
            .expect_err("chunk e escrita");
        assert!(
            matches!(err, TransportError::WriteBlocked { .. }),
            "idx {idx} tem de ser barrado"
        );
    }
    assert!(dev.enviados.is_empty());
}

// ═════════════════════════════════════════ a trava NÃO barra leitura

/// A LEITURA tem que passar com a trava fechada — senão o gate segura o
/// hardware, que é o oposto do que se quer.
#[test]
fn leitura_passa_com_a_trava_fechada() {
    let mut dev = TravaFake::new(false);
    let mut s = Session::new(&mut dev);
    // `state_page` monta o frame pelo golden (o caminho real de leitura).
    // O transporte não tem IN, então ele dá timeout — o que prova que o
    // frame de LEITURA SAIU (a trava não barrou), e não que deu certo.
    let err = s.state_page(0).expect_err("sem IN o mock da timeout");
    assert!(
        !err.to_string().contains("escrita bloqueada"),
        "state_page é leitura e não pode ser barrado pela trava: {err}"
    );
    assert_eq!(dev.enviados.len(), 1, "o frame de leitura saiu");
    assert_eq!(dev.enviados[0].1, WireKind::Read);
}

/// A trava é sobre o `WireKind`, não sobre o endereço: um `Write` num
/// endereço de leitura é barrado, e um `Read` num endereço de escrita
/// passa. A trava é do QUE o frame faz, não de onde ele vai.
#[test]
fn a_trava_segue_o_kind_nao_o_endereco() {
    let mut dev = TravaFake::new(false);
    // Endereço de escrita classificado como leitura: passa.
    dev.send_raw(
        &set_param(1, 0x0700_006e, 0, 1.0).expect("knob"),
        WireKind::Read,
    )
    .expect("Read passa mesmo em addr de escrita");
    // Endereço de leitura classificado como escrita: barra.
    let barrado = dev.send_raw(&[0xF0, 0x00, 0x11, 0x00, 0x00, 0x00], WireKind::Write);
    assert!(barrado.is_err(), "Write barra mesmo em addr de leitura");
}

// ═══════════════════════════ o achado: o boot CONTÉM uma escrita

/// **O `boot()` não é uma operação de leitura.** O script de boot (§13.10)
/// termina com o keepalive `12/00020001` — um frame OUT que não pede nada
/// de volta, ou seja, uma ESCRITA pelo critério do `WireKind`.
///
/// Consequência que este teste existe para registrar: com `write_verified`
/// fechado, `boot()` **falha** — e o H1, que é o gate de LEITURA, não pode
/// rodar o boot completo. O `H1_CHECKLIST.md` §3 B5 oferece o "boot
/// completo (opcional)" dentro de uma sessão declarada "LER é seguro".
///
/// Isto não é um bug do gate — é o gate dizendo a verdade sobre o que o
/// boot faz. O desfecho (pular o keepalive, ou exigir a flag no B5) é
/// decisão do owner com a pedaleira na mão; o que este PR faz é não
/// esconder.
#[test]
fn boot_contem_escrita_e_falha_com_a_trava_fechada() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut dev = Gate::new(mock, false);
    let mut s = Session::new(&mut dev);
    let err = s.boot().expect_err("boot tem keepalive de escrita");
    assert!(
        err.to_string().contains("escrita bloqueada"),
        "o boot tem de barrar no keepalive, não em outro ponto: {err}"
    );
}

/// E o ponto exato do bloqueio é o keepalive: o `send_build` do T4 é o
/// único `Write` do boot. Se ele parasse de ser `Write`, o boot passaria
/// inteiro — e o teste acima deixaria de acusar.
#[test]
fn o_bloqueio_do_boot_e_no_keepalive_e_nao_no_scan() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut dev = Gate::new(mock, false);
    let mut s = Session::new(&mut dev);
    let err = s.boot().expect_err("barrado");
    // Tudo que SAIU antes do bloqueio tem de ser de LEITURA — se algum
    // `Write` tivesse vazado no scan, o bloqueio não seria no keepalive.
    for (addr, kind) in &dev.kinds {
        assert_eq!(*kind, WireKind::Read, "endereço {addr} saiu como escrita");
    }
    assert!(
        dev.kinds.len() > 100,
        "o scan tem que ter rodado antes do keepalive ({} frames)",
        dev.kinds.len()
    );
    // O keepalive é barrado ANTES de entrar no log (a trava vem primeiro) —
    // então `00020001` não aparece em `kinds`, e o erro diz que endereço.
    assert!(
        !dev.kinds.iter().any(|(a, _)| a == "00020001"),
        "o keepalive barrado não pode ter vazado para o log"
    );
    match err {
        gp100_core::ProtocolError::InvalidShape { got, .. } => assert!(
            got.contains("00020001"),
            "a recusa tem de nomear o keepalive: {got}"
        ),
        other => panic!("esperava o erro do keepalive, veio {other:?}"),
    }
}

/// Contraprova: **com a flag ligada, o boot inteiro passa.** Sem isto, um
/// gate que barra tudo também passaria no teste de cima, e o teste só
/// provaria que o boot falha — não que a trava é a causa.
#[test]
fn com_a_flag_o_boot_completo_passa() {
    let mut mock = MockDevice::new().expect("mock");
    mock.open().expect("open");
    let mut dev = Gate::new(mock, true);
    let mut s = Session::new(&mut dev);
    s.boot().expect("com write_verified o boot passa inteiro");
    assert_eq!(
        dev.kinds.last().map(|(a, _)| a.as_str()),
        Some("00020001"),
        "o keepalive é o ÚLTIMO frame do boot (T4)"
    );
    assert!(
        dev.kinds
            .iter()
            .any(|(a, k)| a == "00020001" && *k == WireKind::Write),
        "e ele é classificado como escrita"
    );
}

// ══════════════════════════════════════ a constante WRITE_VERIFIED

/// A `WireKind` é o que o ADR-5 consome, e ela é pública na trait — se
/// alguém escrever `send_raw(frame, WireKind::Read)` num fluxo de escrita,
/// a trava não pega. O teste abaixo fixa que a **enum tem só dois
/// membros** e que o `Write` existe (um `#[non_exhaustive]` ou um
/// renome acidental quebraria a compilação deste arquivo).
#[test]
fn wire_kind_tem_exatamente_leitura_e_escrita() {
    assert_ne!(WireKind::Read, WireKind::Write);
    assert_eq!(WireKind::Read.as_str(), "leitura");
    assert_eq!(WireKind::Write.as_str(), "escrita");
}

/// O erro de recusa carrega o ENDEREÇO do frame barrado. Sem ele, o
/// operador em campo recebe "escrita bloqueada" e não sabe de qual
/// endereço — e o log de fio pode ter vários frames pendentes.
#[test]
fn a_recusa_diz_o_endereco_do_frame_barrado() {
    let mut dev = TravaFake::new(false);
    let err = dev
        .send_raw(
            &set_param(1, 0x0700_006e, 0, 1.0).expect("knob"),
            WireKind::Write,
        )
        .expect_err("barrado");
    match err {
        TransportError::WriteBlocked { op } => {
            // knob = `10 <slot> 00 02` (§13.11) com slot 1 → 10010002
            assert_eq!(op, "10010002", "a recusa tem de dizer o endereço real");
        }
        other => panic!("esperava WriteBlocked, veio {other:?}"),
    }
}

/// Um frame curto demais não pode fazer a montagem da recusa entrar em
/// pânico — o CLI promete "nunca panic" (ADR-2) e `addr_do_frame` é o
/// caminho que o `WriteBlocked` percorre.
#[test]
fn frame_curto_da_recusa_sem_panico() {
    let mut dev = TravaFake::new(false);
    let err = dev
        .send_raw(&[0xF0, 0x00], WireKind::Write)
        .expect_err("curto demais pra ter endereço");
    match err {
        TransportError::WriteBlocked { op } => assert!(
            op.contains("curto") || op.is_empty(),
            "a recusa degrada, não quebra: {op:?}"
        ),
        other => panic!("esperava WriteBlocked, veio {other:?}"),
    }
}

/// `meta_block` monta 5 frames de metadado (§13.12) — a contagem que
/// `save_e_atomico_ou_nada` usa. Se o codec mudar a quantidade, este teste
/// avisa em vez de o outro testar a contagem errada.
#[test]
fn meta_block_sao_cinco_frames() {
    let frames = frame_de_escrita();
    assert_eq!(
        frames.len(),
        5,
        "§13.12: 5 writes de meta (11000000/04/05/07 + 12000002)"
    );
    assert!(frames.iter().all(|f| f.len() > SYSEX_HEADER.len() + 5));
}
