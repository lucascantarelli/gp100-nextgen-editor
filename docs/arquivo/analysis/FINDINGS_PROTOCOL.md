# FINDINGS — Descompilação GP-100.exe (protocolo)

> ✅ Status: achados corroborados em campo — o que era hipótese aqui virou
> confirmação no `docs/PROTOCOL.md` §13 (fio) e `docs/protocol_golden.json`.
> Mapa geral: `docs/INDEX.md`.

Data: 2026-09-28 · Método: pefile + capstone (x86/32), venv uv da raiz (`.venv/`) · (histórico: feito com venv em `analysis/.venv`, desde então consolidado)
Scripts: `rtti_disasm.py`, `disasm_deep.py`, `xrefs_opcodes.py`

## 1. Funções-chave identificadas (VA, image base 0x400000)

| VA | Papel |
|---|---|
| `0x4B48B0` | `BitStream::writeBits(value, nbits)` — bit-writer central (135 xrefs) |
| `0x4B4970` | `BitStream::writeBuffer(ptr, nbytes)` (2 args: edx=ptr, push=contagem) |
| `0x4B4800` | segunda primitiva do stream (value-only) |
| `0x620920` | `crc8_07(buf, len)` sobre buffer do dispositivo (tabela em `0x1A21F40`) |
| `0x6209D0` | **Serializer principal** do parser (34 writeBits), com jump-table de 7 casos |
| `0x623AA0` | pós-processamento do pacote (chamado após o serializer) |
| `0x626xxx` | mega-função de sessão FSM (estado ~0x2E38 bytes) |

## 2. FRAMING — **CORREÇÃO vs GP-50**

O exe **não** usa nibble-split: usa `BitStream` de **campos de bits**
(escreve 1 bit + 7 bits + 4 bits + payloads de N bits, `0x4B4970` para blobs).
Também **não** há `F0..F7`: o fluxo é USB-MIDI bruto (a função de sessão grava
`"FlaG"` (0x616C4 67, little-endian de `FlaG`?) — na verdade literal `0x664C6143`
= "FaFl"... ver §5) e o **sincronizador é um flip-flop de 2 estados**
(`[obj+0x1AE0] = 0x22` e depois `0xFFFFFF` — delimitadores de sessão).

Implicação: os pacotes do GP-100 **não são byte-idênticos** aos do GP-50 no
nível de wire. A camada lógica (comandos/objetos) ainda está por mapear.

## 3. Tabela de tipos de dados (enum → ID)

Em `0x620570` (função `dataTypeFromTag`), um `tag` de 32 bits do cabeçalho do
objeto é mapeado para IDs pequenos (4 bits no fio):

| Tag (u32) | ID |
|---|---|
| 0x100 | 6 (default ≤0x100) |
| 0x0C0 | 1 |
| 0x100 | 6 |
| 0x240 | 2 |
| 0x400 | 0xA |
| 0x480 | 3 |
| 0x800 | 0xB |
| 0x900 | 4 |
| 0x1000 | 0xC |
| 0x1200 | 5 |
| 0x2000 | 0xD |
| 0x4000 | 0xE |
| 0x8000 | 0xF |

Depois: `writeBits(id, 4)`; em seguida **sample rate** codificado
(0x1F40=8000→4, 0x3E80=16000→5, 0x5622=22050→6, 0xAC44=44100→7, provável 0xBB80=48000→8)
— **formato de transferência de IR/áudio**.

## 4. Constantes avulsas confirmadas em código

- `0x817` — constante escrita via `0x4B4800` (provável magic/comando de sessão).
- `0x24` — campo `writeBits(0x24, nbits)` no caso 5 (bloco IR?) antes de um
  `writeBuffer` de `len` bytes + 16B (`0x10`) de rodapé.
- O serializer `0x6209D0` tem 7 formatos de objeto (jump-table `0x621000`),
  tamanhos 0x10/0x18/0x20 por campo — consistente com TLV binário, não XML.

## 5. Observações sobre o literal "Flag"

Em `0x626D24`: `mov edx, 0x664C6143` = bytes `43 61 4C 66` = "CaLf"... na ordem
little-endian da string é "FalC"/"Flag" dependendo da leitura; é gravado num
buffer de 32 bytes (`push 0x20` em `0x4B4970`). É magic/assinatura de sessão.

## 6. Conclusão para o projeto

1. O GP-100 (geração i.MX RT) **não compartilha o wire-format** do GP-50
   (MVsilicon B1): confirmado por disassembly (bit-stream + tags u32 ≠ nibble+F0F7).
2. O protocolo do GP-100 é um **formato TLV binário** com enum de tipos de dados
   (tabela §3) e CRC-8/0x07 no mesmo polinômio da família.
3. Próximos passos para mapear comandos host→device:
   - Rastrear os **estados** da FSM (função 0x626xxx; campo `[obj+0]` = estado
     com jump-table `0x621210` e handlers `0x621040/0x621030/0x621020`).
   - Localizar as funções que montam requests de leitura (calls a `0x4B48B0`
     seguidos de `midiOutLongMsg`/`MidiOutput`).
   - Captura MIDI spy continua sendo o caminho mais rápido para os opcodes de
     alto nível; o disassembly provou que a engenharia é possível, mas custosa.
