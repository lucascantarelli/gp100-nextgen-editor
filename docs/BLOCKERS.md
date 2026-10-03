# MATRIZ DE BLOQUEIOS — Podemos seguir sem captura USB?

**Data:** 2026-09-28 · **Status:** ✅ atual · Mapa geral: `docs/INDEX.md` · **Resposta curta: NÃO é bloqueante para ~90% do projeto.**
Os dois únicos pontos que dependiam de hardware — (1) opcodes lógicos host→device
e (2) validação de escrita — foram **resolvidos em campo** nas sessões de captura
1–4 (§13 do PROTOCOL.md; item 11 verificado pelo usuário no display da pedaleira).
Tudo o mais — e são as partes que levam semanas — está resolvido com os arquivos locais.

---

## 1. Matriz por subsistema

| # | Subsistema | Dados necessários | Fonte local suficiente? | Status |
|---|---|---|---|---|
| 1 | Modelo de dados (efeitos, parâmetros, ranges, defaults) | tabela de algoritmos | ✅ `parameters.json` (185 algs, validado 3 vias) | **RESOLVIDO** |
| 2 | Formato de preset XML (pp*, Effect, ppEXP1, ppCtrl, ppIRInfo) | schema | ✅ patches reais + strings do exe (`preset_info`, `pp*`) | **RESOLVIDO** |
| 3 | Round-trip XML byte-idêntico (ler/editar/gravar .prst) | schema + amostras | ✅ `all.prst` (99 presets) + 2 unitários | **RESOLVIDO** |
| 4 | Editor (cadeia, knobs, biblioteca, i18n) | dicionário + strings de UI | ✅ `algorithm.xml` + `English.lng` + `SimplifiedChinese.lng` + tabela de UI da firmware | **RESOLVIDO** |
| 5 | Gestão de SnapTone/NAM (slots, conversão desktop) | formato .nam + refit | ✅ parcial — o próprio exe tem o motor NAM (WaveNet/LSTM + Eigen); refit é o `NamConvertThread`; formato .nam é open source (Arduino-Tone-... NeuralAmpModelerCore) | **RESOLVIDO/RE** |
| 6 | Laboratório de IRs (formato WAV → ppIRCRC) | layout IR + CRC | ⚠️ CRC confirmado CRC-8/0x07; layout exato do bloco IR vem do serializer (`0x6209D0`, caso com sample-rates) — exequível por RE estática | **RE ESTÁTICA, sem hardware** |
| 7 | Enumeração do dispositivo (VID/PID) | ID de hardware | ✅ VID_84EF & PID_0021 confirmados no .inf do driver instalado | **RESOLVIDO** |
| 8 | Transporte (USB-MIDI, porta, taxa) | API SO | ✅ USB-MIDI class-compliant (fw strings `GP-100 MIDI` + imports Win32 do exe); ALSA/JUCE-midi equivalentes no Linux | **RESOLVIDO** |
| 9 | **Framing do wire (GP-100)** | ordem de bits/bytes | ✅ **RESOLVIDO EM CAMPO** (captura real, §13 do PROTOCOL.md: SysEx `F0 21 25 7F 47 50 2D 64` + func 0x11/0x12 + addr 4B; o TLV bit-level/CRC-8 é formato de ARQUIVO, não de fio) |
| 10 | **Opcodes lógicos host→device** (ler preset N, escrever, global, drum...) | sequências de comando | ✅ modelo endereçado confirmado em campo (READ 0x11 / resposta-escrita 0x12 + mapa de endereços §13.3 + upload de IR §13.7 + **envelope semântico do knob §13.11**); semântica fina das páginas `13 01 00 xx` = cruzar com parameters.json |
| 10b | **`change-effect` (`0x47`)** — trocar o algoritmo dentro de um slot (a lista de efeitos por pedal, issue #19) | sequência de comando | ❌ **ÚNICO opcode da família `0x4X` sem formato validado.** `0x43` select, `0x48` change-param (§13.11) e `0x4F` bulk (§4) têm bytes fechados; o `0x47` não. A UI da #19 é **inteira em prévia local**; só a escrita depende de captura própria → roteiro pronto em `docs/CAPTURE_PLAN.md` (CAPTURA 5) | **PENDENTE (roteiro pronto)** |
| 11 | Validação de escrita (read-back, timing) | device real | ✅ **RESOLVIDO EM CAMPO** — save da UI capturado (§13.12: metadados 11xx + ops 00020000 + re-sync 11000008, sem readback 13xx) e **persistência confirmada pelo usuário no display**: o slot de destino recebeu o preset com os valores editados da sessão 3 (AMP Gain ~99) ⇒ o save persiste o ESTADO AO VIVO. feature-flag WRITE_VERIFIED pode nascer `true` para os fluxos capturados (knob set, save, upload de IR) — **exceto** o `change-effect` (item 10b), que só entra com a CAPTURA 5 |
| 12 | Firmware update (HTFW-like) | bootloader | ⚠️ container FRMW decodificado (header/TOC); rotina de upgrade do device é a parte mais arriscada — **adiável indefinidamente** (V2+) | **Diferido por política de segurança** |

---

## 2. O que a captura USB daria que os arquivos não dão (✅ OBTIDO — sessões 1–4)

Exatamente **duas** coisas:
1. A sequência binária exata de cada comando lógico (opcodes, ordem de campos,
   valores de índice/bank) — hoje inferida da FSM (estados/handlers localizados:
   `0x626xxx`, jump-tables `0x621000`/`0x621210`) e do protocolo lógico da
   família GP-50.
2. Timing/ACKs reais para fechar o codec 100% (settle, reassembly, retry).

E mesmo isso tem caminho alternativo **sem pedaleira**:
- **RE estática dirigida** (temos capstone+RTTI operando): percorrer os handlers
  `0x621020/0x621030/0x621040` + tabela `0x621210` e as chamadas a `midiOut*`
  para recuperar os construtores de request. Custo estimado: dias, não minutos —
  mas é determinístico e auditável.
- **Corroborar com o protocolo GP-50** (lógica idêntica: nomes/corpos/IRs/SnapTone)
  para priorizar o que procurar.

## 3. Decisão de arquitetura que remove o bloqueio do caminho crítico

```
gp100-core (Rust)
 ├─ model/        ← parameters.json  (PRONTO)
 ├─ preset/       ← XML round-trip   (PRONTO p/ implementar)
 ├─ transport/    ← trait DeviceTransport
 │    ├─ mock.rs            ← grava/replay de capturas (.pcap/.json)   [sem hardware]
 │    ├─ usbmidi.rs         ← midir/ALSA/WinMM                          [API pronta]
 │    └─ codec/             ← TLV bit-level + CRC-8/0x07                [do disassembly]
 └─ session/      ← FSM (ler/escrever/global)  [opcodes: RE estática → mock → confirmar]
```

O app inteiro é desenvolvido e testado contra o **mock** (capturas sintéticas +
vetores do disassembly). Quando qualquer GP-100 real aparecer (seu ou de um
beta-tester da comunidade), uma única sessão de captura valida os opcodes e o
mesmo binário passa a falar com o hardware. **Nada do que foi produzido é
descartado.**

## 4. Riscos de tentar escrever no device sem a validação
- A comunidade GP-50 travou uma pedaleira com tráfego adivinhado (documentado).
- **Status atual:** o protocolo de escrita foi capturado e validado em campo
  (knob set §13.11; save §13.12 com persistência confirmada no display;
  upload de IR §13.7 com ACK por chunk). O gp100-core deve limitar a escrita
  aos fluxos capturados; fluxos NOVOS continuam atrás de
  `WRITE_VERIFIED=false` até terem captura própria.

## 5. Ordem recomendada
1. `gp100-core`: model + preset round-trip + testes com `all.prst` ✅ dados prontos
2. Editor UI completo (mock device) ✅ dados prontos
3. Transporte/FSM direto das capturas 1–4 (§13.10–13.12) ✅ protocolo validado em campo
4. `WRITE_VERIFIED=true` para os fluxos capturados: set de parâmetro, save, upload de IR ✅
5. Codec TLV/CRC-8 (§10–11) segue relevante apenas para objetos .bin (IR/NAM), não para presets
