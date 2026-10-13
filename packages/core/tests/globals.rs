//! Registradores globais (§13.13 — sessão G3–G6/G7): o caminho de produção
//! transporte → FSM (`Session::read_register`/`write_register`) exercitado
//! contra o `MockDevice` (o mesmo caminho do aparelho — D8).
//!
//! O que travam:
//!   - o bloco medido lê INTEIRO na ordem da tabela `REGS_GLOBAIS`;
//!   - o decode medido (nibble-par: valor = d0*16 + d1, §13.13.4) devolve
//!     os defaults espelhados da UI (master 99 · drum 120/80);
//!   - write → read roundtrip (o mock guarda o valor como o aparelho);
//!   - resposta com comprimento FORA do medido é InvalidShape (o mock só
//!     produz formas medidas; a trava de forma é da Session).

use gp100_core::codec::{nibble_pair_payload, nibble_pair_value};
use gp100_core::session::{Session, REGS_GLOBAIS};
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport;

/// gVolume (§13.13.7: "volume master").
const G_VOLUME: [u8; 4] = [0x00, 0x02, 0x00, 0x03];
/// drumBpm (medido no diff controlado §13.13.4).
const DRUM_BPM: [u8; 4] = [0x00, 0x01, 0x02, 0x03];
/// drumVolume (RE do Suite §13.13.7).
const DRUM_VOLUME: [u8; 4] = [0x00, 0x01, 0x02, 0x04];
/// enable do Global EQ (8B medido).
const EQ_ENABLE: [u8; 4] = [0x00, 0x02, 0x00, 0x11];

#[test]
fn read_globals_devolve_o_bloco_inteiro_com_os_defaults() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("mock aberto");
    let mut session = Session::new(&mut mock);

    let regs = session.read_globals().expect("bloco lido");
    assert_eq!(
        regs.len(),
        REGS_GLOBAIS.len(),
        "a tabela inteira responde — tudo-ou-nada"
    );

    let get = |addr: [u8; 4]| {
        regs.iter()
            .find(|(a, _)| *a == addr)
            .unwrap_or_else(|| panic!("registrador {addr:02x?} presente"))
            .1
            .clone()
    };

    // defaults espelhados da UI (master 99, drum 120/80) via decode medido
    assert_eq!(nibble_pair_value(&get(G_VOLUME)), Some(99));
    assert_eq!(nibble_pair_value(&get(DRUM_BPM)), Some(120));
    assert_eq!(nibble_pair_value(&get(DRUM_VOLUME)), Some(80));
    // enable do Global EQ é o registro de 8B da família medida
    assert_eq!(get(EQ_ENABLE).len(), 8);
}

#[test]
fn write_register_faz_roundtrip_no_mock() {
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("mock aberto");
    let mut session = Session::new(&mut mock);

    let payload = nibble_pair_payload(42).expect("encode");
    session.write_register(G_VOLUME, &payload).expect("write");
    let lido = session.read_register(G_VOLUME, &[4, 8]).expect("read");
    assert_eq!(
        nibble_pair_value(&lido),
        Some(42),
        "o mock guarda o valor escrito como o aparelho (§13.13.1)"
    );
}

#[test]
fn read_register_rejeita_payload_fora_do_medido() {
    // trava de forma: a Session valida COMPRIMENTO medido (§13.13.1) —
    // 4B ou 8B para o SYSTEM; um 6B nunca é forma válida.
    let mut mock = MockDevice::new().expect("mock montado");
    mock.open().expect("mock aberto");
    let mut session = Session::new(&mut mock);
    let err = session.read_register(G_VOLUME, &[6]).expect_err("6B não é forma medida");
    assert!(
        err.to_string().contains("medido"),
        "erro cita a origem da forma: {err}"
    );
}
