# gp100-core — núcleo de protocolo (lib Rust)

> **Fontes de verdade:** [docs/protocol_golden.json](../../docs/protocol_golden.json)
> (spec executável, 40 templates) · [docs/PROTOCOL.md](../../docs/PROTOCOL.md) §13
> (narrativa do fio) · [docs/DECISIONS.md](../../docs/DECISIONS.md) (ADR-1..6) ·
> [analysis/fixtures/](../../analysis/fixtures) (replay)

Biblioteca de protocolo e presets da pedaleira Valeton GP-100, construída por
engenharia reversa **sem adivinhar um byte** (regra R1): todo formato vem do
golden-file e todo fluxo de escrita é um dos 3 capturados em campo (knob §13.11,
save §13.12, upload de IR §13.7). Consumidores ([../cli](../cli) e
[../app](../app)) tocam SOMENTE a API pública — nenhum binário fala com
codec/golden direto.

## Arquitetura

```
model       dicionário serde: 185 algs / 639 controles, validação na carga
    ↓
preset      .prst = XML; parser/writer com LAYOUT COMO DADO —
    ↓       round-trip byte-idêntico (regra R4, sagrada)
preset_json .prst em JSON VERSIONADO (#114): o layout é PUBLICADO, não
    ↓       derivado — o wrap do Suite não segue largura (medido: 82 a 91
    ↓       colunas quebram e não quebram), então adivinhar violaria o R1
    ↓
tone_sheet  folha de timbre em PDF (#114): writer próprio, fontes base-14,
    ↓       sem dependência nativa (ADR-9) e sem stream comprimido
pedalboard  o BoardView: o preset projetado pelo dicionário, na ordem do sinal
gain        o ASSISTENTE DE GAIN STAGING (#115): função PURA sobre o BoardView
    ↓       — posição na faixa do dicionário, nunca dB medido; o método e a
    ↓       limitação saem NO relatório para a tela mostrar. Sem transporte:
    ↓       não existe caminho para um byte sair daqui
golden      GoldenFile::embedded() (OnceLock, 1 parse por processo):
    ↓       Template::build_request / matches_response (despacho by-len)
codec       fio PURO e sem estado: envelope §13.1, trim no 1º F7,
    ↓       nibble strict, set_param / meta_block / ir_begin / ir_chunk
transport   trait DeviceTransport (ADR-4, bytes crus, sync, ciclo de
    ↓       vida do CHAMADOR) + MockDevice (responde conforme o golden,
            D1–D8) · RealDevice (H1) atrás da feature `real-device`:
            midir/WinMM, callback→fila, trim no 1º F7 na entrada
session     FSM Session<T: DeviceTransport> (ADR-6 rev.3): boot/scan,
            select/state_page, set_param, save_preset (fire-and-forget D3),
            upload_ir (ACK por chunk), list_user_irs, backlog D7
```

Regras de dispatch da FSM (FONTE ÚNICA = ADR-6): **D1** a transação é dona do
endpoint · **D2** push classificado pelo contexto · **D3** save fire-and-forget ·
**D4** sem read-back · **D5** sem match = `InvalidShape` tipado · **D6** sem
retry (timeout 3s por transação, ADR-3) · **D7** backlog de IN não solicitado ·
**D8** consumidor único do stream IN.

## Como construir e testar

```bash
cargo build --workspace          # workspace da raiz (packages/core + packages/cli)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test                       # inclui os DOC-TESTS
```

O gate Python (mesma CI) valida a especificação contra as capturas:

```bash
uv run pytest                    # provas do golden + hash da baseline + paridade de fixtures
```

## Exemplos (API pública)

**Dicionário** (185 algoritmos / 639 controles, validado 3 vias):

```rust
use gp100_core::model::Dictionary;

let dict = Dictionary::from_json(gp100_core::model::DICTIONARY_JSON)?;
assert_eq!(dict.len(), 185);
let bog = dict.by_name("Bog RedM"); // lookup por nome (fallback first-wins)
```

**Presets `.prst`** — parse com round-trip byte-idêntico (R4):

```rust
use gp100_core::preset::Document;

let xml = std::fs::read_to_string("files/patches/all.prst")?;
let doc = Document::parse(xml.as_bytes())?;
for p in doc.presets() {
    println!("pp={} nome={}", p.pp_id().unwrap_or("?"), p.pp_name().unwrap_or("?"));
}
let bytes = doc.to_bytes(); // byte-idêntico ao original (provado nos 3 .prst)
```

**Golden-file** — gerar request e decodificar envelope (R1: bytes DE cá):

```rust
use gp100_core::golden::{decode_envelope, GoldenFile};

let golden = GoldenFile::embedded()?;
// select de preset §13.10 (t9: 2 vars = pp u16 BE)
let tpl = golden.request_template(0x11, &[0x13, 0x01, 0x00, 0x00], 2)?;
let select = tpl.build_request(&[0x01, 0x00])?; // SysEx completo (F0…F7)
let (func, addr, payload) = decode_envelope(&select)?; // (0x11, 13010000, …)
```

**Sessão sobre o MockDevice** (mock é o default — ADR-4/ADR-5):

```rust
use gp100_core::session::Session;
use gp100_core::transport::mock::MockDevice;
use gp100_core::transport::DeviceTransport; // trait p/ mock.open()

let mut mock = MockDevice::new()?;   // estado de all.prst + dicionário
mock.open()?;                        // ciclo de vida é do CHAMADOR (ADR-4)
let mut session = Session::new(&mut mock); // blanket impl &mut T: DeviceTransport

let table = session.list_user_irs()?;        // tabela dos 20 User IRs (§13.12)
session.select_preset(0x0007)?;              // select → meta6 (§13.10)
let page = session.state_page(0)?;           // página 0 (196B, shape 13xx opaco)
session.set_param(3, 0x0700_006e, 0, 15.0)?; // knob §13.11, fire-and-forget (D4)
session.save_preset(0x0007, 6, "Blink OD")?; // save §13.12, ZERO IN esperado (D3)
drop(session); // devolve o transporte ao dono (D8: consumidor único)
```

## Contratos (tests/ — caixa-preta, contra evidência de campo)

| Arquivo | Prova |
|---|---|
| `wire_envelope.rs` | envelope §13.1 (`F0 21 25 7F 47 50 2D 64 \| FUNC \| ADDR \| DATA \| F7`) |
| `golden_consumer.rs` | propriedade extract→build == exemplo nos 40 templates |
| `codec_wire.rs` | 92 knobs + 2 saves + 1186 frames IR byte a byte (fixtures P4) |
| `roundtrip_prst.rs` | round-trip byte-idêntico dos 3 `.prst` (R4) |
| `preset_json_roundtrip.rs` | `.prst` → JSON → `.prst` byte-idêntico, JSON canônico e recusa de versão futura (#114) |
| `tone_sheet.rs` | folha em PDF: integridade da `xref`, 1 página por preset, determinismo, sem compressão (#114) |
| `gain_staging.rs` | assistente de gain (#115): origem de cada número recalculada do board, exclusões declaradas, risco/ordem de ajuste e o ByteGuard do "nenhum byte sai" |
| `model_dictionary.rs` | carga/validação/rejeição do dicionário |
| `transport_trait.rs` | trait pub/object-safe, DeviceGone, reconexão |
| `golden_response.rs` | simetria `build_response` nos 15 exemplos IN |
| `mock_device.rs` | diálogo completo mock↔codec (D1–D8) |
| `replay_fixtures.rs` | **replay byte-a-byte das 4 fixtures** (boot 2299/2299 OUTs) |
| `session_skeleton.rs` | contratos de ciclo de vida da Session |
| `boot_mock.rs` | boot END-TO-END contra o mock + progresso observacional |

Unitários `#[cfg(test)]` dentro de `src/` só para internals; helpers
compartilhados em `tests/common/mod.rs`.

## Erros e convenções

- Toda falha de protocolo é `ProtocolError` tipado (ADR-2, thiserror):
  `InvalidShape` / `Timeout` / `UnexpectedAck` — sem `unwrap`/`panic!` na lib.
- Endianness (ADR-1): pp/PG/CRC **BE** no fio; effectCode e float32 **LE**;
  payloads de objeto **nibble-expandidos** (hi primeiro).
- Magic number de protocolo NUNCA inline: ou é const documentada ou entra no
  golden (R1). Doc-comments em PT-BR com evidência; `#![deny(missing_docs)]`.
- `RealDevice` só existe atrás da feature `real-device` (default OFF) — nada de
  I/O de device fora do módulo `transport` (ADR-4).
