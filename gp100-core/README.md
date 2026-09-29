# gp100-core — núcleo do GP-100 NextGen Editor

> **Status:** ✅ Fase M0 concluída (M0.1–M0.8, 29/09) · CI verde · Baseline do protocolo: v1.0 (`docs/protocol_golden.json`, sha256 no §13)
> **Fontes de verdade:** [docs/protocol_golden.json](../docs/protocol_golden.json) (spec executável) · [docs/PROTOCOL.md](../docs/PROTOCOL.md) §13 (narrativa de campo) · [docs/DECISIONS.md](../docs/DECISIONS.md) (ADR-1..6) · [analysis/fixtures/](../analysis/fixtures/) (replay P4)
> **Mapa geral do projeto:** [docs/INDEX.md](../docs/INDEX.md) · **Plano:** [docs/ROADMAP.md](../docs/ROADMAP.md) · **UI:** [docs/UI_PLAN.md](../docs/UI_PLAN.md)

Biblioteca Rust de protocolo e presets da pedaleira Valeton GP-100, construída
por engenharia reversa **sem adivinhar um byte** (regra R1 do ROADMAP): todo
formato vem do golden-file (40 templates request→resposta provados byte-a-byte
contra 4 sessões de captura) e todo fluxo de escrita é um dos 3 capturados em
campo (knob §13.11, save §13.12, upload de IR §13.7).

## Workspace

```
gp100-core/   lib — todo o protocolo e presets (este README)
gp100-cli/    bin — operação/demonstração contra o MockDevice (M0.7)
```

O `gp100-cli` consome SOMENTE a API pública desta lib (regra ADR-6: "o CLI
chama só os métodos da FSM"); a UI (M1, Tauri) fará o mesmo — nenhum binário
toca golden/codec diretamente.

## Arquitetura (fases M0)

```
model (M0.1)      dicionário serde: 185 algs / 639 controles, validação na carga
    ↓
preset (M0.2)     .prst = XML; parser/writer com LAYOUT COMO DADO —
    ↓             round-trip byte-idêntico dos 3 arquivos (R4, sagrado)
golden (M0.3)     GoldenFile::embedded() (OnceLock, 1 parse por processo):
    ↓             Template::build_request / matches_response (despacho by-len)
codec (M0.4)      fio PURO e sem estado: envelope §13.1, trim no 1º F7,
    ↓             nibble strict, set_param / meta_block / ir_begin / ir_chunk
transport (M0.5)  trait DeviceTransport (ADR-4, bytes crus, sync, ciclo de
    ↓             vida do CHAMADOR) + MockDevice (responde conforme o golden,
                  D1–D8) · RealDevice (H1) atrás da feature `real-device`:
                  midir/WinMM, callback→fila, trim no 1º F7 na entrada
session (M0.6)    FSM Session<T: DeviceTransport> (ADR-6 rev.3): boot/scan,
                  select/state_page, set_param, save_preset (fire-and-forget D3),
                  upload_ir (ACK por chunk), list_user_irs, backlog D7
gp100-cli (M0.7)  subcomandos info / list-user-irs / dump-preset / set-param
                  --dry-run / save --dry-run + --log (schema das fixtures P4)
```

Regras de dispatch da FSM (FONTE ÚNICA = ADR-6): **D1** a transação é dona do
endpoint · **D2** push classificado pelo contexto · **D3** save fire-and-forget ·
**D4** sem read-back · **D5** sem match = `InvalidShape` tipado · **D6** sem
retry (timeout 3s por transação, ADR-3) · **D7** backlog de IN não solicitado ·
**D8** consumidor único do stream IN.

## Como construir e testar

```bash
# terminal novo sem cargo no PATH: scripts/add_cargo_path.ps1 (1x) resolve no sistema
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test            # inclui os DOC-TESTS (exemplo de doc que mente quebra o build)
```

O gate Python (mesma CI) valida a especificação contra as capturas:

```bash
uv run pytest         # 10/10: 5 provas do validate_golden + hash da baseline + paridade de fixtures
```

CI (`.github/workflows/ci.yml`): os mesmos gates por push/PR, em runner
`windows-latest` (toolchain `stable-x86_64-pc-windows-gnu` pinada em
`rust-toolchain.toml` — não trocar o alvo; ver `.agents/skills/rust-practices`).

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

## O CLI (gp100-cli)

```bash
cargo run -p gp100-cli -- info                        # estado do mock
cargo run -p gp100-cli -- list-user-irs               # tabela dos 20 slots
cargo run -p gp100-cli -- dump-preset 0x0007          # select + 9 páginas (hex)
cargo run -p gp100-cli -- set-param 3 0x0700006e 0 15.0 --dry-run
cargo run -p gp100-cli -- save 0x0007 6 "Blink OD" --dry-run
cargo run -p gp100-cli -- list-user-irs --log session.jsonl  # frames no schema P4
```

Política de hardware: `--real` é recusado SEMPRE nesta fase (exit 2) — o gate H
(H1 leitura / H2 escrita com `WRITE_VERIFIED`) ainda não rodou. `--log` grava
OUT/IN no MESMO schema das fixtures P4 (`{"s","dir","func","addr","data"}`),
consumível por `analysis/decode_wire.py` e pelos testes de replay — é o log que
o [H1_CHECKLIST](../docs/H1_CHECKLIST.md) usa em campo.

## Testes (modelo híbrido)

- **Unitários** `#[cfg(test)]` dentro de `src/` — só internals (ex.: primitivas
  nibble do codec, carga do estado do mock).
- **Contratos** caixa-preta em `tests/` — contra EVIDÊNCIA de campo, lida do
  repo (regime de bytes `-text` no `.gitattributes`):

| Arquivo | Prova |
|---|---|
| `wire_envelope.rs` | envelope §13.1 (F0 21 25 7F 47 50 2D 64 \| FUNC \| ADDR \| DATA \| F7) |
| `golden_consumer.rs` | propriedade extract→build == exemplo nos 40 templates |
| `codec_wire.rs` | 92 knobs + 2 saves + 1186 frames IR byte a byte (fixtures P4) |
| `roundtrip_prst.rs` | round-trip byte-idêntico dos 3 `.prst` (R4) |
| `model_dictionary.rs` | carga/validação/rejeição do dicionário |
| `transport_trait.rs` | trait pub/object-safe, DeviceGone, reconexão |
| `golden_response.rs` | simetria `build_response` nos 15 exemplos IN |
| `mock_device.rs` | diálogo completo mock↔codec (D1–D8) |
| `replay_fixtures.rs` | **replay byte-a-byte das 4 fixtures** (boot 2299/2299 OUTs) |
| `session_skeleton.rs` | contratos de ciclo de vida da Session |

Regra de decisão (skill `rust-practices`): usa só API pública → `tests/`;
precisa de internals → `src/`. Helpers compartilhados em `tests/common/mod.rs`.

## Erros e convenções

- Toda falha de protocolo é `ProtocolError` tipado (ADR-2, thiserror):
  `InvalidShape` / `Timeout` / `UnexpectedAck` — sem `unwrap`/`panic!` na lib.
- Endianness (ADR-1): pp/PG/CRC **BE** no fio; effectCode e float32 **LE**;
  payloads de objeto **nibble-expandidos** (hi primeiro).
- Magic number de protocolo NUNCA inline: ou é const documentada ou entra no
  golden (R1). Doc-comments em PT-BR com evidência; `#![deny(missing_docs)]`.
- `RealDevice` só existe atrás da feature `real-device` (default OFF) — nada de
  I/O de device fora do módulo `transport` (ADR-4).

## Próximos passos

- **Gate H** (hardware): [docs/H1_CHECKLIST.md](../docs/H1_CHECKLIST.md) — o
  `--log` do CLI já produz o insumo do R3.
- **M1 (UI)**: [docs/UI_PLAN.md](../docs/UI_PLAN.md) — a UI falará com o core
  via commands Tauri (DeviceActor, D8), nunca importando `gp100-core` direto.
