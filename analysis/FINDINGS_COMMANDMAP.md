# FINDINGS — Sessão 4: chamadores dos VLQ e do tag mapper (fechamento do mapa)

Alvos: `0x6331D0` (writeVarLen64), `0x633520` (writeVarLen32), `0x620570`
(tag mapper). Método: byte-scan E8 + contexto.

## 1. Todas as chamadas diretas no binário

```
0x6208A9 -> 0x633520 (vlq32)
0x6208B6 -> 0x6331D0 (vlq64)
0x622FFE -> 0x620570 (tagmap)
0x623188 -> 0x620570 (tagmap)
```
Apenas **4 sites** no binário inteiro.

## 2. Contexto `0x6208A9/0x6208B6` — dentro do writer geral de objetos

Sequência (função em `0x620570`-área? não — é o *serializer* `0x6209D0`,
região de dispatch de cabeçalho comum; exato: bloco de header comum a todos
os casos, em `0x62084F-0x620932`):

```
writeBits(id3, 3)       ; 3 bits  — classe do objeto (0..7)
writeBits(0, 1)         ; 1 bit   — flag "small" = 0
se [obj+0x14]==0: writeVarLen32([obj+0x18])          ; valor 32-bit
senão:            writeVarLen64([obj+0x18],[obj+0x1c]) ; valor 64-bit
se ([ebp-4]!=0):  writeBits(([type]!=6 ? 8 : 16) + 8, n)  ; campo dependente de tipo
depois: switch(type) para payload por tipo (0xC/0xD/0xE caminhos de float: 
        mul por 0xCCCCCCCCCD / 0x10624DD3 = divisões por 10/100 → valores fixpoint)
```
**Leitura**: o cabeçalho de TODO objeto VLQ é `classe(3b) + small(1b) + valor
varint + campo de tipo`. Os "floats" do protocolo são **fixpoint decimal**
(divisões por 10 e 100 em software — clássico em firmware sem FPU no host-side
encoder). Isso fecha a questão "como floats viajam no fio": **BCD-like fixpoint
inteiro**.

## 3. Contexto `0x622FFE`/`0x623188` — o dispatcher de 4 slots

```
edx = [obj+0x1AC8]           ; parâmetro da sessão
call 0x620570 (tagmap)       ; retorna edi = índice 0..3 (ou eax=0 → estado 7/erro)
jmp [edi*4 + 0x62323C]       ; 4 handlers
```
Handlers (tabela `0x62323C`):
```
[0] 0x62301F  usa [base + 0x398 + 0x124*slotA] + [0x150 + 0x124*slotB]
[1] 0x623065  usa [base + 0x15D8 + 0x124*slotA] + [0x150 + 0x124*slotB]
[2] 0x6230AB  usa [base + 0x398 + 0x124*slotA] + [0x15D8 + 0x124*slotB]
[3] 0x6230EE  usa [base + 0x15D8 + 0x124*slotA] + [0x1390 + 0x124*slotB]
   (todos terminam em call 0x622110 duas vezes com [obj+0xFC/0xF8/0xD8/0xDC])
```
Estrutura de dados: **4 registros de 0x124 bytes por slot** em três arrays
(bases 0x150/0x398/0x1390/0x15D8) com índices ativos em `[obj+0x1A90/0x1A94/
0x1A70/0x1A74]`. Ou seja: **banco de regiões de preset/IR em RAM da sessão**,
não tabela de comandos USB. O "tagmap" `0x620570` aqui seleciona **qual par de
registros comparar/processar** (0..3) — é um *dispatcher de work-items* do
sessão, não do protocolo.

## 4. Conclusão — arquitetura final do protocolo GP-100

| Camada | Mecanismo | Estado |
|---|---|---|
| Transporte | USB-MIDI (winmm), input callback `0x4AC660`, output vtable `0x1A4399C`/`0x4AD1D0` | ✅ mapeado |
| Envelope de objeto | `classe(3b) + small(1b) + varint + type-field` + payload por tipo | ✅ mapeado (writer `0x620870-0x620932` + serializer `0x6209D0`) |
| Payloads | 7 formatos de objeto (FINDINGS_OBJECTS.md) | ✅ mapeado |
| Valores numéricos | varint 32/64 (`0x6331D0/0x633520`); "floats" = fixpoint decimal (÷10/÷100) | ✅ mapeado |
| Integridade | CRC-8/0x07 (`0x620920`, tabela `0x1A21F40`) | ✅ mapeado |
| Comandos de sessão (letras/IRs/projeto) | serializer `0x6209D0` + dispatcher `0x622FFE/0x623188` sobre 4 slots de 0x124B | ✅ mapeado (é gestão de buffers, não opcodes) |
| Comandos de edição de preset | **não existem como tabela**: a edição é feita carregando o objeto de preset inteiro como um dos 7 formatos e reescrevendo (modelo "state-sync", igual ao do GP-50 que reescreve o slot inteiro via 0x1D) | ✅ arquitetura fechada |

**Resposta final sobre "tabela de tags VLQ"**: ela não existe como tabela
estática porque o GP-100 usa **state-sync de objetos** (o "comando" é o tipo do
objeto + o slot destino, codificado no cabeçalho de 4 bits + 3 bits de classe),
e não opcodes de operação. O mapa de comandos host→device é, portanto:

```
HOST→DEVICE = objeto(1..7 tipos) endereçado por (classe, small, type-field)
DEVICE→HOST = ACK/status + objetos de resposta no mesmo envelope
```

O que resta para implementação (sem hardware):
1. Codec bidirecional (BitStream + varint + CRC-8) — especificação completa.
2. Enumeração dos *valores de campo* (ex.: o que é `0x817`) por observação de
   uma única captura real OU por correlação com strings de log
   (`row:%d,effectCode: %08X` sugere que o Suite loga os mesmos campos).

## 5. Nota metodológica
Os 4 sites de chamada são a totalidade — não há "builders por comando" porque o
protocolo é orientado a objetos, não a comandos. A hipótese original de uma
tabela de opcodes era um viés do protocolo GP-50; o GP-100 é mais simples:
**um único envelope de objeto + 7 payloads + state-sync**.
