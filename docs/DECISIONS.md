# 📜 DECISIONS — Decisões de design pré-assinadas (ADR-lite)

> **Banner:** ✅ atual · **Última revisão:** 2026-09-29 · **Origem:** ROADMAP P5 (ADR-1..5) + pré-desenho M0.6 (ADR-6)
>
> Registra as decisões estruturais do `gp100-core` **antes** de cada bloco de
> implementação (ADR-1..5 pré-assinados antes do M0; ADR-6 sancionado antes do
> M0.6), para que nenhuma issue vire debate no meio do caminho. Evidências citam
> o golden (`docs/protocol_golden.json`, Baseline v1.0 — sha256 `0426d6a8…ef859`,
> ver §13 do PROTOCOL.md) e as provas de `validate_golden.py`.
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
- Implementadores: `MockDevice` (M0.5 — transporte de TESTE: core, CLI e
  harness, responde conforme o golden) e `RealDevice` (midir/WinMM/ALSA)
  **atrás da feature `real-device`**. No **core/CLI** ela segue default OFF
  (política de hardware, VISION §7). No **app** (`gp100-ui`) é **default ON
  desde 09/10/2026 (#165, Era Hardware)**: `tauri dev`/`tauri build` abrem o
  aparelho sem flag; `--no-default-features` gera o app sem transporte algum
  (backend `Desligado`).
- `Session` (M0.6) é genérica sobre `DeviceTransport` — replay das fixtures não
  conhece hardware.

---

## ADR-5 — Feature-flag `WRITE_VERIFIED` (mock sempre permite)

**Status:** Accepted · **Pré-assinada:** 28/09 (owner) · **rev. 04/10** (feature
`write-verified` + `WireKind` declarado pela `Session`) · **rev. 06/10** (keepalive
do boot omitido com a trava fechada; select = `Read`) · **rev. 09/10/2026**
(#165: o default do APP vira o aparelho — a trava de escrita NÃO muda:
`write-verified` continua feature separada, default OFF; o CLI segue mock por
default, com `--mock-device` explícito de teste) · **Afeta:** M0.5, Gate H (H2)

**Decisão.** Escrita no device **real** é guardada por `WRITE_VERIFIED` — que
virou, na revisão de 04/10 (H2), uma **feature de compilação** `write-verified`
(default OFF), não uma constante de config: a trava é da **build**, então não
existe flag, variável de ambiente ou argumento que a abra num binário que não a
tem. Enquanto `WRITE_VERIFIED == false`, o transporte real **recusa writes com
erro tipado** (`TransportError::WriteBlocked { op }`, com o endereço do frame
recusado). O `MockDevice` **sempre permite** writes — caso contrário os replays
das fixtures (knob/save/IR) não existiriam.

**Como a escrita é classificada (rev. 04/10).** `DeviceTransport::send_raw`
passa a receber um `WireKind` (`Read` | `Write`) **obrigatório, sem default** — a
classificação é **declarada pela `Session`**, que é quem conhece a semântica, e
**não deduzida do byte FUNC**. Deduzir por FUNC estaria errado: `0x12` é usado
tanto para escrita quanto para a leitura de página do §13.10, e um gate por FUNC
recusaria o próprio caminho de leitura. Parâmetro obrigatório (em vez de
`Option`/`default`) porque o compilador é a trava secundária: esquecer de
classificar é erro de build, não erro de campo.

**Estado atual:** os 3 fluxos capturados (set §13.11, save §13.12, IR §13.7) já
foram verificados em campo na S4 (persistência confirmada no display —
BLOCKERS item 11 fechado), mas a feature **nasce OFF** e só é ligada no binário
de campo com o DoD do H2 executado **pelo gp100-core no device** (3 fluxos, um
por vez, com read-back/verificação no display). O PR do H2 **não a liga por
padrão** — entrega o mecanismo e deixa a decisão com o owner e a pedaleira.

**Consequência registrada pelo PR (não óbvia), RESOLVIDA em 06/10:** o keepalive
de boot (`12/00020001`, §13.12) é um frame OUT que não pede resposta — logo é `Write`
pelo critério acima, e o **boot completo (B5 do H1) passava a exigir a feature do
H2**.

**Decisão do owner, 06/10: pular o keepalive com a trava fechada.** O
`Session::boot` pergunta ao TRANSPORTE — `DeviceTransport::permite_escrita()`,
que existe desde 06/10 com default `true` (`RealDevice` devolve `WRITE_VERIFIED`,
o `MockDevice` devolve `true`, e os embrulhos `Box`/`&mut`/`LoggingTransport`
deletram) — e **omite** o ping ×2 quando a resposta é `false`, em vez de mandar
e ser barrado no último frame (um build de LEITURA ficaria sem leitura, que era
justamente o que a alternativa do B5/H1 pedia). Perguntar ao `cfg!(feature)` no
core pularia o keepalive também no mock e mudaria o boot de 2297 para 2295 sem
ninguém ter pedido — por isso a pergunta vai a quem vai enviar o byte. A omissão
é **contábil**: o relatório sai com 2295 transações, que é o que de fato saiu no
fio (`tests/write_gate.rs` prova os dois lados: trava fechada → boot completo sem
ping; aberta → 2297 com ping ×2).

**Decisão do owner, 06/10 — o select NÃO é escrita.** `Session::select_preset`
envia `WireKind::Read` (é o mesmo select com que o `boot()` varre os 199 presets
para LER), e a DoD da #126 listava `select` entre os botões de escrita. Medido no
código, o owner optou por manter a troca de preset **liberada** no build de
leitura e travar só o que escreve: knob, IR, SnapTone e `save_preset`.

**A tela tem UM leitor do campo.** `escritaLiberada(info)` (em
`src/ipc/device.ts`) lê `writeVerified` e é o que o `FieldDiagPanel`, o
`PedalModal`, o `IrLabPanel` e o `SnapTonePanel` usam — botão desabilitado COM O
MOTIVO (`MSG.writeLockedHint`), nunca recusa depois do clique. Coberto por unit
(`tests/writeLock.ui.test.tsx`) e e2e (`e2e/writeLock.spec.ts`, pelo gancho
`gp100.debug.writeVerified`, que simula o build de leitura sem compilar nada).

**Consequências.**
- Divergência de fio descoberta no H1/H2 = fluxo R3 (captura → golden → validate),
  nunca "ajuste" ad-hoc no codec.
- Fluxo de escrita novo (fora os 3 capturados) = proibido até captura própria +
  baseline nova + esta trava reavaliada por ADR.
- Trava **atômica por operação** (ADR-6, consequência 6): `save_preset` é barrado
  ANTES do 1º write do `meta_block`, então nunca sai frame parcial de save.

---

## ADR-6 — `Session` (M0.6): assinaturas da FSM e dispatch do IN ambíguo

**Status:** **Accepted — sancionado pelo owner em 29/09** (rev.3, após revisão
crítica + re-derivação do save do log cru, `analysis/derive_save_ops.py` — D3
definitivo) · **Afeta:** M0.5, M0.6, M0.7 · **Vincula:** ADR-2 (erros), ADR-3
(timeout), ADR-4 (trait), ADR-5 (escrita real)

**Decisão.** A FSM de sessão é um tipo único, genérico sobre o transporte
(ADR-4), que expõe EXATAMENTE as operações capturadas — nada de API especulativa:

```rust
pub struct Session<T: DeviceTransport> {
    transport: T,
    // (rev.2) golden NÃO é campo: `GoldenFile::embedded()` (1 parse por
    // processo) é chamado nos métodos — campo &'static de valor único é
    // ruído de API e impedia injetar golden em teste, se um dia precisarem.
}

impl<T: DeviceTransport> Session<T> {
    /// NÃO abre o transporte (ciclo de vida é do chamador — ADR-4).
    pub fn new(transport: T) -> Result<Self, ProtocolError>;

    /// Boot + scan (§13.10): replay da sequência capturada (boot.jsonl, 2299 OUT)
    /// via GoldenFile::build_request + match_response por transação. O caso
    /// CONGELADO do boot (payload const, 0 vars) vs geral (pp+PG) já é resolvido
    /// pela desambiguação por var_count de `request_template`.
    pub fn boot(&mut self) -> Result<BootReport, ProtocolError>;

    /// Página de estado do pp corrente (família 13xx — dispatch por CONTEXTO,
    /// regra D2 abaixo). Forma byte-a-byte da página 13xx segue FORA (ROADMAP).
    pub fn scan_state(&mut self) -> Result<StatePage, ProtocolError>;

    /// Knob da UI (§13.11): `codec::set_param` fire-and-forget, SEM read-back
    /// (captura não tem resposta — a FSM não inventa uma).
    /// `chain_slot` = 1..=9, posição na CADEIA — NÃO confundir com o `ir_slot`
    /// de `upload_ir` (0..=19, tabela de User IRs; rev.2 nomeou os dois para
    /// evitar troca de slot em chamada posicional).
    pub fn set_param(&mut self, chain_slot: u8, code: u32, ctrl: u8, value: f32)
        -> Result<(), ProtocolError>;

    /// Save (§13.12 RE-DERIVADO — ver D3): `codec::meta_block(pp, pp_type, name)`
    /// + ciclo de ops CAPTURADO na S4: op 0 com o meta, op 0 de novo em +578ms,
    /// op 1 ×2 em +593ms; FIM dos writes = commit. ZERO IN esperado — o save é
    /// fire-and-forget nas 2 amostras. Persistência no display é da pedaleira
    /// (S4, BLOCKERS 11); "salvou?" em H2 = display/`list_user_irs`, NUNCA
    /// interpretar burst 11xx tardio como confirmação.
    pub fn save_preset(&mut self, pp: u16, pp_type: u16, name: &str)
        -> Result<(), ProtocolError>;

    /// IR (§13.7): `ir_begin(ir_slot)` + chunks 15B/33B esperando o ACK
    /// `[slot][idx u16 BE][01]` POR chunk (timeout ADR-3 cada). DUPLICAR o
    /// último chunk é regra da FSM (capturado, idx 0x0226 slot 1) — nunca do
    /// codec. `ir_slot` = 0..=19; `blob` tem de ser múltiplo de 15B (strict,
    /// rev.2: captura só tem múltiplos exatos — pedaço final é REJEITADO, não
    /// padado).
    pub fn upload_ir(&mut self, ir_slot: u8, blob: &[u8])
        -> Result<IrUploadReport, ProtocolError>;

    /// Tabela dos 20 User IRs: req em `12001002`; o by-len do match_response
    /// garante que a resposta lida é a TABELA (75B nibble-exp), não um ACK (4B).
    pub fn list_user_irs(&mut self) -> Result<UserIrTable, ProtocolError>;

    /// Devolve o transporte (close()/reuso) — Session não possui o ciclo de vida.
    pub fn into_transport(self) -> T;
}
```

Tipos de saída (`BootReport`, `StatePage`, `UserIrTable`, `IrUploadReport`) nascem
MÍNIMOS no módulo `session`, derivados das vars que os templates extraem; crescem
só quando a UI pedir (YAGNI).

### Regras de dispatch do IN (o coração do ADR)

- **D1 — A transação é dona do endpoint.** Quem pediu interpreta a PRÓXIMA
  mensagem do `(func, addr)` pedido como resposta, via
  `GoldenFile::match_response` (by-len resolve `12001002`: ACK 4B × tabela 75B).
  Zero heurística fora do golden.
- **D2 — Push é classificado pelo CONTEXTO, não pela mensagem.** O caso
  `13010001` (push e req com os MESMOS 6 bytes) é indistinguível por conteúdo:
  em `boot`/`scan_state` quem decide a semântica é o ESTADO da operação (o que
  está sendo paginado), nunca a msg isolada. A ordem do arquivo no golden só
  desempata FORMA; contexto decide SEMÂNTICA.
- **D3 — Save é FIRE-AND-FORGET (DEFINITIVO — re-derivado do log cru 29/09,
  `derive_save_ops.py`).** ZERO msgs IN associadas ao save nas 2 amostras (S4:
  11s até fechar o Suite sem uma msg; S2: ops isoladas, ±118s de qualquer
  11xx). Os bursts `11000008`×N + `12000001` são eventos de SINCRONIZAÇÃO de
  tabela do Suite/app: na S2 precedidos de 61 requests OUT (são respostas de
  leitura; `12000001` chegou 22s ANTES das ops); na S4 a cópia pós-save
  (+11,4s) veio com janela de 0 OUT = push espontâneo. Logo: (a)
  `save_preset` envia meta_block + ciclo de ops e NÃO espera nada; (b) ciclo
  de ops = o da S4 (única com meta+ops no mesmo evento): op 0 ×2 [0; +578ms]
  → op 1 ×2 [+593ms] — a S2 (só op 0 ×2) não contradiz: o editor já estava
  em modo edição; a FSM usa o ciclo da S4 (superset) e o H1 revalida; (c)
  bursts/espontâneos de IN são tratados por D7 (backlog), NUNCA como parte
  do save; (d) push fora do padrão da transação corrente =
  `ProtocolError::UnexpectedAck` (hex curto).
- **D4 — Fire-and-forget é fire-and-forget.** `set_param`/ops não esperam nada
  e não drenam; se o device empurrar algo, é a transação seguinte que o
  encontra (as fixtures já estão nessa ordem).
- **D5 — Sem match = erro tipado.** Resposta que não casa com nenhum template
  do endpoint = `InvalidShape { expected, got }` (hex curto). Nunca "engolir".
- **D6 — Sem retry no M0.6.** Timeout (ADR-3: 3s/transação, incluindo pushes
  do drain) sobe como `Timeout`. Retransmitir é política nova → ADR no gate H.
- **D7 — Backlog de IN não solicitado (REV.2 — o que o mock não ensina).** No
  device REAL, pushes podem chegar A QUALQUER MOMENTO (usuário mexe knob na
  pedal; pedal empurra página por conta própria). A espera de resposta/ACK é
  um LOOP FILTRANTE: msg de endpoint ≠ esperado vai para um backlog interno
  e a espera continua; o backlog é drenado pela operação seguinte compatível
  (ou por accessor `pending_pushes()` p/ observação). Sem D7, a primeira
  mexida do usuário durante um upload de IR quebra D1.
- **D8 — Consumidor ÚNICO do stream IN.** `Session` é `&mut self` e NÃO
  expõe listener concorrente: dois consumidores do IN (FSM + observador)
  corrompem D1/D7. Monitorar edição ao vivo na pedal (M1/M3) = poll do
  backlog (`pending_pushes()`) ou novo ADR (async).

**Consequências.**
- O `MockDevice` (M0.5) deve SATISFAZER D1–D8 (fila IN por endpoint, ACK por
  chunk, pushes pós-save; o caminho do backlog se testa com transporte
  sintético que INTERCALA um push no meio de um upload) — este ADR é o
  contrato entre M0.5 e M0.6.
- Replay (DoD do M0.6): `Session` sobre um transporte de replay alimentado
  pelas 4 fixtures; divergência = falha com diff hex (skill `protocol-validate`
  ao fechar a issue).
- O CLI (M0.7) chama SÓ estes métodos — binário não toca golden/codec direto.

### Alternativas consideradas (rejeitadas — registro da revisão)

1. **FSM orientada a eventos/callbacks (async)** — rejeitada: ADR-3 (sync);
   revisitar só no live mode (M3). Registrada porque o M1 VAI querer observar
   pushes ao vivo — a válvula de escape é o poll do backlog (D7/D8), não async.
2. **Dispatch IN só pela ordem do arquivo** — rejeitada: empata no by-len
   (ACK 4B × tabela 75B no mesmo endpoint) e o IN real não é puro por fase
   (a fatia S2 do save.jsonl contém ACKs de IR). Ordem desempata FORMA;
   contexto decide SEMÂNTICA (D2).
3. **Tipar o layout da página 13xx agora** — rejeitada: ROADMAP mantém o
   layout byte-a-byte FORA; `StatePage` fica opaco (bytes + vars). Completar
   sem captura nova viola R1.
4. **`Session::open/close` dona do transporte** — rejeitada: ADR-4 dá o ciclo
   de vida ao chamador; `into_transport()` cobre o CLI. Revisitar se o M1
   quiser reconexão automática.
5. **Retry de timeout dentro da FSM** — rejeitada (D6): retransmitir write
   pode DUPLICAR efeito (knob re-setado, chunk re-enviado); decide-se no
   gate H com o link real medido.
6. **Mock com estado derivado das fixtures** — rejeitado: fixtures são do
   replay (M0.6); o estado do `MockDevice` deriva de `all.prst` + dicionário
   (M0.1/M0.2) — detalhes na issue M0.5.

### O que pode quebrar no gate H1 (device real) — watchlist da revisão

1. **Push intercalado** (risco nº 1): sem D7, a primeira mexida de knob NA
   PEDAL durante `upload_ir`/`save_preset` faz a FSM ler página 13xx onde
   esperava ACK. O mock bem-comportado não pega isso — teste H1 com push
   injetado no meio de upload é obrigatório (transporte sintético).
2. **Framing no RealDevice**: o trim no 1º `F7` (cauda stale do WinMM) hoje
   vive nos loaders Python/`decode_envelope`; o transporte real precisa da
   MESMA regra e de buffers de IN ≥ maior página (~200B observado; 4KB
   seguro) — SysEx repartido em dois buffers desincroniza D1.
3. **Pacing das ops do save**: captura mostra ~0–16ms entre ops e UM gap de
   ~580ms antes da 3ª. Sem evidência de delay mínimo: H1 reproduz o pacing
   capturado antes de qualquer 'otimização' (entrar/sair do modo edição
   pode exigir tempo de processamento).
4. **Save sem resposta é fato derivado (D3 rev.3), mas NÃO universal**: vale
   para o Suite capturado; um firmware/manifest diferente poderia responder —
   H1 loga com o proxy em paralelo e valida o ciclo de ops da S4. Não
   interpretar burst `11000008`/`12000001` tardio (S4: +11,4s, espontâneo)
   como confirmação de save; persistência só se atesta em H2 (display).
5. **'Abrir preset na pedal' NÃO tem fluxo batch no fio**: só existe write
   semântico POR parâmetro (§13.11) — carregar preset = N×`set_param` +
   `save_preset` (composição provada por write, não captura de batch). Se o
   H2 exigir mais velocidade, é captura nova (R2/R3).
6. **WRITE_VERIFIED (ADR-5) atômico por operação**: `save_preset` checa a
   flag ANTES do 1º write do `meta_block` — recusa parcial no meio deixaria
   meta pela metade no device.
7. **Hipótese do flush de fim de sessão (quirk §13.7 fechado 29/09):** AMBAS
   as capturas encerram com burst IN espontâneo (S2: 32 ACKs `12001002`
   tardios; S4: `11000008`+`12000001`) — hipótese principal: flush do ring
   MIM_LONGDATA do proxy no fechamento do Suite; alternativa: eco de commit
   de flash do device. VALIDAR NO H1: rodar com o proxy logando em paralelo
   e confirmar que o burst coincide com o FECHAMENTO do Suite (timestamp
   cruzado com o close do app), não com o fim de uma operação. Se vier do
   device (antes do close), o RealDevice precisa tolerá-lo via backlog D7 —
   e o fechamento de sessão no gp100-cli NÃO deve tratá-lo como erro.**Fora do ADR (fica no módulo/teste):** detalhes hex-exatos do ciclo de ops e
pacing (§13.12 re-derivado 29/09 — D3 é definitivo); layout byte-a-byte
 da página 13xx (ROADMAP: fora de escopo); retransmissões (D6).

---

## ADR-7 — `gp100-ui` fora do workspace gnu; toolchain MSVC para o backend da UI

> **Status:** ✅ aceito (M1.0/A-4, 30/09) · **Vincula:** UI_PLAN §2/§9 (decisão
> de estrutura de pastas/stack), ROADMAP A-4

**Contexto.** O spike M1.0 cria o backend desktop `gp100-ui` (src-tauri,
Tauri 2). O workspace é pinado em `stable-x86_64-pc-windows-gnu`
(rust-toolchain.toml da raiz — host e build de campo do CLI sem MSVC). A
compilação do Tauri 2 nesse pin falha de forma irrecuperável no host: o build
script do `tauri` morre com `STATUS_ACCESS_VIOLATION` (0xC0000005) no gnu,
mesmo com `dlltool` resolvido via `llvm-dlltool`. É limitação CONHECIDA do
Tauri no Windows: a linha de suporte oficial é **MSVC** (pré-requisito
documentado desde o Tauri 1; relatos gnu = bugs com workaround de instalador,
não suporte).

**Decisão.**
1. `gp100-ui` fica **FORA do workspace** (`exclude = ["src-tauri"]` na
   raiz): o core e o CLI de campo permanecem gnu; a UI não contamina o pin.
2. `src-tauri/rust-toolchain.toml` próprio com
   `channel = "stable-x86_64-pc-windows-msvc"` — dentro da pasta, o cargo
   usa MSVC sem flags (e sem exigir nada do resto do repo).
3. A CI **é a prova de build**: job `gp100-ui` na matrix 3 OSes — o runner
   Windows traz MSVC nativo; linux/macos sobrepõem `RUSTUP_TOOLCHAIN=stable`
   (o triple MSVC não existe lá). O crate ainda não compila no host de dev
   (sem VS Build Tools instalado) — instalar MSVC no host é decisão do owner
   (`rustup toolchain install stable-x86_64-pc-windows-msvc` + VS Build
   Tools); até lá, dev da UI = CI verde + `pnpm dev` no browser (fallback
   mock do ipc).
4. Deps do crate são **pinadas por cópia** (versões iguais às do workspace;
   `workspace = true` não atravessa a fronteira do `exclude`). Drift de
   versão é pego no review e pelo `cargo update` deliberado.

**Consequências.**
- (+) Core/CLI de campo intocados (R4 do toolchain gnu preservado); UI sobre
  a stack oficialmente suportada do Tauri.
- (+) CI valida a UI nos 3 OSes como qualquer outro projeto.
- (−) Dois lockfiles Rust (raiz + src-tauri) — aceito; upgrades são
  deliberados e separados.
- (−) Dev Windows da UI sem MSVC local não compila o shell — mitigado pelo
  fallback mock do front (`pnpm dev`/vitest) e pela CI.

---

## ADR-8 — Toolchains portáveis para o Dependabot; entry `cargo /` não cobre o workspace raiz

> **Status:** ✅ aceito (30/09, owner) · **Origem:** ativação do Dependabot
> (complemento do security noturno) · **Vincula:** ADR-7 (workspace próprio
> do gp100-ui)

**Contexto.** O Dependabot (`dependabot.yml`, 3 ecossistemas) abre PRs de
upgrade; seu updater roda em container **Linux** e não expõe
`RUSTUP_TOOLCHAIN` — todo cargo ali respeita os `rust-toolchain.toml` do
repo. Canais com TRIPLE de Windows (`stable-x86_64-pc-windows-gnu` na raiz,
`stable-x86_64-pc-windows-msvc` no src-tauri) nem parseiam no Linux
(rustup: `target tuple in channel name`) — as 2 entradas cargo do primeiro
deploy falharam com `dependency_file_not_resolvable` em TODA dependência
(provado 30/09, runs 36702829331/36702829326). É a mesma dependência de
host que a CI já contorna com `RUSTUP_TOOLCHAIN` por perna.

**Decisão.**
1. `src-tauri/rust-toolchain.toml` usa canal SEM triple (`channel =
   "stable"`) — portável: resolve para o triple do host. O alvo EXATO no
   Windows já vem de `RUSTUP_TOOLCHAIN` nos workflows (job `gp100-ui`) e o
   default do host já é `stable-x86_64-pc-windows-msvc`. Nada muda na
   prática: o pin MSVC explícito era redundante (CI pina a perna; host
   Windows ainda não compila o shell — ADR-7, falta VS Build Tools).
2. O pin gnu da RAIZ **permanece** (load-bearing: host sem VS Build
   Tools — sem ele o cargo local cai no default msvc e o link quebra;
   decisão P2 do owner). Consequência deliberada: **a entry `cargo /`
   (lockfile da raiz, core+cli) NÃO existe no dependabot.yml** — o PR
   automático cobre só src-tauri, npm e github-actions. O lockfile da
   raiz segue com `cargo update` DELIBERADO (padrão A-1/A-3) e o
   security noturno DETETA vulnerabilidades (cargo audit já cobre o
   Cargo.lock da raiz — a remediação manual é acionada pela issue
   ACHADOS).
3. Labels do Dependabot (`dependencies`, `rust`, `npm`, `github-actions`)
   são provisionadas ANTES via `gh label create --force` — o Dependabot
   não cria labels e update com label inexistente falha (mesma lição do
   `achados-security`).

**Alternativas rejeitadas.**
- *Portabilizar o pin da raiz também* (`channel = "stable"`): resolvia as
  4 entries, mas transferia o controle da toolchain do repo para o
  `rustup default` do host — exatamente o que o pin P2 existe para
  evitar.
- *Fixar versão exata em vez de "stable"* no src-tauri: o Dependabot
  também não lê canais com triple (o problema é o PARSE do rustup, não a
  mobilidade do canal) e create uma segunda coisa para sincronizar.

**Consequências.**
- (+) Dependabot funcional onde opera (src-tauri + npm + actions), CI
  dispara em cada PR (14 jobs, cache quente).
- (+) Zero mudança de comportamento local/CI: RUSTUP_TOOLCHAIN nos
  workflows é a fonte da verdade das pernas.
- (−) Upgrades do core/CLI continuam manuais/deliberados — aceito: é o
  padrão do projeto (A-1/A-3) e a detecção noturna cobre o risco.

---

## Aplicação

| Issue | ADRs que vincula |
|---|---|
| M0.1 (modelo do dicionário) | 2 |
| M0.2 (.prst round-trip, R4) | 2 |
| M0.3 (golden-file consumer) | 1, 2 |
| M0.4 (codec) | 1, 2, 4 |
| M0.5 (transporte + mock) | 2, 3, 4, 5, 6 |
| M0.6 (FSM + replay) | 2, 3, 4, 6 |
| M0.7 (gp100-cli) | 2, 3, 4, 5, 6 |
| M1.0/A-4 (gp100-ui, spike) | 7 |
| Dependabot (src-tauri + npm + actions) | 7, 8 |
| H1–H3 (gate de hardware) | 3, 5 |
| M2 (#24/#25/#26) | 9 |

---

## ADR-9 — A persistência do app mora num crate próprio do workspace gnu, não no crate do Tauri

> **Status:** ✅ aceito (M2/#26) · **Vincula:** #26 (biblioteca), #24 (IR lab),
> #25 (SnapTone) · **Herdou:** ADR-7 (a divisão gnu/MSVC)

**Contexto.** A #26 pede a biblioteca (fábrica + usuário) em SQLite, com
migrações versionadas, busca e import/export. A pergunta que precede o código é
**onde esse banco vive**, porque #24 (laboratório de IRs) e #25 (gestor
SnapTone/NAM) precisam exatamente do mesmo armazenamento — se a #26 escolher um
substrate, as três herdam.

Duas respostas eram possíveis: `rusqlite` dentro do crate do Tauri
(`packages/app/api`, que já é dono do `AppState`) ou um crate novo. A escolha não
é estética; o ADR-7 já tinha produzido a evidência que decide.

**A evidência que decide.** O `gp100-ui` está FORA do workspace gnu de propósito
(ADR-7: o Tauri 2 exige MSVC no Windows) e por isso os testes dele rodam num
único job, `Testes · Rust (cargo test · ui-rust)`, preso ao runner Windows. E,
medido nesta máquina: o crate do Tauri **não compila local** — o Git Bash do
Windows expõe `/usr/bin/link.exe` (o `link` do coreutils), que sombreia o
`link.exe` do MSVC e mata o link de qualquer crate de build script. A
consequência é direta: **persistência escrita dentro do crate MSVC é
persistência que não se consegue verificar na máquina de desenvolvimento** — só
na CI, e só em uma plataforma.

Isso importa mais do que parece. A #26 pede migrações e import/export: exatamente
as partes que quebram de forma dependente da plataforma. O `sqlite3_libversion()` diverge
entre o SQLite embutido e o do sistema, e um `ALTER TABLE` aceito numa versão
pode ser recusado noutra.

**Decisão.**
1. A persistência é um crate novo, `packages/library` (`gp100-library`),
   **membro do workspace raiz** (gnu). Sem dependência de Tauri: ele não sabe
   que existe uma janela.
2. Ele usa `rusqlite` com a feature `bundled` (SQLite embutido, sem dependência
   de biblioteca do sistema) — o mesmo SQLite em toda máquina, o que torna o
   comportamento das migrações reproduzível.
3. O `gp100-ui` vira **só a casca**: commands `library_*` finos que delegam ao
   crate. Nenhuma regra de armazenamento mora no shell.
4. Os 99 presets de fábrica entram no banco na **primeira execução** (seed do
   `all.prst`), e o seed reutiliza o parser do `gp100-core` — não há segunda
   implementação do formato `.prst` no projeto.
5. A versão do esquema vive em `PRAGMA user_version`. O número é lido do
   próprio arquivo, nunca de uma constante no código.

**Alternativas rejeitadas.**
- *SQLite no front (sql.js/WASM).* Teria footprint de WASM, banco inteiro em
  memória e a busca passaria a rodar na UI. Descartado.
- *Manter `localStorage` versionado.* Trocar de esquema viraria código, não
  migração — que é justamente o que a issue pede.

**Consequências.**
- (+) As migrações, a busca e o import/export são testados no job `Testes · Rust`
  do workspace, **nas três plataformas** da matriz — localmente também.
- (+) #24 e #25 passam a ter um armazenamento com migração de verdade, sem
  repetir a decisão de substrate.
- (−) `packages/app/api` passa a depender de um crate que vive em OUTRO
  workspace. Cargo resolve por path, mas isso precisa de CI para provar.
- (−) O escopo do job `rust` no `ci_plan.py` precisa de `packages/library/` na
  expressão — o mesmo debt de escopo que a #82 §4 apontou.
- (−) `rusqlite` com `bundled` compila o SQLite a partir do fonte: primeiro
  build mais lento e exige um compilador C (minGW no Windows-gnu, cc nos
  containers Linux).

---

## ADR-10 — Trava de conteúdo no `set-param`: faixa do valor antes do fio (#110)

**Status:** Accepted · **Pré-assinada:** 05/10/2026 (owner) · **Afeta:** `gp100-core` (codec + `param_range`), CLI de campo, `DeviceActor` do app

**Contexto.** Em 05/10/2026, no gate H2, o runbook de campo mandava
`set-param 3 0x0700006e 0 99.5`. O `99.5` era um número inventado. O
firmware V2.1 tem, em `Drivers/audio/audio.c:1828`, o assert

```text
CODE:para <= GetParaMaxVal(
```

e ao assertar **para de responder a toda transação**, inclusive às leituras
puras. O device continua enumerado e `OK` no Windows, o que elimina cabo,
driver e porta como causa; a única recuperação é um **power-cycle físico**.
O detalhe cruel é que o `set-param` é fire-and-forget (§13.11, D4): o fio
não devolve **nenhum** aviso, e o operador descobre que quebrou o pedal
quando a leitura seguinte toma timeout.

**Decisão.** O `set_param_payload` valida o **conteúdo** do valor antes de
montar o frame, e uma variante nova de `ProtocolError`
(`ValueOutOfRange { addr, param, got, allowed }`) sobe até a borda. A regra
vem do dicionário (`analysis/parameters.json`, derivado do `algorithm.xml`
oficial da Suite V1.5.1):

| par `(slot, code, ctrl)` | regra | ação |
|---|---|---|
| `knob` | `min <= v <= max` (normalizado por `Control::range`) | **recusa** fora da faixa |
| `switch`/`combox` | `v ∈ option_ids` (id exato) | **recusa** id desconhecido |
| **sem regra** no dicionário | — | **aceita** |
| `NaN` / `±inf` | — | **recusa sempre** |

A chave carrega o **slot**, não só o `code`: o `nibble` do `effectCode` não
é o módulo (`0x05` = C-Wah no PRE, `0x03` = Green OD no DST), e sem o slot
`Boost` e `14 Boost` — mesmo `effectCode`, PRE e DST — colidiriam.

**Por que o dicionário, e não só as amostras.** A armadilha conhecida da
captura é que `knobs.jsonl` é **uma sessão só**: uma faixa derivada dela é
piso, não verdade. Mas o dicionário é a fonte declarada da própria Suite, e
a Suite é quem fala com o firmware. A corroboração está medida e é gate
(`analysis/param_ranges.py --check`): das 92 amostras reais, **13 dos 14
pares observados caem inteiros dentro da faixa declarada, zero
contradições**. Duas fontes independentes — o que a Suite declara e o que
ela de fato manda — concordam, e a Suite nunca escreve fora do que o
aparelho aceita. Cobertura: 639 regras para 639 controles (575 knob +
64 discretos).

**Por que "sem regra" aceita.** O 14º par é `0x0a00002c` (U-ban 4x12, CAB)
com `ctrl = 1`: o aparelho **varreu esse knob de 0 a 99** em 8 amostras
reais, mas o dicionário só descreve o `ctrl = 0` desse cab. É um buraco do
dicionário, não um valor perigoso — recusá-lo quebraria um knob que
comprovadamente funciona. O gate nomeia o par (`observed_without_rule`)
para que o buraco fique visível em vez de sumir. `NaN`/`inf` são recusados
mesmo sem regra: o bit-pattern de `f32` para eles é lixo que nenhum
firmware aceitaria.

**Alternativas rejeitadas.**
- *Só os 14 pares com amostra real.* 625 pares sem medição ficariam sem
  trava justamente onde não sabemos nada. Menos atrito, bem menos proteção.
- *Recusar também o que está sem regra.* Mais seguro contra dicionário
  incompleto, ao custo de travar o knob do CAB que o aparelho aceita.
- *Escape hatch (`--force-param`) em campo.* Deixado para uma necessidade
  real de campo; não entra agora porque a evidência diz que a tabela não
  bloqueia nada que a Suite mande.

**Consequências.**
- (+) A trava é de **conteúdo**, não de política: vale igual no mock e no
  aparelho real, então o H1 (read-only) fica seguro por construção e o
  mock continua exercitando escrita (ADR-5).
- (+) Ela mora no `set_param_payload`, o **único** ponto por onde todo
  `set-param` passa (CLI, `DeviceActor`, replay das fixtures), e **antes**
  de existirem bytes — provado por mutação em
  `packages/core/tests/value_gate.rs` (transporte que pune qualquer byte)
  e no CLI (`set_param_fora_da_faixa_falha_sem_mandar_nada`).
- (−) Se o dicionário declarar uma faixa **mais larga** que o
  `GetParaMaxVal` do firmware para algum controle, a trava deixa passar um
  valor que o aparelho rejeita. É o risco residual: nenhum dos 13 pares
  corroborados mostra isso, e nenhum caminho de escrita novo depende de
  faixa inventada.
- (−) `--dry-run` do CLI passou a recusar valor fora da faixa. É
  intencional: um frame que o firmware rejeitaria não é inspecionável.

---

## ADR-11 — O `gp100-cli` fica congelado para capacidades novas (decisão do owner, 05/10)

**Status:** aceito · owner, 05/10/2026 · pergunta respondida por
`docs/REAL_DEVICE_GAP.md` §6

**Contexto.** O `gp100-core` tem 12 capacidades. O `gp100-cli` e o app expuseram
subconjuntos **diferentes** desse mesmo core: o CLI tinha `save`,
`dump-preset` e o log de fio, e o app não; o app tinha SnapTone e o CLI não.
Por muito tempo o motivo técnico para o CLI existir foi concreto — ele era a
única cobertura de CoreMIDI do transporte USB-MIDI, porque o crate do Tauri do
app só era compilado no Windows.

**O que resolveu o impasse.** Três commits, nesta ordem:
1. `save`/`dump-preset` e o logger/dry-run foram para o **core** e para o
   `app/api` (passos 4 e 4b do gap);
2. ganharam **tela** — painel de diagnóstico de campo, no app (passo 4c);
3. o `ui-rust` entrou na **matriz macOS** da CI, e o caminho do aparelho real
   do app passou a ser compilado para CoreMIDI (passo 6b).

Depois disso o app é um **superset estrito** do CLI: expõe as 9 capacidades do
CLI **mais** o SnapTone. E as duas capacidades que continuam vermelhas para o
app (`0x47` e `set_inventory`) são falta de protocolo e falta de decisão de
campo — nenhuma se resolve trazendo o CLI para dentro do app.

**Decisão.** *Manter, mas congelar.* Capacidade nova **não entra** no
`gp100-cli`; vai para o `app/api` (command + porta em `ui/src/ipc/`) ou não
existe. Correção de bug entra, divergência permanente não.

**Alternativas rejeitadas.**
- *Arquivar o crate agora.* Tira a ferramenta de quem prefere um executável
  sem abrir a UI — e a sessão de campo ainda não aconteceu para medir se
  alguém precisa dela. Arquivar antes da medição é decidir com o número que
  a decisão promete esperar.
- *Deixar sem registro e decidir depois.* Um crate sem política escrita recebe
  capacidade nova por omissão: a próxima feature "cabe aqui" e ninguém
  contesta o rato. A regra precisa estar no arquivo de quem vai mexer, não numa issue.
- *Cortar o job `Distribuição · CLI de campo`.* Continua: ele é o artefato que
  os gates H1–H3 exercitam, e congelar não é aposentar.

**Consequências.**
- (+) O produto tem um dono só. "Onde essa capacidade entra?" deixa de ter
  duas respostas válidas.
- (+) O CLI continua sendo o **executável dos gates**: `--log` no schema P4 e
  `--dry-run` são o insumo do juiz de campo, e ele roda em 3 SOs.
- (−) Uma capability que só faça sentido fora da UI (script de campo
  headless, CI) vai exigir uma conversa de novo. Aceito: são raros e o custo
  de manter dois produtos não é.
- (−) A regra é social. Ela é escrita no cabeçalho de
  `packages/cli/src/main.rs` e no `packages/cli/README.md` — os dois lugares
  que quem for mexer nele primeiro vai ler.

**Revisão.** Quando a sessão de campo (#17) sair assinada pelo aparelho, a
pergunta "o CLI ainda serve a alguém que não o app?" ganha resposta **medida** —
e essa resposta, e não este ADR, decide o futuro do crate.

> **Atualização 06/10 (#132/ADR-12):** das duas capacidades vermelhas do
> contexto, `set_inventory` saiu da lista — o aparelho já nasce com o
> inventário da captura por default. Continua vermelho só o `0x47` (falta
> de protocolo).

> **Atualização 09/10/2026 (#165, Era Hardware):** exceção registrada — o CLI
> ganha `--mock-device`, um **selo explícito do transporte mock** para
> harness/teste (e antídoto contra `--real` acidental num script). Não é
> capacidade nova: o default do CLI segue mock, a flag conflita com `--real` e
> nada muda o comportamento do mock. O app inverteu o default (aparelho ligado
> por padrão); o CLI **não** inverteu — congelamento preservado.

---

## ADR-12 — Um espaço único de `pp`: inventário do aparelho, trava antes do fio, abertura pelo `current_pp` (#132)

**Status:** Accepted · 06/10/2026 (critérios de aceite assinados na issue
#132) · **Afeta:** `gp100-core` (`session`, `preset`, `pedalboard`,
`transport`), `gp100-library` (seed), `DeviceActor`, `useStage`

**Contexto.** Em 06/10/2026 o display do GP-100 mostrou uma assertiva NOVA —

```text
CODE:PresetNum <TOTAL_PA
LINE: 912
file: ..\..\Drivers\audio\audio.c
```

— e o pedal parou de responder até power-cycle físico (a mesma classe do
assert do ADR-10, mas falando de **número de preset**, não de valor de
parâmetro). A investigação (relatório `reports/assert-audio-912.md`, sem
tráfego novo no aparelho) achou **quatro numerações de `pp` convivendo**:

| onde | base | evidência |
|---|---|---|
| `files/patches/all.prst` | strings `"0".."98"` | o próprio arquivo |
| `gp100-core` (board/documento) | o MESMO texto lido como **hex** → `0x00..0x98` espaçado | `preset.rs` (`from_str_radix(…, 16)`); `gain_staging.rs` ("0x0a não existe") |
| `gp100-library` + artefato do front | **decimal** → `0..98` contíguo | `seed.rs`; `dump_preset_list.py` (ASSERTA `0..98`) |
| **aparelho** | `0x0000..0x0062` + `0x0100..0x0162` (198 pps) | captura S1 (`boot.jsonl`) **e** sonda de campo `dump-preset 0x0100` |

E **ninguém validava o `pp`**: o `select_preset` só checava shape, o app
abria com `openPreset(0)` fixo, e o boot do aparelho usava o default
`0..198` — que manda 99 selects fora do aparelho (`0x0063..0x00c5`) e nunca
alcança o banco `0x01xx`, onde o pedal liga. `PresetNum < TOTAL_PA` é
literalmente sobre isto.

**Decisão — quatro camadas, uma por superfície.**

1. **O espaço do fio é o da captura.** [`session::inventario_do_aparelho`]
   devolve os 198 pps da S1 **na ordem da S1** (`0x0100..=0x0162` depois
   `0x0000..=0x0062`). É o DEFAULT do `boot()` **quando o transporte é o
   aparelho** (`DeviceTransport::e_aparelho`, a mesma pergunta por objeto do
   `permite_escrita`); `set_inventory` continua prevalecendo — quem varre
   define a faixa (é o caminho do R3 do `H1_CHECKLIST`, sem número de
   parede: se o aparelho descobrir pps novos, quem varre descobre).
   No mock, o default continua `0..198` e o comportamento é o de sempre.
2. **A recusa é ANTES do frame.** `Session::select_preset` no aparelho
   devolve `ProtocolError::ValueOutOfRange { addr: "11/13010000",
   param: "pp", got, allowed }` — a **irmã da ADR-10**: mesma variante,
   mesma semântica (conteúdo antes de existir byte), e a mensagem traz a
   faixa legível (`0x0000..0x0062, 0x0100..0x0162`) porque o runbook de
   campo precisa saber até onde o pedal vai. No mock não há trava.
3. **`ppID` é DECIMAL numa base só.** `preset_list`, `board_view_for`,
   `apenas_preset` e o `MockState` passaram a ler pelo helper
   `preset::pp_id_decimal`, o MESMO que a semente da biblioteca usa. O hex
   era um bug independente do aparelho: `board(24)` casava o bloco
   `ppID="18"`, e a partir de 10 todo clique da lista (decimal) abria outro
   bloco. Um pp do fio com byte de banco vira índice do documento pelo
   `preset::indice_do_documento` (`0x01XX → XX`; byte alto fora de
   `0x00`/`0x01` fica INTACTO, para o lookup falhar em vez de adivinhar).
4. **A abertura automática não adivinha.** `useStage` abre o
   `current_pp` que o backend reporta (`device_preset_library().currentPp`)
   em vez do `0` fixo — no aparelho, o valor vem do que o boot varreu e a
   trava do item 2 garante que só um pp provado sai daqui.

**Alternativas rejeitadas.**
- *Manter `0..198` como faixa do aparelho.* É o frame do assert: 99 selects
  fora do banco e o banco real fora do scan. O `0..198` era um chute de
  contagem ("199 presets"), não uma medição.
- *Travar também o mock.* O espaço do mock é o do documento e os testes
  varrem pps arbitrários (`0xffff`, replay com inventário próprio). A
  trava é de APARELHO — igual à política de escrita, a identidade mora no
  transporte (`e_aparelho`), não no `cfg!(feature)`.
- *Unificar em HEX (a semente passar a ler hex).* A lista hex não existe no
  aparelho: `"70".."98"` viraria `0x70..0x98`, fora dos dois bancos
  provados, e deixaria de ser contígua — o `dump_preset_list.py` já
  ASSERTA `0..98`. Decimal é a base que o aparelho usa.
- *Ler o push `12/12000000` (primeiro IN da captura) como "corrente".* O
  próprio golden o descreve como "status 2B no boot (§13.3)"; transformá-lo
  em `current_pp` é outra medição, não esta correção.

**Consequências.**
- (+) O boot do aparelho reproduz a sequência do Suite: mesmo inventário =
  mesma contagem (2299, provada contra a captura por `tests/pp_gate.rs`).
  Mock e docs seguem 2297/2295 — nada muda para quem não tem aparelho.
- (+) A trava cobre CLI e app no mesmo ponto (`Session::select_preset`),
  antes do golden e do frame.
- (−) Se uma unidade tiver pps fora da captura, a trava recusa até
  `set_inventory` ser ligado ao que o boot descobrir (R3 do H1_CHECKLIST).
  Aceito: recusar é o lado seguro — o outro lado é o power-cycle.
- (−) `e_aparelho` precisa ser declarado com verdade por quem emula o
  aparelho (`RealDevice` → `true`, `LoggingTransport` repassa, o
  `AparelhoFake` dos testes declara). É o mesmo contrato de
  `permite_escrita`.

**Medição pendente (critério 5 da issue).** O teste de campo — UM `select`
no pp que o scan devolveu, conferindo o display — continua sendo a porta
para a retomada da sessão A/B (#116). Este ADR não fecha campo; fecha o
que o código pode provar sozinho.
