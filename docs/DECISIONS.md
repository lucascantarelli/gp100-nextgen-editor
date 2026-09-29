# 📜 DECISIONS — Decisões de design pré-assinadas (ADR-lite)

> **Banner:** ✅ atual · **Última revisão:** 2026-09-28 · **Origem:** ROADMAP P5
>
> Registra as 5 escolhas estruturais do `gp100-core` **antes** do primeiro commit
> de lógica de negócio (M0), para que nenhuma issue vire debate no meio do caminho.
> Evidências citam o golden (`docs/protocol_golden.json`, Baseline v1.0 — sha256
> `0426d6a8…ef859`, ver §13 do PROTOCOL.md) e as provas de `validate_golden.py`.
>
> **Regra de mudança:** decisão alterada = **nova entrada ADR** marcando a antiga
> como *Superseded* (nunca apagar). Mudança de PROTOCOLO (bytes) não passa por
> aqui — passa pelo fluxo R2/R3 do ROADMAP (captura → golden → validate → bump).

---

## ADR-1 — Endianness e nibble-expansão no fio

**Status:** Accepted · **Pré-assinada:** 28/09 (owner) · **Afeta:** M0.3, M0.4, M0.6

**Decisão.** O fio do GP-100 usa endian MISTO e payloads nibble-expandidos. Regras
canônicas (o codec consulta o golden; nunca assume):

| Campo | Convenção | Evidência (golden/prova) |
|---|---|---|
| `pp` (preset id), `PG` (página) | **u16 BE** | boot 2299/2299: select `11|13010000 ← 01 00` (pp 0x0100); página `12|13010004 ← [pp BE][PG BE] 01` (5B) (§13.10) |
| `ppType` / ícones | **u16 BE** | save 77/77: `12|11000005 ← 00 04 00 00` ("It's GP100", tipo 4) (§13.12) |
| CRC (tabela de IRs) | **CRC32 BE** | tabela `12|12001002` 75B nibble-exp: CRC32 BE por slot; vazio = ppIRCRC do `.prst` (§13.12) |
| `effectCode` | **u32 LE** | knobs 89/89 + 3/3: `12|10xx0002` (§13.11) |
| float de valor físico | **f32 LE** | idem: `[effectCode u32 LE][ctrl u8][00][f32 LE]` |
| payloads de objeto | **nibble-expandidos, hi primeiro** | todo write de objeto 10xx/11xx/1200/IR |

**Exemplo trabalhado** (fixture `knobs.jsonl` linha 1, provado byte-a-byte pela prova B):
payload de 20B `06 0e 00 00 00 00 00 07 … 04 01` → colapso → 10B reais
`[6e 00 00 07][00][00][00 00 70 41]` = effectCode `0x0700006e` (u32 **LE**:
nibble 0x07, index 0x6e), ctrl 0, valor **15.0** (f32 **LE**).

**Consequências.**
- Codec (M0.4) expõe helpers separados: `be_u16/be_u32` para campos de
  endereçamento; `le_u32/le_f32` para semântica; `nibble_expand/collapse` (hi
  primeiro) para payloads de objeto.
- Proibido inferir largura/endian de campo novo sem golden (R1).

---

## ADR-2 — Erros tipados com `thiserror` (`ProtocolError`)

**Status:** Accepted · **Pré-assinada:** 28/09 (owner) · **Afeta:** M0.3–M0.6

**Decisão.** Toda falha de protocolo é um erro tipado único, `ProtocolError`
(definido com `thiserror`, já no workspace), com variantes mínimas:

- `InvalidShape { expected, got }` — mensagem de comprimento/const errados.
  Cobre inclusive truncamento do ring buffer do proxy (SEM-HDR), que é **dado
  conhecido**, não bug do codec (fixtures já excluem; o codec sinaliza).
- `Timeout` — resposta não chegou dentro da janela da transação (ADR-3).
- `UnexpectedAck` — resposta/ACK com conteúdo fora do esperado pela FSM (M0.6).

**Consequências.**
- Nada de `unwrap`/`panic!` na lib: camadas superiores (UI/Tauri) recebem
  `Result<_, ProtocolError>` sempre.
- Variantes novas entram aqui por ADR; erro de I/O do transporte fica separado
  (`TransportError` em M0.5), não polui o erro de protocolo.

---

## ADR-3 — Transporte síncrono, timeout por transação = 3s

**Status:** Accepted · **Pré-assinada:** 28/09 (owner) · **Afeta:** M0.5, M0.6

**Decisão.** O transporte do M0 é **síncrono e bloqueante**, com timeout
**de 3000 ms por transação** — o mesmo valor comprovado no pairing do golden
(`PAIR_TIMEOUT_MS = 3000` em `build_golden.py`, que pareia 100% das transações
das 4 sessões).

**Consequências.**
- Mock e replay determinísticos: o mock responde dentro da janela; testes do
  M0.6 não têm flakiness de tempo.
- Sem tokio/async no M0. A UI (Tauri) chama o core síncrono via `spawn_blocking`;
  revisitar async só se o live mode (M3) exigir — novo ADR na ocasião.
- Timeout é por transação (request→resposta), não global de sessão.

---

## ADR-4 — Trait `DeviceTransport`

**Status:** Accepted · **Pré-assinada:** 28/09 (owner) · **Afeta:** M0.4, M0.5, M0.6

**Decisão.** A fronteira com o device é a trait:

```rust
pub trait DeviceTransport {
    fn open(&mut self) -> Result<(), TransportError>;
    fn close(&mut self) -> Result<(), TransportError>;
    fn send_raw(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError>;
}
```

**Consequências.**
- A trait trafega **bytes crus**: framing do SysEx (header/F7/trim), nibble e
  semântica moram no codec (M0.4), nunca no transporte.
- Implementadores previstos: `MockDevice` (M0.5, default — responde conforme o
  golden) e `RealDevice` (midir/WinMM/ALSA) **atrás da feature `real-device`**,
  desabilitada por default (política de hardware, VISION §7).
- `Session` (M0.6) é genérica sobre `DeviceTransport` — replay das fixtures não
  conhece hardware.

---

## ADR-5 — Feature-flag `WRITE_VERIFIED` (mock sempre permite)

**Status:** Accepted · **Pré-assinada:** 28/09 (owner) · **Afeta:** M0.5, Gate H (H2)

**Decisão.** Escrita no device **real** é guardada por flag `WRITE_VERIFIED`
(constante/config do transporte real). Enquanto `WRITE_VERIFIED == false`, o
transporte real **recusa writes com erro explícito**. O `MockDevice` **sempre
permite** writes — caso contrário os replays das fixtures (knob/save/IR) não
existiriam.

**Estado atual:** os 3 fluxos capturados (set §13.11, save §13.12, IR §13.7) já
foram verificados em campo na S4 (persistência confirmada no display —
BLOCKERS item 11 fechado), mas a flag **começa false** e só vira `true` com o
DoD do H2 executado **pelo gp100-core no device** (3 fluxos, um por vez, com
read-back/verificação no display).

**Consequências.**
- Divergência de fio descoberta no H1/H2 = fluxo R3 (captura → golden → validate),
  nunca "ajuste" ad-hoc no codec.
- Fluxo de escrita novo (fora os 3 capturados) = proibido até captura própria +
  baseline nova + esta flag reavaliada por ADR.

---

## Aplicação

| Issue | ADRs que vincula |
|---|---|
| M0.3 (golden-file consumer) | 1, 2 |
| M0.4 (codec) | 1, 2, 4 |
| M0.5 (transporte + mock) | 2, 3, 4, 5 |
| M0.6 (FSM + replay) | 2, 3, 4 |
| H1–H3 (gate de hardware) | 3, 5 |
