# PLANO DE CAPTURA — Como obter os valores de campo específicos (0x817, flags)

> 📜 **HISTÓRICO (2026-09-28):** plano cumprido — a Rota 1b (proxy winmm) foi a
> vencedora e as sessões 1–4 fecharam tudo isto. Estado atual do protocolo:
> `docs/PROTOCOL.md` §13 + `docs/protocol_golden.json`. Gaps restantes de captura:
> `analysis/capture_gaps.md` (G3–G6). Mapa geral: `docs/INDEX.md`.

**Objetivo:** capturar 1 sessão real do Suite GP-100 para calibrar os campos
ambíguos do protocolo já mapeado. **Não é para "descobrir o protocolo"** — o
mapa estrutural está fechado (`docs/PROTOCOL.md` §12); a captura só preenche
valores de campo e confirma o timing.

## Rota 0 — DebugView (passiva, 5 min, sem instalar nada no fluxo MIDI)

O GP-100.exe importa `OutputDebugStringW` (IAT `0x7DD0B0`) e carrega 62 strings
de log de protocolo (`checksum:%d`, `row:%d,effectCode: %08X`, `IR Data:`,
`Nam Data:`). Se o caminho de log estiver ativo (o campo `debug` existe nas
Preferences do app), **tudo que precisamos aparece no DebugView sem tocar no
device**.

Procedimento:
1. Baixar DebugView (Sysinternals, Microsoft Store ou learn.microsoft.com).
2. Rodar como admin, ativar `Capture Win32 + Global Win32`.
3. Abrir o Suite GP-100, conectar a pedaleira, fazer: scan de presets,
   editar 1 parâmetro, salvar, importar 1 IR, importar 1 NAM (SnapTone).
4. Salvar o log (`File > Save`) → `analysis/captures/debugview_session1.log`.

O que procurar no log: linhas com `checksum:` (confirma CRC-8/0x07 no fio),
`effectCode: %08X` (tags de objeto), `IR Data:`/`Nam Data:` (payload dos
tipos 5).

## Rota 1 — MIDI spy (passiva, 15 min, ferramenta pronta neste repo)

Só se a Rota 0 não logar os bytes. Duas opções:

### 1a. MIDI-OX / MIDI Monitor-style (Windows)
- MIDI-OX (grátis) ou Snoize/ MIDI Monitor (mac) como *stub driver*? No
  Windows, MIDI-OX não é driver-filtro; usar **loopMIDI** (Tobias Erichsen) como
  hub: Suite → loopMIDI port → nosso sniffer lê a mesma porta. **Mas** o GP-100
  é hardware USB-MIDI, não dá para "espiar" a porta física sem um driver filtro.
- Solução robusta no Windows: **`miditest`/ `MIDIMonitor` com driver filtro** é
  frágil; o caminho confiável é o **1b**.

### 1b. Proxy DLL (passiva, ~1h de setup, 100% confiável)
Criar `winmm_proxy.dll` que encaminha tudo para a `winmm.dll` real e **loga
`midiOutLongMsg`/`midiOutShortMsg`/`midiInAddBuffer`** com timestamp para
`captures/midi_trace.jsonl`. Instalar colocando a DLL junto ao `GP-100.exe`
(com o .exe original renomeado ou via `.local` config do loader). Riscos: zero
para o device (somente leitura das mensagens); reversível removendo a DLL.

> Entrego o código-fonte do proxy na próxima sessão (é pequeno: 4 funções
> hookadas, 1 thread de flush). Não o compilei ainda porque exige MSVC/MinGW
> na máquina.

## Rota 2 —hardware real (ativa, 30 min, só para WRITE_VERIFIED)
1. Conectar a GP-100, abrir Suite, ativar o spy da Rota 0/1.
2. No Suite: ler todos os presets (state-sync de leitura), editar 1 knob,
   gravar em slot vazio, importar 1 IR e 1 NAM.
3. O sniffer produz `captures/session1.jsonl` com eventos
   `{ts, dir, bytes[]}`.
4. Rodar `analysis/decode_capture.py` (a criar) que decodifica com o mapa
   atual e aponta os campos desconhecidos — deve fechar `0x817` e flags em
   minutos.

## Ordem recomendada
1. **Rota 0 primeiro** (5 min, zero risco) — pode resolver tudo sozinha.
2. Rota 1b se precisar dos bytes crus (o proxy é passivo e seguro).
3. Rota 2 apenas para `WRITE_VERIFIED` (validação de escrita real), com a
   pedaleira física — e mesmo assim, primeiro slot vazio + read-back.

## Critério de "feito"
- [ ] `0x817` decodificado (provável assinatura de sessão IR: 2071 = 0x817
      pode ser coincidência numérica; confirmar no log).
- [ ] Flags de sessão (2 bits do tipo 5) semantados.
- [ ] 1 trace completo de leitura + 1 de escrita decodificados sem campos
      desconhecidos.
- [ ] `WRITE_VERIFIED=true` no gp100-core (só com Rota 2).

---

# CAPTURA 5 — `change-effect` (`0x47`): trocar o algoritmo dentro de um slot

> **Por que esta é a única captura que ainda segura a edição de patch.** A
> família `0x4X` do PROTOCOL §4 é `0x43` select, **`0x47` change-effect**,
> `0x48` change-param, `0x49` toggle-block, `0x4F` bulk write. Só `0x48`
> (§13.11, envelope `10 <slot> 00 02`) e `0x4F` (§4, bulk do `.prst`, op `0x1D`)
> têm formato validado em campo. Sem o `0x47`, a **lista de efeitos por pedal**
> (issue #19) fica em prévia local — a UI é inteira, a escrita não.
>
> Não é um fluxo novo: é o mesmo envelope semântico do §13.11 com outro
> endereço. A hipótese mais provável (a testar, não a assumir) é
> `10 <slot> 00 01 | [effectCode u32 LE]` — espelhando o `00 02` do knob, com
> `effectCode` = `(nibble << 24) | index` do `.prst`. **Só a captura decide.**

## Roteiro (30–45 min com a pedaleira, mesma instrumentação das sessões 1–4)

1. Instrumentar o Suite com a **Rota 1b** (proxy winmm) ou espelhar na
   `loopMIDI` — a sessão 3 (`analysis/captures/session3.jsonl`) já tem o
   ferramental pronto, inclusive `analysis/dump_edit_writes.py`, que extrai os
   `12 10 XX 00 02`.
2. Conectar a GP-100, abrir um patch de fábrica **não vazio** (o mesmo da
   sessão 3), e abrir o painel do módulo PRE.
3. **Uma troca por vez**, com 2 s de settle entre elas e o DebugView aberto:
   - PRE: `C-Wah` → `COMP` (mesma família, mesmo slot) — isola o **código**;
   - PRE: `C-Wah` → `OCTA` (família PRE, índice alto) — confirma se o índice
     entra como u32 cru ou campo a campo;
   - qualquer módulo: `PRE` → `Boost`/`Boost` (o MESMO algoritmo `0x1A0001A`
     existe em PRE, DST, MOD, DLY e CAB) — separa **troca de algoritmo** de
     **troca de módulo/família** (nibble muda junto?).
4. Em cada par, capturar **um write logo após** a troca e confirmar o read-back
   pelo display da pedaleira (o efeito mostrado muda para o escolhido).
5. Repetir um passo com a **troca de POSIÇÃO** de um módulo (o "arraste ⇄" da
   UI): é o `0x49`/reordenação e provavelmente sai no mesmo `0x4F` — separar os
   dois no log é o ponto.

## O que a captura precisa responder

- [ ] Endereço e formato exatos do `change-effect` (`0x47` ou `10 <slot> 00 01`?).
- [ ] O `effectCode` vai como **u32 LE** (como no §13.11) ou expandido nibble-a-nibble?
- [ ] Ao trocar de efeito, o device **zera os `params_N`** do slot? (O `.prst`
      tem 15 `params` por `Effect`; se o wire mandar só o código, os params
      antigos ficam — e isso decide se a UI precisa mandar defaults junto.)
- [ ] A troca de **posição** entre módulos é um write próprio ou sai no bulk
      `0x4F` do `.prst` inteiro?
- [ ] Ack/resposta? (O §13.11 registra ZERO ack nas edições de knob.)

## Depois da captura

1. Congelar os bytes em `analysis/fixtures/` + um caso no golden
   (`analysis/build_golden.py` → `docs/protocol_golden.json`), como nas sessões
   3 e 4.
2. Promover o item 10 da matriz do `docs/BLOCKERS.md` (opcodes lógicos) de
   "parcial" para "RESOLVIDO EM CAMPO" **só** para o `change-effect`.
3. Habilitar `WRITE_VERIFIED` do `change_effect` no gp100-core e ligar o
   `device_change_effect` no front (hoje a UI da #19 é prévia local).
