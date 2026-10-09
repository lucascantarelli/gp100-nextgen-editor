# PROTOCOLO USB — Valeton GP-100 (e família GP)
**Versão:** 2.0 · **Data:** 2026-09-28 · **Status:** ✅ referência atual
**Como ler:** §13 = protocolo de FIO (confirmado em campo, capturas 1–4) — é o que o
gp100-core fala; §1–12 = formato de ARQUIVO (TLV/CRC-8/objetos). A especificação
executável byte-a-byte é `docs/protocol_golden.json` (validada por
`analysis/validate_golden.py`). Mapa geral da documentação: `docs/INDEX.md`.
**Histórico:** framing do GP-50/GP-5 validado por terceiros (§7); GP-100 confirmado
DIFERENTE no wire-format por disassembly (§9) — mesma CRC-8/0x07, framing TLV de bits.

---

## 1. ARQUITETURA DE TRANSPORTE (confirmada no GP-100.exe)

O `GP-100.exe` V1.5.1 (app **JUCE**/C++, 18 MB) expõe duas interfaces USB (strings da firmware: `GP-100 Audio`, `GP-100 MIDI`):

| Interface | Uso |
|---|---|
| `GP-100 Audio` | USB Audio Class 2 — streaming de guitar/USB para DAW |
| `GP-100 MIDI` | USB MIDI Class — **todo o protocolo de controle (SysEx vendor)** |

Evidências no executável:
- Imports `midiInOpen/midiOutOpen/midiInAddBuffer/midiOutLongMsg` (Win32 MIDI) via JUCE (`Win32MidiService`, classes `MidiInput/MidiOutput`).
- Classes do protocolo: `MIDIManager`, `MIDIMSGProcess`, `MIDIUSBFsm` (máquina de estados), `MIDIListener`, `MIDIMessageListener`.
- Threads de alto nível: `GetPresetsThread`, `RequestPreset`, `RequstPresetPerPackage`, `UpdateAllPreset`, `UpdateAllPresetsBackgroundThread`, `UploadOnePreset`, `FirmwareUpdate`, `UpdateFirmware`, `CheckUpdateThread` (HTTP a `valeton.net`), `NamConvertThread` (NAM core embutido: WaveNet/LSTM/ConvNet + Eigen + r8brain), `DataManager`.
- Strings de log de debug: `row:%d,effectCode: %08X`, `checksum:%d`, `IR Data:`, `Nam Data:`, `ppIRCRC`, `%02x `.

**Conclusão de transporte:** comandos via **SysEx vendor em USB-MIDI**; sem endpoints HID custom. O driver ASIO (Valeton_UsbAudio) é apenas para áudio.

---

## 2. FRAMING DO PACOTE (host → device)

Layout GP-50 (família), confirmado 298/298 contra capturas do Valeton Suite:

```
BUF (pré-nibble):
  [0] CRC-8      (ver §3; calculado com este byte zerado)
  [1] command    (0x01 leitura/comando; 0x1D write de preset; 0x92 upload SnapTone; ...)
  [2] index      (índice do bloco na transferência, 0,1,2,...)
  [3] length     (nº de bytes de payload; blocos cheios usam 0x13 = 19)
  [4..] payload  ([3] bytes)

wire = 0xF0 + nibble-expand(BUF, hi-first) + 0xF7
```

Nibble-expand: cada byte b vira dois bytes MIDI `[b>>4, b&0x0F]` (hi primeiro). Um bloco de 19B de payload = 23 bytes BUF = 46 nibbles = 48B no fio.

Requisições de leitura conhecidas (família): `BUF = [crc, 0x01, 0x00, 0x02, 0x12, selector]` com selector `0x40` = tabela de nomes (100 patches), `0x41` = corpo do patch ativo (511B), `0x24` = SnapTone, `0x20` = IR.

Respostas: message type `0x02` (vs `0x01` comandos); ACK de status com 16 bytes fixos; respostas de dados chegam reassembly multi-pacote com tag `cmd` e eco do selector (`12 41`).

---

## 3. CHECKSUM — CRC-8, polinômio 0x07

```
CRC-8/SMBUS: poly 0x07, init 0x00, no reflect, no final XOR
Calculado sobre TODO o BUF (cmd, index, len, payload) com BUF[0] = 0.
```
```python
def crc8_07(buf):
    c = 0
    for b in buf:
        c ^= b
        for _ in range(8):
            c = ((c << 1) ^ 0x07) & 0xFF if c & 0x80 else (c << 1) & 0xFF
    return c
```
Referências: tabela de 256 entradas presente em apps da família; implementação verificada 298/298 pela comunidade (GP-50) e reproduz os 6 CRCs do gp5-wc (GP-5).

---

## 4. ESCRITA DE PRESET (op `0x1D`) — VALIDADO NA FAMÍLIA

Payload lógico:
```
device_payload = [0x11, 0x4F, slot, 0x00, 0x00, 0x00] + prst[0x19:]
```
- `0x11 0x4F` — marcador constante da família (0x4X = família de comandos de edição: 0x43 select, 0x47 change-effect, 0x48 change-param, 0x49 toggle-block; 0x4F = bulk write).
- `slot` — índice do patch **0-based**.
- `prst[0x19:]` — corpo do .prst a partir do nome (substitui o sentinel `FF FF FF FF`).
- Stream: blocos de 19 bytes, index 0..N, cada um `BUF = [crc, 0x1D, index, len, chunk...]` nibble-encapsulado em `F0..F7`; o device ACKa cada bloco. **Autocommit** (não há pacote de commit).
- Validação: 29/29 pacotes byte-exatos vs captura; write real verificado com read-back (slot 90 do GP-50).

**Regras de segurança impostas pela comunidade (adotaremos):**
1. Uma única porta persistente; uma requisição por vez; settle ≥ 0,25 s entre operações (0,15 s corrompe; loop tight **trava a pedaleira**).
2. Nunca enviar tráfego adivinhado — só stream reconstruído e validado byte-a-byte contra captura real.
3. Primeira escrita sempre em slot vazio + read-back antes de tocar em patches reais.
4. Leitura de banco: Program Change (seleciona slot) → `0x41` (lê corpo). Sem bulk read: ~1 s por patch.

---

## 5. SNAPTONE UPLOAD (op `0x92`)

Stream de blocos de dados `cmd=0x92`, index 0..N, payload de 19B (último curto), transportando ~2,7 KB do modelo convertido (SnapTone = refit compacto feito no **desktop**; o DSP da pedaleira não roda NAM nativo). ACK de 16B por bloco. Detalhes da conversão: `REFIT_FINDINGS` (projeto GP-50) — `startClone` ajusta static-nonlinearity + convolver IR (~16 wide, byte-quantized).

No GP-100 V2.1 há 5 slots (`SnapTone1..5`, flash `FLASH_SNAP_TONE_HEAD_SIZE`), módulo AMP nibble 0x08.

---

## 6. FORMATO `.prst` DA FAMÍLIA (referência GP-50; GP-100 usa XML)

GP-50 (binário): header `GP-50` 20B + CRC-8@0x14 + sentinel 4B + nome 16B + corpo 511B.
**GP-100 (XML)**: formato nativo é o XML documentado em `docs/VISION.md` §4.1 (presets `pp*`/`Effect`), com CRC por User IR (`ppIRCRC`).

---

## 7. FONTES DA FAMÍLIA GP (protocolo compartilhado, evidência de terceiros)
- `drewmerc302/valeton-gp50` — `re/DEVICE_READ.md`, `re/DEVICE_WRITE.md`, `re/SNAPTONE_PROTOCOL.md`, `re/DEVICE_BLOCKORDER.md` (transporte, CRC, opcodes 0x40/0x41/0x1D/0x92, regras de settle).
- `helvecioneto/gp5-wc` (`ble_sysex.json`), `solispensa/Chocotone` (`GP5Protocol.cpp/.h`) — confirmação independente GP-5.

## 8. DIFERENCIAIS GP-100 A CONFIRMAR (captura MIDI spy do Suite GP-100)
- `gWorkMode` (PATCH/STOMP), drum engine (`drumGenre/drumBpm/drumParams`), Global EQ (`HTGlobalEQ`), No-Cab mode, looper.
- Mapeamento de bank/patch para Program Change (bank MSB/LSB?).
- Comando de leitura global (`gParam/gPatch/gBank/gChannel/gVolume/gInput/gUsbGain/...`).

## 9. DESASSEMBLY DO GP-100.exe — RESULTADO (v2.0)
Método e evidências completas: `analysis/FINDINGS_PROTOCOL.md` (scripts em `analysis/`, venv com capstone+pefile).

**Veredito: o GP-100 NÃO usa o wire-format nibble+F0..F7 do GP-50.**

Evidências diretas no código (VA, base 0x400000):
- `0x4B48B0` = `BitStream::writeBits(value, nbits)` — 135 xrefs; o serializer
  principal `0x6209D0` monta os pacotes com campos de 1/4/7/8 bits +
  `writeBuffer` (`0x4B4970`) para blobs — **não há nibble-split nem framing F0/F7**.
- CRC-8/0x07 presente (`crc8` em `0x620920`, tabela em VA `0x1A21F40`) — o
  **checksum é compartilhado com a família**.
- Formato de objeto = **TLV binário** com jump-table de 7 tipos (VA `0x621000`)
  e enum de tipos de dados em `0x620570` (tags u32 0x0C0..0x8000 → IDs 1..0xF),
  incluindo um sub-branch de **sample rates** (8000/16000/22050/44100) —
  transferência de IR/áudio embutida no protocolo.
- FSM de sessão na mega-função `0x626xxx` (estado ~0x2E38 bytes/objeto,
  flip-flop de sincronismo `[obj+0x1AE0]`, magic de sessão gravado com
  literal 0x664C6143 e buffer de 0x20 bytes).

**Consequências práticas:**
1. Nenhum pacote do GP-50 pode ser reutilizado no GP-100 no nível de wire.
2. A camada lógica (opcodes de leitura/escrita de preset) do GP-100 ainda
   precisa de: (a) captura MIDI spy do Suite GP-100 — caminho recomendado — ou
   (b) continuação do disassembly da FSM `0x626xxx` + handlers `0x621020/
   0x621030/0x621040` e tabela `0x621210`.
3. O codec do gp100-core deve ser escrito para o **formato TLV de bits** do
   GP-100, reaproveitando apenas a CRC-8/0x07 e a semântica de objetos
   (nomes/corpos/IRs) já documentada.

---

## 10. SESSÃO 2 DE DESASSEMBLY — CODEC RESOLVIDO ESTRUTURALMENTE
Detalhes completos: `analysis/FINDINGS_FSM.md`. Correções à §9:

1. **A tabela `0x621210` não é jump-table** — é *thrunk* MSVC de `this`-adjust
   (múltipla herança). `0x621020` = set-state trivial; `0x621030` = no-op;
   `0x621040` = `Session::search_sequence` (busca u32 no buffer de sessão
   `[obj+0x2DEC..]`, resultados em `[obj+0x2E20..0x2E38]`).

2. **O encoding de payload é VLQ MIDI-style (varint com bytes de continuação
   0x80|xxxxxx), NÃO nibble-split**: `0x6331D0` (64-bit) e `0x633520` (32-bit)
   implementam `writeVarLen` com prefixos 0xC0/0xE0/0x3000/0xE000 por magnitude.
   Os "opcodes" do GP-100 são **tags VLQ geradas em runtime** — por isso não
   aparecem como imediatos isolados no binário (diferente do GP-50, que os tem
   empacotados). Isto invalida a busca por `11 4F` e congêneres no GP-100.

3. **Cadeia MIDI confirmada**: input `midiInOpen(callback=0x4AC660,
   CALLBACK_FUNCTION)` → demux MIM_DATA/MIM_LONGDATA → listeners; output via
   vtable única `0x1A4399C` (slot 3 = `sendMessageNow` @ `0x4AD1D0`:
   short/long + retries ×0x32/×0x1F3). Todo envio passa pela vtable.

4. **Constantes de objeto confirmadas no serializer `0x6209D0`**:
   `0x06/0x0E/0x0F` (IDs de tipo 4 bits), `0x24` = objeto IR-blob com sample
   rate codificado em sequência (8000→4, 16000→5, 22050→6, 44100→7, 24000→8,
   32000→9), `0x40` = buffer de hardware-params.

5. **Plano de enumeração de comandos** (substitui a busca cega por opcodes):
   disassemblar os *builders* em `0x633xxx–0x634xxx` (todos os chamadores de
   `writeVarLen`) e extrair as tags/consts que cada um emite → tabela de
   comandos host→device derivável 100% por RE estática, com a captura real
   restando apenas como validação final (e não como fonte primária).

---

## 11. SESSÃO 3 — OS 7 FORMATOS DE OBJETO DO SERIALIZER `0x6209D0`
Documentação completa com offsets e bit-widths: `analysis/FINDINGS_OBJECTS.md`.
**Veredito da sessão 4** (`analysis/FINDINGS_COMMANDMAP.md`): estes objetos são
os payloads de um protocolo **state-sync orientado a objetos** — o envelope de
4 bits + classe de 3 bits endereça o objeto; não há opcodes de operação.

Resumo dos 7 layouts (struct base: `u32 type@0x00`):
| type | nome | conteúdo |
|---|---|---|
| 0 | RAW_BLOB | size + data (com back-pad `0x20 - [obj+0x10]`) |
| 1 | U32_LENGTH | writeBits(len*8) |
| 2 | HEADERED_BLOB | header 4B + data (size-4) |
| 3 | PATTERN_TABLE | count × {u64, u64, u16} (stride 0x18) |
| 4 | STRING_TABLE | cookie 0x20 + assert-string 32B + count × {len u32, bytes} |
| 5 | IR/HW_PARAMS | hdr 0x80 + blob + 0x817 + sessões (ts u64, path 12B, flags, subtags 0x6E/0x18) |
| 6 | DUAL_STRING_U64 | u32 + 2 strings (len-prefixed) + 3×u64 |

**Reinterpretação de alto nível**: o tipo 4 carrega um fragmento de assert
`"e sheet track index offset must "` como cookie de 32 bytes — estes objetos
são de **sessão/projeto musical** (letras/sheet + IRs/SnapTones + patterns +
metadados), ou seja, o formato de export/sync de "song session" da Suite, e
**não** os comandos de edição de preset. A camada de comandos permanece nos
builders VLQ (`0x6331D0`/`0x633520` + chamadores `0x633xxx-0x634xxx`).
Constantes mágicas: `0x817` (assinatura de sessão IR), subtags `0x6E`/`0x18`.
Primitivas resolvidas: `0x4B49B0` = writeU32 LE (4×8 bits), `0x4B4A20` =
writeBytes(ptr,push), `0x4B4800` = reserva/zero-fill de N bits.

---

## 12. SESSÃO 4 — MAPA DE COMANDOS FECHADO (state-sync, não opcodes)
Evidência: `analysis/FINDINGS_COMMANDMAP.md`. As 4 únicas chamadas VLQ/tagmap
no binário inteiro:
```
0x6208A9 -> 0x633520 (varint32)    0x6208B6 -> 0x6331D0 (varint64)
0x622FFE -> 0x620570 (dispatcher)  0x623188 -> 0x620570 (dispatcher)
```

### Envelope universal de objeto (fechado)
```
class(3 bits) + small(1 bit) + varint(valor) + type-field
payload: um dos 7 formatos (§11)
integridade: CRC-8/0x07
```

### Correlação com o GP-50
O GP-100 implementa o MESMO modelo lógico do GP-50 (reescrever o objeto
inteiro por slot), mas com envelope orientado a objetos em vez de opcodes
0x40/0x41/0x1D. "Ler" e "escrever" são tipos de objeto endereçados por
(classe, small, type-field) — não há tabela de opcodes de operação.

### Valores numéricos
"floats" viajam como **fixpoint decimal** (divisões por 10/100 em software no
encoder: multiplicadores 0xCCCCCCCCCD e 0x10624DD3 no código) — parâmetros de
DSP nunca são IEEE754 no fio.

### Estruturas de sessão
O dispatcher `0x620570` seleciona pares de registros de 0x124 bytes em 4
bancos (bases 0x150/0x398/0x1390/0x15D8; índices em [obj+0x1A70/74/90/94]) —
gestão de buffers de slots de preset/IR em RAM, coerente com state-sync.

### Status final do mapa
| Item | Estado |
|---|---|
| Transporte USB-MIDI (winmm, callbacks, retries) | ✅ |
| Envelope de objeto (bits, varint, type-field) | ✅ |
| 7 payloads de objeto com offsets/bit-widths | ✅ |
| Numérica (fixpoint decimal) | ✅ |
| CRC-8/0x07 | ✅ |
| Modelo lógico (state-sync por slot) | ✅ |
| Valores de campo específicos (0x817, flags de sessão) | ⚠️ 1 captura real OU logs do Suite |

## 13. SESSÃO 5 — CAPTURA REAL (ROTA 1): FORMATO DE FIO CONFIRMADO

Fonte: `analysis/captures/session1.jsonl` (4.653 eventos MIDI, Suite instrumentado
com proxy winmm.dll; sessão = scan → trocar preset → editar knob → save).
Decoder: `analysis/decode_wire.py`. Build do proxy: `analysis/build_proxy.py`.

### 13.1 Envelope SysEx (não é o envelope de objetos §10-11!)

```
F0 21 25 7F | 47 50 2D 64 | FUNC | ADDR(4B, big-endian) | DATA... | F7
     │            │           │      │
     │            │           │      └─ endereço de 32 bits, MSB primeiro
     │            │           └─ 0x11 = leitura (PC pede) · 0x12 = dados (ambas direções)
     │            └─ "GP-" + 0x64 (100 dec) → GP-100
     └─ Valeton (ID de fabricante 21 25 7F)
```

O envelope orientado a objetos (§10-11: class/small/varint/type-field, CRC-8)
é o formato de **arquivos** de sessão/IR no disco/firmware — o fio MIDI usa
SysEx de endereçamento plano, estilo Roland. São duas camadas distintas.

### 13.2 Códigos de função observados

| FUNC | Direção | Significado | Evidência |
|---|---|---|---|
| `0x11` | PC→GP | **READ REQUEST** (sem dados) | 251× (addr 13/11/12 xx) |
| `0x12` | GP→PC | **READ RESPONSE** (dados) | 2.323× |
| `0x12` | PC→GP | **WRITE** (com dados) | 2.015× |

Não há ACK separado: o slave responde à escrita reenviando o estado (0x13xx)
ou ecoando dados (ver 13.4).

### 13.3 Mapa de endereços observado (4 bytes, MSB primeiro)

| Endereço | Bloco | Conteúdo observado |
|---|---|---|
| `13 00 00 00` | **E** | Estado global do preset atual (~200B lidos como 1 dump) |
| `13 01 00 xx` | **E** | Página `xx` do estado do preset (resposta 210B, ~9 páginas) |
| `13 01 00 02..08` | E | Escritas de 1-4 bytes → commit de edição |
| `13 02 00 07` | E | Lista de slots/índices (42B; contém contadores 05/06) |
| `12 00 10 02` | D | Tabela de nome/rotulação (89B; `0F` repetido = vazio) |
| `11 00 00 00` | B | Identidade: nome ASCII `"Metal"` (28B) |
| `11 00 00 04..` | B | Metadados (34B/64B escritos no save) |
| `11 00 00 07` | B | Bloco de 64B zerado (escrito no save) |
| `11 00 00 08 xx yy` | B | Páginas de leitura longa: resposta pagina `08 xx yy` com F7 **embutido** a cada 28B e continua |
| `12 00 00 00` | D | Escrita de controle (22B, 7× durante save) |
| `10 01 00 02` | A | Escrita com contadores (`08` = nº de itens) |
| `00 00 02 00` | — | Escritas de 2 bytes + `01/02 00 00` (modo/play state?) |

Resposta `12 13 01 00 03` (página 03, 210B): byte 1 = **versão do estado**
incrementa a cada escrita aceita (state-sync: Slave confirma incrementando
o contador da página — coerente com §12).

### 13.4 Padrões de transação

> **Referência executável:** `docs/protocol_golden.json` (gerado por
> `analysis/build_golden.py` a partir das sessões 1–4) contém os 40 templates
> request→resposta com padrões de payload (const/var) e semântica por
> endereço. Este §13 é a narrativa; o golden é a referência byte-a-byte
> para implementação e fixtures de teste.
> **Baseline v1.0 (congelada 28/09/2026 — regra R2/R3 do ROADMAP):**
> - `docs/protocol_golden.json` sha256 `0426d6a8843c57d16ab917170ed5f50eb95026281f1f460f4c2520aaeefef859`
> - `analysis/parameters.json` sha256 `196c67992cd072bbd78c1f7ce3cfc615133d73934e2aa034c6f32b13c171a020`
> Qualquer mudança nesses arquivos exige: nova captura → build_golden → validate 100% →
> novo hash + justificativa aqui.
>
> **Validado por** `analysis/validate_golden.py` — 5 provas:
> accounting IN 100% / OUT 99,57% (sobras = fragmentos truncados do ring
> buffer do proxy); regeneração byte-a-byte: knobs 89/89 + 3/3 (§13.11),
> boot/scan 2.299/2.299 (§13.10), save 77/77 (§13.12, metadados gerados do
> .prst), upload de IR 1.186/1.186 (§13.7, framing por regras) — 100% de
> cobertura generativa em TODAS as fases de captura.
> Nota de ordem: o `10050001` (BEGIN/reserva do slot de IR) vem ANTES do
> burst de chunks; o fim do upload é o último chunk duplicado, sem commit no fio.

**Edição de knob** (⚠️ modelo antigo da sessão 1 — supersedido pelo §13.11:
a edição real usa writes semânticos `10xx0002`; o tráfego abaixo é o
download de estado do scan de presets):
```
PC→GP  12 13010004  01 0000 NN 01     (NN = valor 0..8; 19B)
GP→PC  12 13010003  010000VV + página  (210B; VV incrementou: 01→02→03→…)
```
Sem ACK dedicado: a resposta é a página de estado atualizada (read-back).

**Save (t≈37-68s):**
1. `12 00020000 00/01/02 00 00` — 3 comandos de modo
2. `12 10010002 …` ×3 — writes de knob por hardware (C-Wah Range; ver §13.11 —
   a leitura antiga "metadados, 0x08=contagem" estava errada: 0x08 = índice do alg)
3. `12 11000000 49742773204750313030…` — **nome ASCII direto no fio**: `It's GP100`
4. `12 11000004/05/07` — metadados + bloco zerado 64B
5. `12 12000002 0000000000000000` — commit
6. Burst de 33 leituras paginadas `11 00 00 08 xx yy` (verificação pós-save)

### 13.5 Artefatos de captura (não do protocolo)

- Buffers MIM_LONGDATA chegam com `dwBytesRecorded` não-confiável no
  callback → lixo **depois** do F7 (buffer reutilizado). Trim no 1º F7 é seguro.
- Resposta de leitura longa pagina com F7 **embutido** (a cada 28B) e continua
  no próximo buffer — não confundir com fim de mensagem.
- 1 fragmento órfão de 11B sem header (cauda de reassembly no Suite).

### 13.6 Calibração pendente (foco atualizado)

| Item | Estado |
|---|---|
| Envelope de fio, FUNC, endereços, transações | ✅ confirmado em campo |
| Semântica de cada página do bloco `13 01 00 xx` | ⚠️ cruzar com parameters.json |
| Bloco `00 00 02 00` (modo?) e `10 01 00 02` | ⚠️ sessão 2 |
| Magic 0x817 / flags tipo-5 (formato de ARQUIVO) | ⚠️ exige import de IR (sessão 2) |
| WRITE_VERIFIED (editar e ler de volta pelo app próprio) | ⚠️ após gp100-core |

### 13.7 Upload de IR do usuário (SESSÃO 2, confirmado em campo)

Timeline da captura: 2 bursts idênticos de 5s (import mono @85-90s, estéreo
@130-135s), save com metadados @149s, varredura pós-save @159s.

**Transação de upload** (PC→GP, ~5s por IR):
1. `12 10 01 00 02 | 01 00 00 00 0a` — BEGIN: slot 01, modo 0x0a (upload)
2. 296× `12 12 00 10 02 | [slot u8] [idx u16 BE] [30 nibbles]` — dados
   - cada BYTE do payload carrega UM NIBBLE (pares de nibbles = byte real)
   - 30 nibbles/msg = **15 bytes reais**; idx conta CHUNKS em páginas de 128
     (0-127, 256-383, 512-…) — as "lacunas" de índice são fronteira de página,
     NÃO perda. RAZÃO PROVÁVEL DOS GAPS (achado M0.5, 29/09, evidência:
     replay dos chunks no gp100-core): os bytes de idx viajam CRUS no
     payload e `idx_hi/idx_lo` na faixa 128–255 geraria um byte `F7`
     cru no meio do SysEx — que trunca a mensagem (regra do trim no 1º
     F7). Com páginas de 128 (base = página×256), o byte ALTO de idx só
     assume 0x00/0x01/0x02 e o BAIXO fica contido em 0x00–0x7F — F7 é
     IMPOSSÍVEL nos dois bytes do índice (se idx fosse contínuo,
     0x00F7/0x01F7/0x02F7 existiriam: o replay do M0.5 bateu exatamente
     no 0x00F7 = chunk 247 contínuo).
   - ACK por chunk: GP→PC `12 12 00 10 02 [slot] [idx] 01` (18B)
   - último chunk DA ÚLTIMA PÁGINA (idx 0x226 = 550 = 512+38; 295 = 128
     +128+39) é enviado 2× — o marcador de fim é a DUPLICAÇÃO; CORREÇÃO
     29/09: o payload NÃO é `0F`×15, é a cauda REAL do blob (slot0
     colapsa `b0 b0 10 20 f0 …`, slot1 `00 0a 07 06 0f …`);
     blob = 295 chunks únicos × 15B = 4.425B
3. `12 10 05 00 01 | 00 [slot] 00 00 01 00 00 0a` — BEGIN/reserva do slot, **8B
   cru** (correção v2 da ordem: pelo timestamp, este write vem ANTES do burst de
   chunks — é a reserva do slot de IR; o fim do upload é a duplicação do
   último chunk, sem commit no fio); slot = u8 do IR de destino (`00 …` slot 0,
   `00 01 …` slot 1)

**Blob de IR no device (4.425B = 295 chunks × 15B):**
```
[0..15]  nome do arquivo original, null-padded ("test_ir_mono\0\0")
[14..15] u16: 0x00BC (mono) / 0x00B4 (estéreo) — campo a calibrar
[16..33] zeros
[34..]   dados: samples int32/int24 LE (orig[5..8] batem EXATAMENTE nos
         offsets 55/60/64/68 do blob mono)
```

**NÃO é o WAV cru**: 4.425B << 33.075B do arquivo (11.025 samples int24).
O device armazena representação reduzida/transformada (comprimento a calibrar
na sessão 3 com IRs de durações variadas). Primeiros 5 samples-tiny da
assinatura aparecem com alinhamento irregular (provável round-trip float) —
os samples de magnitude normal comprovam int24/int32 LE nativo.

**0x817**: 0 ocorrências em 5.836 eventos de fio. O magic é do FORMATO DE
ARQUIVO (sessão/export), não do fio. Próximo passo: exportar biblioteca/
preset para disco pelo próprio Suite e varrer o arquivo local (não precisa
de hardware).

**QUIRK FECHADO (29/09) — os 32 ACKs tardios da S2:** as ÚLTIMAS mensagens do
log S2 são ACKs `12001002` de slot 1, idx `0x208..0x226` (+dup), chegando em
+159,1s — 26,5s após o fim do upload e 9,5s após o meta write, com ZERO OUT
na janela inteira. Mesmo padrão de FIM DE SESSÃO na S4 (33 msgs IN
`11000008`+`12000001`, últimas do log, +10,8s pós-ops, 0 OUT — ver §13.12).
Ambas as sessões encerram com um burst IN espontâneo. Hipótese principal:
flush dos buffers MIM_LONGDATA do proxy no fechamento do Suite (a cauda do
ring é entregue no close/reset do midiIn); alternativa: eco de commit de
flash do device. EM QUALQUER CASO: não é resposta do save (D3 do ADR-6
mantida), não é retransmissão do upload, e a FSM NÃO modela — o backlog D7
absorve; o MockDevice não emite.

### 13.8 CAÇADA AO 0x817 — RESOLVIDA (sem hardware)

Evidência: `.prst` exportado pelo Suite é **XML puro** (`<?xml …><GP-100>…`),
zero ocorrências byte-aligned de 81 7F / 08 17 e 1 falso-positivo bit-shifted
(texto `ms_1="0.5"` — decodifica como 0x817 no shift 3; NÃO é o magic).
Varredura de imediatos no .text (`analysis/find_817_imm.py`): exatamente
**2 usos reais**, ambos no par serializer/deserializer de objetos:

| VA | Papel | Código |
|---|---|---|
| `0x620CEF` | **writer** (dentro do serializer `0x6209D0`, ramo tipo-5) | `writeBits([edi+0xa0]!=0, 1)` → `writeConst(0x817)` → `writeBits([edi+0xa4], 8)` |
| `0x62A15B` | **reader** (deserializer) | `cmp campo, 0x817; setne; mov [ebx+0x90], eax` (flag "formato ≠ 0x817") |

**Conclusão:** 0x817 é o **marcador de versão/compat do cabeçalho do objeto
tipo 5 (IR/HW_PARAMS)** — constante de enquadramento escrita após o header de
0x80 bytes e 1 bit de flag, verificada na leitura. Nunca aparece no fio MIDI
(0/5.836 eventos) nem nos `.prst` (XML). Alvo de captura ENCERRADO; os
subtags 0x6E/0x18 e flags de sessão só importam para implementar os arquivos
internos de song-session — fora do escopo do editor (presets = XML, IR = fio).

### 13.9 Formato `.prst` = XML (preset library, resolvido por sorte)

Encontrados em `files/patches/`: `all.prst` (413.629B, biblioteca completa),
`Blink OD.prst`, `its gp100.prst` (o preset da sessão 1! `ppName="It's GP100"`).

```
<GP-100>
  <preset_info software="1.2.0" firmware="2.1" product="GP-100" count="1"
               platform="WINDOWS" time="<ms epoch>"/>
  <presets ppBank="0" ppName="It's GP100" ppVolume="50" ppID="0" ppBPM="120"
           ppIRNum="27" ppType="4" ppTypeName="Rock" ppAuthor ppNotes>
    <Effect effectModuleName="RVB" effectName="Hall" effectState="1"
            effectCode="201326593" params_0..params_14 x= y=/>
    ... (9 slots: PRE DST AMP NR CAB EQ MOD DLY RVB)
    <ppCtrl c11 c12 c13 c21 c22 c23/>
    <ppEXP1 expTarget expVolume min max> <ppEXP1_N expMId expCode expIndex min max/>
```

Chaves: `effectCode = (nibble_do_modulo << 24) | indice_do_alg` — o byte
alto e o NIBBLE do modulo (PRE=1 DST=3 AMP=7 RVB=0C...), NAO o slot x/y
(cfr. parameters.json: Bog RedM = 0x0700006E, nibble 7, index 110; Hall =
0x0C000001, nibble 12, index 1). `params_N` = valores crus dos controles
(0-14, mesma ordem do dicionario); `ppIRNum` = IR selecionado. Compatível com o dicionário
`analysis/parameters.json` (códigos validados 0-conflito). O gp100-core pode
importar/exportar bibliotecas completas SEM tocar no hardware.

### 13.10 Scan de presets e paginação de estado (modelo final da sessão 1)

A enxurrada de 0–10s NÃO é edição — é o Suite baixando o estado dos presets:

```
OUT 13 01 00 00 <- [pp u16BE]          seleciona preset pp (2B)
IN  13 01 00 01 <- [pp] 0c 1c 01 40     meta do preset (6B)
OUT 13 01 00 02 <- [pp u16BE] 01       abre o bloco de paginas
IN  13 01 00 03 <- [pp] ? ? PG + 192B   pagina PG do preset pp (PG=0..8)
OUT 13 01 00 04 <- [pp u16BE] [PG u16BE] 01   avanca pagina (5B, ambos u16BE)
```

**Larguras exatas confirmadas pela validação do golden-file** (sessão 1,
198 presets × 9 páginas = 1.782 requests de página regenerados byte-a-byte
por `analysis/validate_golden.py`): quirk de boot = o preset atual (pp 0x0100
no exemplo) recebe select+open DUPLICADOS (re-leitura): 2 selects respondem
meta6 cada (e 1 resposta de tabela T1 chega TARDIA entre elas — backlog),
depois 2 opens respondem PÁGINA 0 duas vezes em `13010003` (não é um 3º
meta6); as 9 páginas seguem: req pg0..7 → pág1..8, pg8 → 4B em `13010005`.
Há 1 sonda do banco 2
(`13 02 00 00` pp 0000 + 9 páginas); a tabela de nomes `11000008` usa chave
`[banco u8][índice u8]` e o boot varre bancos 0x00–0x02 completos (16) +
banco 0x03 com 13 (61 leituras).
- 9 paginas por preset (0..8; a 8 vem truncada em 32B e a 9 é só o header 4B).
- O byte que eu achei que era "contador de versão" (d[3]) é o NUMERO DA PAGINA.
- **Zero SysEx durante a edição de knob na sessão 1**: knobs físicos não
  ecoam para o host (o Suite só reflete UI local). Edição via UI do Suite
  NÃO ocorreu na captura — a sessão 1 editou no hardware.Mapa de knob→offset: RESOLVIDO na sessão 3 — e o modelo era outro: a edição
pela UI usa writes semânticos diretos (ver §13.11), sem offsets de página.
As páginas 13 01 00 03 servem para LER o estado (e foram a base do pareio
por timeline da sessão 3).

#### Layout da página (DECIFRADO 08/10/2026 — #155)

O corpo, que chegava opaco, nibble-decodifica para **96 B** (pg 0..7) e
**14 B** (pg 8):

| | no fio | decodificado |
|---|---|---|
| pg 0..7 | 196 B = 4 B header + 192 B de nibbles | 4 B + 96 B |
| pg 8 | 32 B = 4 B header + 28 B de nibbles | 4 B + 14 B |

**Header (4 B, fora do nibble):** `[pp u16BE] [00] [PG]`. O `PG` é o
**número da página** (0..8) e **não a posição do pedido**: o `open`
`13 01 00 02` entrega a página 0, os reqs `13 01 00 04` com PG 0..7
entregam as páginas 1..8, e o req PG 8 devolve só o ACK de 4 B em
`13 01 00 05` — que **não é página**. Conta fechada: 1 open + 8 reqs = 9
páginas = 1.782 mensagens de `13 01 00 03` na captura.

**Corpo:** pares de nibbles expandidos (`codec::nibble_collapse`).

**pg 0 decodificada (offsets provados):**

| offset | o que |
|---|---|
| 0..2 | `pp` u16 LE |
| 2..14 | **nome ASCII[12]** com pad NUL |
| 14..32 | **cadeia** — 9 × u16 LE (posição → `x` do `<Effect>` no `.prst`) |
| 32..68 | **`effectCode`** — 9 × u32 LE |
| 68..96 | params ×7 (f32 LE) |

Os params são 135 f32 LE (9 slots × 15) em bloco contíguo atravessando as
páginas: pg0[68..96] + pg1..pg5 inteiros + pg6[0..32]. `effectState` mora
em pg6 @32 (9 × u16 LE).

**Provas** (gate `state_pages` — `analysis/validate_state_pages.py`,
artefato `analysis/state_pages_offsets.json`, embutido no binário):

| o quê | medido |
|---|---|
| cadeia (posição → `x`) | **198/198** — 40 pps têm a cadeia TROCADA no fio |
| `effectCode` | **1782/1782** |
| `effectState` | **1782/1782** |
| params | **26552/26554** (as 2 divergências são valores editados no hardware depois do dump) |
| nome | **198/198** + prova-negativa |

**Consumo:** `gp100_core::preset_pages::{decode, Paginas::{nome, slots}}` —
entram 9 `StatePage`, sai o domínio. O boot já baixa as 1.782 páginas e as
guarda em `Session::preset_state(pp)`; o actor serve o palco e os nomes a
partir delas, **nunca de um `select` novo** (ler outro pp não pode mudar o
preset que o pedal mostra).

### 13.11 Writes de edição pela UI — envelope semântico do knob (sessão 3)

Toda edição de knob pela UI do Suite emite um write SEMÂNTICO, independente
das páginas de estado 13xx (o device não ecoa nada — zero ACK, zero push de
página durante a edição; a UI mantém o estado local):

```
OUT  func 12   addr 10 [slot 1..9] 00 02    payload: 20B nibble-expandido
bytes reais (10B): [effectCode u32 LE 4B] [ctrl u8] [00] [float32 LE valor]
```

- `slot` = posição na cadeia (1=PRE 2=DST 3=AMP 4=NR 5=CAB 6=EQ 7=MOD 8=DLY
  9=RVB) — NÃO é o nibble do módulo;
- `effectCode` = idêntico ao .prst (nibble<<24 | index; Bog RedM=0x0700006E,
  Hall=0x0C000001) — no fio vai como **u32 LE**: Bog RedM = `6e 00 00 07`,
  Hall = `01 00 00 0c` (b0=LSB do índice, b3=nibble do módulo; confirmado
  na sessão 1: C-Wah PRE = `08 00 00 05` = 0x05000008);
- `ctrl` = índice do controle = posição `params_N` do .prst = `pos` do
  `controls[]` no parameters.json (AMP: 0=Gain 1=PRES 2=Master…; EQ:
  0..4 = 80Hz/240Hz/750Hz/2.2kHz/6.6kHz);
- valor em **unidade física** (float32 LE) com a mesma escala min/max do
  dicionário: knobs 0–99, EQ em dB [−50,+50], MOD Rate 0.1–10.0;
- expansão nibble = igual ao upload de IR (§13.7): cada byte real vira 2
  bytes no fio, hi-nibble primeiro;
- [5] = 0x00 nas 89 amostras (provavelmente flags/reservado).

Validado com a sessão 3: 89 writes, 14 grupos (slot,ctrl), 13/14 dentro do
range do dicionário. O 14º é o knob **Mic do CAB** (U-ban 4x12, ctrl 1):
existe no hardware e no .prst (`params_1=80`) mas NÃO virou `controls[]`
no algorithm.xml do Suite — controle oculto do dicionário. Corolário: ao
serializar presets, PRESERVAR todos `params_N` 0..14 mesmo sem nome.

Mapa completo (89 edits com timeline): `analysis/knob_map.json`.

**Corolário para o gp100-core:** para SETAR um parâmetro basta 1 SysEx
`F0 21 25 7F 47 50 2D 64 | 12 | 10 <slot> 00 02 | <20B nibble-expandido> |
F7` — não precisa abrir o bloco 13xx nem conhecer offsets de página. As
páginas `13 01 00 03` continuam necessárias para LER o estado completo
e para o ciclo de save (sessão 4, item 11 do BLOCKERS).

**Golden congela os templates de set_param POR INSTÂNCIA (achado M0.5,
29/09 — evidência: dump dos 40 templates do golden):** o endpoint
`12/10xx0002` tem 9 templates (t24, t32–t39), um por knob capturado, e o
`request_payload` de cada um carrega as CONSTS do knob específico (ex.:
t32 = `mixed(c15+v5)` com const `060e0000…` = Bog RedM LE). Consequências:
(1) um knob NOVO não casa em nenhum template — quem valida o SHAPE
`[code u32 LE][ctrl][00][f32 LE]` é o CODEC (`set_param_parse`, provado
byte a byte nos 92 knobs da P4); o golden descreve as INSTÂNCIAS
observadas, não a gramática do endereço; (2) o MockDevice (M0.5) valida
set_param pelo codec, e só por templates para os demais endereços.

### 13.12 Registro de objetos 11xx/12xx + fluxo de save (sessões 1–4)

Além do bloco 13xx (estado de preset) e do 10xx0002 (set de parâmetro), o
Suite usa endereços de "registro de objetos":

**Estrutura do .prst (achado M0.5, 29/09 — evidência: regex sobre
all.prst + parse do preset.rs):** o container `<ppIRInfo>` com as 20
tags `<ppIRInfo0..19>` (cada uma com ppIRNum/ppIRName/ppIRCRC) é filho
DA RAIZ `<GP>` — irmão de `<preset_info>` e dos `<preset>`, NÃO está
dentro de preset_info nem de um preset. Além disso, `ppIRNum` é o
ÍNDICE GLOBAL do IR no device (ex.: 168820736+), NÃO o slot 0..19 —
quem define o slot é a posição da tag (`ppIRInfo0` = slot 0). No
gp100-core, `MockState::load` lê os CRCs de fábrica por posição da
tag (código em `transport/mock.rs`).

**Leituras de boot (t<15s em TODAS as sessões):**
```
OUT 11 12001002 [pág]      → IN 12 75B nibble-exp., d[0]=pág (0..0x13);
                              **tabela dos 20 User IRs** (não é de tipos!)
                              — layout decifrado: [0]=slot cru; [1..74]
                              nibble-exp = 37B reais = nome 32B + tag 5B
OUT 11 12001012 [i]        → IN 12 44B nibble-exp. `12 10 2c 00 [i] 00 01 …`
                              (5 entradas, provavelmente setlist/loja)
OUT 11 11000008 [hi][lo] 00 00   (61 leituras, chave de 2B)
                             → IN 12 14B: [0..3]=chave ecoada, [4..13]=nome
                               ASCII do slot. 0x0000..0x000F = slots de
                               fábrica ("Metal", "Indie", "Rock", "Funk",
                               "Blues", "Jazz", "Bass"…), 0x0100+ = zeros
```

**Tabela de User IRs — layout DECIFRADO (28/09, cross-ref com .prst):**

A leitura `12001002` NÃO é tabela de tipos: é o registro dos **20 slots de
User IR**. Prova: após importar "test_ir_mono"/"test_ir_stereo" (S2), as
páginas 0/1 passaram a conter esses nomes em ASCII (S3/S4); slots nunca
usados estão preenchidos com `0xFF`.

```
payload 75B = [slot u8 cru] + 74B nibble-expandidos -> 37B reais:
[0..31]   nome ASCII null-padded (0xFF*32 = slot vazio)
[32]      desconhecido: 0x00 se vazio; 0xE4/0xC9 nos ocupados (amostra única)
[33..36]  CRC32 IEEE BE do slot — nos VAZIOS casa EXATAMENTE com o ppIRCRC
          do .prst (pag0 vazio = 0x1D38E987 = "User IR 1"; pag2 = 0xDCCA2776
          = "User IR 3") ⇒ cada slot tem CRC de fábrica registrado no XML;
          nos OCUPADOS não casa com crc32(blob/wav/data) em nenhuma variante
          testada (provável CRC do setor de flash com padding) — A DECIFRAR
[36]      trailer: 0x76 vazio / 0xD4,0xB4 ocupados (provável byte de check;
          1 amostra)
```

Corolários: (1) o `ppIRCRC` do `.prst` é o CRC32 IEEE do registro do slot —
compatibilidade .prst↔device direta; (2) para listar IRs do device basta
ler as 20 páginas e decodificar nome+CRC; (3) o flag byte[32] e o CRC de
slot ocupado ficam como pendência menor (não bloqueia: nome já basta p/ UI).

**Status e ops globais:**
```
IN  12 12000001  `01 00 00`   status "pronto" — fecha o BOOT e bursts de
                              sincronização (CORREÇÃO 29/09: NÃO é resposta
                              de ciclo de op/save — na S2 chegou 22s ANTES das
                              ops do save; ver ciclo re-derivado abaixo)
OUT 12 00020000  8B, bytes [4..5] = nº da op (u16 BE):
   op 1 = entrar/em modo edição (sessões 1, 2 e 4)
   op 2 = 2ª etapa da abertura (só sessão 1)
   op 0 = sair/commit (visto ao fechar editor e no fluxo de save)
OUT 12 00020001  `00 00 00 00`  (keepalive/ping no boot)
```

**Bloco de metadados do preset atual (write; campos decifrados na validação do
golden-file com 2 amostras: 'It's GP100' pp=0/type=4 e 'Blink OD' pp=1/type=6):**
```
OUT 12 11000000  20B: [0..3] zeros + [4..5] pp ID u16 BE + [6..7] zeros
                      + nome ASCII [8..17] (12 chars, null-padded)
OUT 12 11000004  20B: zeros (autor/notas?)
OUT 12 11000005  04B: [0..1] ppType u16 BE (4=Rock, 6=Pop) + [2..3] zeros
OUT 12 11000007  50B: zeros (reservado)
OUT 12 12000002  08B: [0..3] zeros + [4..5] pp ID u16 BE + [6..7] zeros
```
Ambos pp e ppType vêm do .prst (ppID/ppType) — o fluxo de save completo é
generável a partir do arquivo + regras (77/77 byte-a-byte, prova D do
validate_golden.py).
Ocorreu ANTES da edição na sessão 1 (54.7s, ao abrir o editor) e no fluxo
de save da sessão 4 — o Suite grava os metadados do preset atual ao abrir
para editar (e aparentemente de novo ao salvar).

**Ciclo do SAVE — RE-DERIVADO do log cru (29/09, `derive_save_ops.py`; corrige
a leitura anterior, que atribuía o burst 11xx ao save por artefato de janela):**
```
S4 (save "It's GP100", ts 30241.9s+):
1663.64s  bloco de metadados 11xx (acima)
1663.66s  OUT 00020000 op 0  (x2: 2ª em +578ms)
1664.25s  OUT 00020000 op 1  (x2)   ← saiu e reentrou no modo edição
          → FIM. ZERO msgs IN até o fechamento do Suite (+11s)

S2 (fluxo separado — mesma análise):
17942.0s  burst IN 11000008 ×61 + 12000001  ← 61 requests OUT 11000008 nos
          instantes anteriores (é o SUITE lendo/sincronizando a tabela; NÃO
          tem relação com o save que vem a seguir)
17964.3s  OUT 00020000 op 0  (x2, 16ms de gap)  ← o "save" da S2 é SÓ ISTO
          (zero IN; nenhum 11xx por ±118s — meta já havia sido escrita ao
          abrir o editor, como na S1)
18082.6s  OUT 11xx (metadados "Blink OD" — 118s DEPOIS das ops; marca outra
          abertura/troca de editor, não o mesmo evento de save)
```
**Corolários (base do D3 do ADR-6):** (1) o save NÃO gera resposta IN — é
fire-and-forget em ambas as amostras; (2) o burst `11000008`×N + `12000001`
é evento de SINCRONIZAÇÃO de tabela do próprio Suite/app: na S2 precedido de
requests OUT (respostas do pairio), na S4 a cópia pós-save (+11,4s) veio sem
NENHUM request (janela com 0 OUT) — push espontâneo do device/app; (3) a
sequência de ops não é única (S4: 0,0→1,1; S2: só 0,0) — a FSM usa a da S4
(única com meta+ops no mesmo evento) e o H1 revalida; (4) a cópia pós-save
da S4 mostra o device repassando a tabela de slots user sozinho — o IN
verificável "salvou?" será preciso vir de `list_user_irs`/display (H2);
(5) AMBAS as sessões ENCERRAM com burst IN espontâneo (S2: 32 ACKs
`12001002` idx `0x208..0x226`; S4: `11000008`+`12000001`) — quirk FECHADO
no §13.7: flush de fim de sessão do ring do proxy (hipótese principal).

**O que NÃO apareceu no save (importante):** nenhum download/write 13xx,
nenhum endereço de destino de slot explícito, nenhuma confirmação de
commit em flash, e NENHUM readback do estado. O save da UI aparentemente
se apoia nos writes semânticos 10xx0002 (§13.11) já feitos durante a
edição + metadados 11xx; a persistência física é interna ao device.
**VERIFICADO EM CAMPO (28/09/2026):** o usuário confirmou no display da
pedaleira que o slot de destino recebeu o preset com os valores editados
da sessão 3 (AMP Gain ~99) — ou seja, o save persiste o **ESTADO AO VIVO**
(writes 10xx0002 acumulados), não uma cópia do preset original, e o fluxo
acima é o caminho real de commit. Item 11 do BLOCKERS: FECHADO.

---

### 13.14 Baseline v1.1 — o que o golden NÃO sabe (e por que)

> Escrito em 04/10 (#23, gate H3). A baseline é `docs/protocol_golden.json`;
> a versão, o hash, o motivo e o histórico vivem em **`analysis/baseline.json`**
> (`uv run python analysis/baseline.py show`). Até a v1.0 o hash era um literal
> dentro de um teste, sem versão e sem motivo: colar o hash novo passava.

**O achado.** O `build_golden.py` cortava cada frame no **primeiro** par de
nibbles `f7` (`hx.find("f7")`) em vez do `F7` **final** do SysEx. Nomes de preset
em ASCII quase sempre têm esse par no meio — "World" = `576f726c64` contém `f7`
em `6f72`, e `o` seguido de `p`–`z` é o caso comum. **16 frames completos** das
4 capturas eram truncados assim, e o nome "Acoustic" virava 5 bytes de 11.

**O erro maior.** As capturas têm **100 linhas sem `F7` final**: o proxy morreu
no flush e deixou buffers de 256 bytes preenchidos com `ff`. Reconstruir o fim
delas com `rfind("f7")` produz frames de 196 bytes que **nunca existiram no
fio**. A regra agora é o contrário: **linha sem `F7` final não é frame** — é
saída truncada, e o certo é contá-la e seguir (R1: protocolo adivinhado não
entra). O golden declara os descartes em `_meta.excluded`.

**O que a v1.0 afirmava sem evidência (e a v1.1 não afirma mais):**

| Endereço | O que a v1.0 dizia | Por que caiu |
|---|---|---|
| `13000000` | `push` com `const` de 242 bytes | É o **prefixo** de uma linha cortada (o proxy corta em 256 bytes). O único template dele vinha de dado corrompido, então o template **saiu** (40 → 39 templates). |
| `12001002` resync | `push` com `by_len` {4B, 75B}, 192 instâncias | As 32 instâncias de 4 bytes eram **fabricadas**. As 592 instâncias REAIS de 4 bytes são os ACKs do upload de IR (§13.7) e estão no template `req`. |
| `11000008` resync | 290 push | 290 = 244 frames completos + 46 dos fabricarados. Contagem correta: **244**. |

**Consequência no save (§13.12).** A prova D era `77/77`. **63 dessas 77
mensagens eram fabricaradas** — o `find("f7")` cortava dentro do buffer `ff` e
o `body()` devolvia `010e000000000000000000000000`, 14 bytes que são exatamente
a forma de um registro de usuário, e passava na regra. Hoje a prova D é **14/14**:
o lado **OUT** (9 frames do S4 + 5 do S2, derivados do `.prst`), que é
byte-a-byte e continua provado. O lado **IN** (resync `11000008`, resync
`12001002`, status `12000001`) é **gap de captura**.

> **Isto não volta por ajuste de código.** Volta com captura nova (R2/R3). O
> `docs/H3_CHECKLIST.md` tem o roteiro, e o `--log` do `gp100-cli` agora grava
> `t` (ms desde a abertura) justamente para que essa captura possa entrar no
> pipeline: sem relógio não há segmentação por fase nem casamento OUT→IN.

**Os três gaps que a sessão de campo fecha:**

1. **Resposta completa ao `13000000`** — o dump de boot. É o maior frame do
   protocolo e nunca foi capturado inteiro.
2. **Resync do save** — os ACKs e o status pós-save, que a v1.0 "provou" com
   bytes inventados.
3. **Regeneração da baseline a partir do gp100-core** — o DoD literal do H3, que
   antes era impossível: o `build_golden` carregava **0 eventos** de um log do
   core (só aceitava `out_long`/`in_long` + `hex`). `analysis/wirelog.py`
   normaliza os dois formatos, e `analysis/validate_core_capture.py` julga a
   sessão frame a frame.
