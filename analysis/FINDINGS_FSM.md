# FINDINGS — FSM handlers + opcode extraction (RE estática, sessão 2)

Scripts: `disasm_fsm.py`, `disasm_senders.py`, `opcode_extract.py`
Método: linear sweep capstone sobre .text (185.418 insns), byte-pattern scan de
imports winmm (IAT), xref engine própria (E8/E9 rel32 + thunks E9 + vtables).

## 1. Handlers pedidos

| VA | Veredito |
|---|---|
| `0x621020` | trivial: `*estado = 3` (set-state) |
| `0x621030` | `ret 0` (no-op / estado não suportado) |
| `0x621040` | **scanner de memória da FSM** (função-membro `search`): procura sequência de u32 no array `[obj+0x2DEC..]`, retorna índice/match e escreve resultados em `[obj+0x2E20..0x2E38]`; no fim chama `realloc`-like (`0x757D3D`) |
| tabela `0x621210` | **não é jump-table de estado** — é *thrunk* MSVC (E9...) de ajuste de `this` para múltipla herança |

**Correção do modelo mental**: a FSM de sessão do GP-100 NÃO é um switch
de comandos como o PC_BT_Comm do GP-50. É um **pipeline orientado a objetos**:
input callback → demux por bytes de status → parser (BitReader) → objetos de
sessão → e um **codificador de escrita separado** (abaixo).

## 2. Cadeia MIDI real (confirmada por disassembly)

- Entrada: `midiInOpen(..., callback=0x4AC660, CALLBACK_FUNCTION=0x30000)` —
  callback do JUCE `Win32MidiIn`; demux por `wMsg` 0x3C3 (MIM_DATA) / 0x3C4
  (MIM_LONGDATA) → listeners.
- Saída: **UMA vtable de MidiOutput** (`0x1A4399C`, 4 slots, construída em
  `0x4ACD40`): slot3 = `0x4AD1D0` = `sendMessageNow(msg)`:
  - len ≤ 3 e `msg[0] != 0xF0` → `midiOutShortMsg` (retry ×0x32)
  - senão → `midiOutLongMsg` (buffer MIDIHDR de 0x40, retry ×0x1F3)
- **Zero chamadas diretas** a 0x4AD1D0: todos os envios passam pela vtable
  (polimórfico JUCE). Isso confirma que a captura MIDI spy capta exatamente
  o mesmo fluxo que o driver de verdade.

## 3. Descoberta principal: os comandos usam codificação de comprimento variável

Funções do módulo de protocolo (todas em `0x620000–0x634000`; 129 dos 135 xrefs
do BitWriter):

| VA | Função | Semântica |
|---|---|---|
| `0x4B48B0` | `BitStream::writeBits(value, n)` | primitiva (JUCE) |
| `0x6331D0` | `writeVarLen(u64, nbits)` | **codificação MIDI VLQ**: <0x80 → 1 byte; <0x800 → `0xC0 \| (v>>6)`, `0x80\|(v&0x3F)`; <0x10000 → `0xE0\|…`; continua com bytes de continuação `0x80\|…` |
| `0x633520` | `writeVarLen(u32)` (32-bit) | mesma família: <0x80→1B; <0x800→2B (`0x3000\|v>>6` or-ed); <0x10000→3B (`0xE000\|…`) — **é um varint MIDI-style, não nibble-split** |

Ou seja: os bytes de comando/payload do GP-100 são **varints com bytes de
continuação 0x80-0xFF** (formato MIDI running-length), em vez dos pares de
nibbles do GP-50. Isso explica por que não encontramos `0x11 0x4F` nem no exe
nem como constantes isoladas: **os "opcodes" são valores codificados em VLQ**,
gerados em runtime a partir de enums (por isso o scan de imediatos só achou
0x06/0x0E/0x24/0x40 — que são **campos de cabeçalho**, não comandos).

## 4. Constantes confirmadas no builder de objetos

Em `0x6209D0` (dispatch de 7 formatos de objeto):
- **0x06/0x0E/0x0F** → IDs de tipo (4 bits) — coerentes com a tabela de tags.
- **0x24** → tipo de objeto "IR blob" com sample rate codificado logo em
  seguida (8000→4, 16000→5, 22050→6, 44100→7, 24000→8, 32000→9?; valores
  vistos: 0x1F40, 0x3E80, 0x5622, 0xAC44, 0x5DC0, 0xBB80 esperado).
- **0x40** → writeBuffer de N bytes com ponteiro/tamanho de struct
  (`push [obj+0x34]; mov edx,0x40; push [obj+0x30]; call writeBuffer`) —
  provável "hardware param block".
- Cabeçalho de objeto: `writeBits(1, 1)` flag + `writeBits(id, 4)` +
  `writeBits(len?, 3|5|7)` + payload via `0x633520/0x6331D0`.

## 5. O que isso muda na estratégia

1. **O codec GP-100 é reconstruível por RE estática**: as funções geradoras
   (writeVarLen, writeObject, writeHeader) foram todas localizadas e são
   pequenas e legíveis. Não é preciso inferir wire-format por tentativa e erro.
2. O "0x1D/0x92" do GP-50 não existem no GP-100 — os comandos são objetos
   VLQ-tagged. A captura real ainda é o caminho mais rápido para a **lista de
   enums de comando** (qual tag = "ler preset", qual = "escrever"), mas agora
   é uma questão de enumeração, não de arquitetura.
3. Próximo alvo natural: as funções que chamam `0x633520/0x6331D0` dentro do
   módulo (todas em 0x633xxx-0x634xxx) — elas são os **builders de request**;
   enumerar as constantes de tag que passam por elas dá a tabela de comandos.

## 6. Tabela de funções novas (para o codec Rust)

| VA | Papel no codec |
|---|---|
| `0x4B48B0` | `write_bits(u32 v, int n)` |
| `0x4B4970` | `write_bytes(ptr, n)` |
| `0x6331D0` | `write_vlq64(u64)` |
| `0x633520` | `write_vlq32(u32)` |
| `0x6209D0` | `parse_object(BitReader&, Session&)` — 7 formatos |
| `0x620570` | `data_type_from_tag(u32) -> 4-bit id` |
| `0x620920` | `crc8_07(buf, len)` |
| `0x621040` | `Session::search_sequence(u32* arr, n)` |
| `0x4AD1D0` | `MidiOut::sendMessageNow(JuceMidiMessage&)` |
| `0x4AC660` | winmm input callback → listeners |
