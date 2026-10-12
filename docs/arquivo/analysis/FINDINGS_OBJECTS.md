# FINDINGS — Parser/serializer `0x6209D0`: os 7 formatos de objeto

Sessão 3 de disassembly. Função: `BOOL encode_object(BitStream* esi, Object* edi)`
— dispatch por `obj->u32@0x00` (0..6) via jump-table `0x621000`.
Helpers (assinaturas resolvidas):
- `0x4B48B0 BitStream::writeBits(u32 value, int nbits)` — this=stream, edx=value, push=nbits
- `0x4B49B0 BitStream::writeU32(u32 v)` = 4×writeBits(8) little-endian (LSB first)
- `0x4B4A20 BitStream::writeBytes(ptr, len)`
- `0x4B4970 BitStream::writeBytes(edx=ptr, push=len)` (fastcall variante)
- `0x4B4800 BitStream::skipOrWrite(nbits)` — reservas de campo com valor fixo/zero-fill
- Cauda comum: sucesso = `mov eax,1; ret`; qualquer falha → `0x620FF5` (`xor eax,eax; ret`)
- Join `0x620FBE`: `writeBytes(edx, push)` genérico usado por casos 1/2/6/7.
- Saída default (case >6): `0x620FD8` = `writeBytes(obj->0x10, obj->u32@0x08)` e retorna `len != 0`.

Struct base (todas as variantes): `u32 type@0x00`.

## Tipo 0 — "RAW_BLOB" (`0x620A42`, também caída do case 3 vazio)
```
[0x00] u32 type = 0
[0x08] u32 size
[0x10] u8 data[size]
wire: writeBits(0x20 - (size&0x1F) ajuste , n)  // pseudo: preâmbulo de 0x20 bits (ver 0x620A12)
      writeBytes(data, size - 4)
```
Cálculo exato no prologue: `eax = 0x20; eax -= [obj+0x10]; edx = [obj+8] + eax` —
o tamanho no fio é `size + (0x20 - [obj+0x10])`, com `[obj+0x10]` = offset de
retrocesso (back-pad). Em seguida `writeBits(size-4, n)` + payload a partir de
`[obj+0x14]`.

## Tipo 1 — "U32_LENGTH" (`0x620B0C`)
```
[0x00] u32 type = 1
[0x08] u32 len
wire: writeBits(len << 3, n)      // comprimento em BITS (len*8)
```

## Tipo 2 — "HEADERED_BLOB" (`0x620B1E`) — HEADER + COUNT + DATA
```
[0x00] u32 type = 2
[0x08] u32 size          // tamanho total
[0x10] u8  header[4]     // 4 bytes fixos de cabeçalho
[0x14] u8  data[size-4]
wire: writeBytes(header, 4); writeBytes(data, size-4)
```

## Tipo 3 — "PATTERN_TABLE" (`0x620B44`) — array de 24-byte records
```
[0x00] u32 type = 3
[0x10] u32 count
[0x14] records: cada 0x18 bytes:
        +0x00 u64 v0  → writeBits(lo,0x20); writeBits(hi,0x20)   (2×32 bits)
        +0x08 u64 v1  → idem
        +0x10 u32 v2  → writeBits(v2, 0x10)                      (16 bits)
wire: repetido count vezes.
```
Interpretação provável: tabela de eventos/pontos (2×64-bit + 16-bit flags) —
candidate a "drum pattern" ou "looper cue list".

## Tipo 4 — "STRING_TABLE" (`0x620C02`) — pares (u32 len, bytes) com cookie
```
[0x00] u32 type = 4
[0x18] u32 count
[0x1c] entries: { u8 len; u8 bytes[len]; }  (len em cada entrada)
wire: writeU32(0x20);                      // cookie/versão fixo
      writeBytes(&const@0x1994348, 0x20);  // 32B: "e sheet track index offset must "
      writeU32(count);
      para cada entrada:
        writeU32(entry.len);               // comprimento como U32 LE
        writeBytes(entry.bytes, len);
```
A constante de 32B é uma **fragmento de string de assert** ("...e sheet track
index offset must ") — confirmou que estes objetos carregam dados de **letra/
texto de música** (letras/“sheet”), não de parâmetros de DSP.

## Tipo 5 — "IR/HW_PARAMS" (`0x620C93`) — o mais complexo (553 bytes de código)
```
[0x000] u32 type = 5
[0x010] u8  hdr[0x80]                    → writeBytes(hdr, 0x80)
[0x098] u32 blob_ptr; [0x9c] u32 blob_len → writeBytes(blob_ptr, blob_len)
[0x0A0] u32 has_extra                    → writeBits(has_extra?1:0, 1)
[0x0A4] u32 n_sessions                   → writeBits(0x817, nbits fixa); writeU32(n_sessions)
[0x0A8] sessions[n_sessions], stride 0x28:
   +0x00 u64 ts        → writeBits(lo,0x20); writeBits(hi,0x20)      // timestamp
   +0x08 u8  b8        → writeBits(b8, 8)
   +0x09 u8  path[12]  → writeBytes(path, 0xC)
   +0x18 u32 flags     → writeBits(flags&1,1); writeBits((flags>>1)&1,1)
   constante: writeBits(0x6E, nbits)                                 // 110 = subtag
   +0x1C u8  n_items   → writeBits(n_items, 8)
   items[n_items], stride 0x10:
      +0x00 u64 v0     → writeBits(lo,0x20); writeBits(hi,0x20)
      +0x08 u8  b8     → writeBits(b8, 8)
      constante: writeBits(0x18, nbits)                              // 24 = subtag
```
Este é o objeto de **importação de IR/SnapTone**: header 0x80 (wav/ir metadata),
blob (dados), e uma lista de sessões de "capture" com timestamps e subtags
fixas 0x6E (110) / 0x18 (24).

## Tipo 6 — "DUAL_STRING_U64" (`0x620EC6`)
```
[0x00] u32 type = 6
[0x10] u32 a                 → writeBits(a, 0x20)
[0x14] char* str1            → writeBits(len1,0x20); writeBytes(str1,len1)   // strlen medido inline
[0x18] char* str2            → writeBits(len2,0x20); writeBytes(str2,len2)
[0x1c] u64 v                 → writeBits(lo,0x20); writeBits(hi,0x20)
[0x20] u64 w                 → idem
[0x24] u64 x                 → idem
```
Formato de metadados textuais + 3×u64 (ex.: título, artista, ids).

## Constantes mágicas
| Valor | Onde | Significado |
|---|---|---|
| `0x817` | caso 5, após flag | assinatura de sessão IR (2071) |
| `0x6E` (110) | caso 5, item | subtag fixa de capture |
| `0x18` (24) | caso 5, sub-item | subtag fixa de item |
| `"e sheet track index offset must "` | caso 4 | fragmento de assert de letras/sheet |

## Interpretação de alto nível (corrigindo §9/§10)
Este serializer **não** é o parser de respostas do device: é o **empacotador
desktop→destino** de objetos de uma "sessão de música" (letras/sheet + IRs +
padrões + metadados). A presença do assert de "sheet track index" e das strings
duplas (tipo 6) sugere exportação de projetos (letras + capturas + backings).
Os 7 formatos são, portanto, **formatos de objeto de sessão/export**, não de
comando de edição — a camada de comando permanece nos builders VLQ
(`0x6331D0/0x633520` e chamadores em `0x633xxx-0x634xxx`).

## Implicações para o gp100-core
- `model/session.rs`: 7 enums com os layouts acima (direto para serde).
- Estes objetos **não** são o protocolo de edição de presets; servem ao
  ecossistema de projeto/export do app (provavelmente o formato de "song session"
  usado pela Suite para sincronizar letras/IRs/backing com o device).
- Próximo alvo para comandos de preset: continuam sendo os builders VLQ.
