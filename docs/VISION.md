# Documento de Visão e Estratégia de Engenharia
## Projeto: GP-100 NextGen Editor — Suíte multiplataforma para a pedaleira Valeton GP-100

> ✅ **Status: atual (rev. v1.2, 29/09).** §3 e §10 registram a RE concluída com as
> evidências das capturas 1–4; §10.6 (round-trip Rust) já executado na M0.2; §11
> é o roadmap vigente — execução issue-a-issue em `docs/ROADMAP.md` (P/M0/H) e
> `docs/UI_PLAN.md` (M1). Mapa geral: `docs/INDEX.md`.
**Versão:** 1.1 · **Data:** 2026-09-28 (rev. pós-sessões de captura 1–4: fases R1–R4 concluídas, roadmap reescrito) · **Classificação:** Interno / Engenharia
**Autor:** Buffy — Arquitetura & Engenharia Reversa

---

## 1. RESUMO EXECUTIVO

O objetivo do projeto é substituir o software oficial da Valeton (shell web/CEF, Windows-only) por uma aplicação **multiplataforma (Windows/Linux/macOS)**, de código aberto, com engenharia de dados superior, UI moderna e segurança absoluta para o hardware.

A fase de **reconhecimento estático já foi executada** com sucesso sobre os artefatos locais e produziu os seguintes ativos verificáveis:

| Ativo | Evidência | Local |
|---|---|---|
| Firmware V2.1 analisada | 3.424.316 B, binário ARM Cortex-M7 (NXP i.MX RT, FlexSPI), ~10.301 strings extraídas | `analysis/fw_strings.txt` |
| Protocolo de patch decodificado | Formato XML (não binário); semântica do `effectCode` resolvida: `(módulo<<24) \| índice` | `analysis/effect_catalog.csv` |
| Catálogo de efeitos | 909 slots, 116 efeitos distintos, 9 módulos, ordem da cadeia mapeada (`x=0..8`) | `analysis/effect_catalog.csv` |
| Módulos do DSP | PRE, DST, AMP, CAB, NR, EQ, MOD, DLY, RVB (nibbles 0x00–0x0C) | tabela §4.2 |
| Funcionalidade oculta | **SnapTone (V2.0+): importação de modelos NAM** no módulo AMP, slots 1–5 | `analysis/release_note.txt` + strings `0x002803AD` |
| Changelog oficial V1.3→V2.1 | Extraído do RTF | `analysis/release_note.txt` |
| Stack do app oficial | Shell CEF/web + MFC; driver ASIO via MSI (`Valeton_UsbAudio_v5.57.2`) | análise de strings do instalador |
| **Protocolo de FIO (§13.1–13.12)** | SysEx Valeton confirmado em campo: ler/editar/salvar/IR | capturas 1–4 + `docs/protocol_golden.json` (40 templates, validação 100%) |
| **Envelope do knob (§13.11)** | write semântico `[effectCode u32LE][ctrl][00][f32 LE]` | `analysis/knob_map.json`: 89/89 edits regeneradas byte-a-byte |

**Riscos neutralizados:** o formato de preset é **XML de texto legível** — não há necessidade de sniffing USB para a camada de dados; a correlação firmware↔patch já é possível hoje. **E, com as capturas 1–4 (proxy winmm), o protocolo de fio completo foi confirmado em campo e validado byte-a-byte (§13 + golden-file) — a última dependência de hardware foi eliminada.**

**Recomendação de stack:** **Tauri 2 + React/TypeScript** no front, **Rust** no núcleo (USB/serial, validação, banco), com fallback avaliado em PySide6 — detalhado na §6.

---

## 2. INVENTÁRIO DE ARTEFATOS (estado real de `files/`)

| Arquivo | Tamanho | Natureza | Status de análise |
|---|---|---|---|
| `GP-100 Firmware V2.1.bin` | 3.424.316 B | Firmware ARM (Cortex-M7, i.MX RT1050/1060-family), sem criptografia | ✅ strings extraídas; descritores USB montados em runtime (padrão NXP SDK) |
| `GP-100 Firmware Release Note.rtf` | 162 KB | Changelog V1.3→V2.1 (EN/CN) com imagens embutidas | ✅ texto extraído |
| `GP-100 Setup V1.5.1 for Windows.exe` | 80 MB | Instalador NSIS (x86 launcher); app = shell CEF/web | ✅ extraído (`analysis/nsis_app/`) + binário nativo desmontado (`analysis/FINDINGS_*.md`) |
| `Valeton USB ASIO Driver V5.57.2 Setup.exe` | 4,7 MB | Instalador que baixa/empacota MSI do driver USB Audio | ✅ MSI extraído (`analysis/driver_ext/`); **VID_84EF / PID_0021** confirmados no `.inf` |
| `GP-100_Online Manual_EN_Firmware V2.0.pdf` | 4,8 MB | Manual oficial; texto em fontes CID com ToUnicode CMap | ✅ texto bruto extraído (`analysis/manual_streams.txt`) |
| `patches/*.prst` (3 arquivos) | 4 KB / 413 KB | **XML**: 1 preset, 99 presets, preset unitário | ✅ 100% decodificado (§4) |
| `images/*.png` | — | Screenshots do app oficial V1.5.1 | ✅ inspecionados (visual + Settings) |
| `prompt_inicial.md` | — | Briefing do projeto | ✅ |

Diretório de trabalho: `analysis/` (scripts + produtos). Ferramentas da fase RE: `extract_strings.py`, `rtf_text.py`, `pdf_text.py`, `build_catalog.py`/`build_parameters.py` (dicionário), desmontadores capstone (`disasm_*.py`, `rtti_disasm.py`), **proxy winmm** (`build_proxy.py` + `midi_proxy.c`), decoders de captura (`decode_wire.py`, `validate_knob_map.py`, …), **golden-file** (`build_golden.py`/`validate_golden.py`).

---

## 3. PLANO DE AÇÃO — ENGENHARIA REVERSA (✅ CONCLUÍDA — rev. v1.1)

### Fase R1 — Firmware e dicionário (✅ CONCLUÍDA)
1. ✅ Strings ASCII/UTF-16LE com offsets (`extract_strings.py` → `fw_strings.txt`, ~10.301 strings).
2. ✅ Censo de símbolos: `algorithmParaNameConstData.c`, `FlashDataProcess.c`, asserts de flash (`FLASH_*_SIZE`) — delimitam presets/IRs/SnapTones/global.
3. ✅ Tabela de UI multilíngue localizada (0x279xxx–0x281xxx) e reutilizada para i18n do novo app.
4. ✅ **Dicionário canônico**: 185 algoritmos / 639 controles com ranges e defaults — obtido do `algorithm.xml` (payload oficial do instalador) e validado em 3 vias (909 slots de patches, strings da firmware, nome verificado no firmware p/ 150) → `analysis/parameters.json`.
5. ✅ Auditoria opcional documentada em `analysis/README_GHIDRA.md` (extração in-place da `algorithmParaNameConstData`, não bloqueia nada).
6. ✅ RE estática do exe (capstone, sem Ghidra): FSM/opcodes (`FINDINGS_FSM.md`), 7 formatos de objeto (`FINDINGS_OBJECTS.md`), VLV/tag-mapper (`FINDINGS_COMMANDMAP.md`), protocolo (`FINDINGS_PROTOCOL.md`) — corrobora o §13. 0x817 resolvido (§13.8).

### Fase R2 — Formato de presets (✅ CONCLUÍDA)
1. ✅ Parser XML completo; 116 efeitos catalogados em 909 slots (`effect_catalog.csv`).
2. ✅ `effectCode = (nibble<<24) | index` (§4.2); confirmado no FIO como u32 LE (§13.11).
3. ✅ Cadeia de posição fixa `x=0..8`.
4. ✅ `params_0..14` pareados com o dicionário (mesma ordem; valores do fio = unidade física float32).
5. ✅ **Controle oculto descoberto**: Mic do CAB (ctrl 1) existe no .prst e no hardware mas não no algorithm.xml — regra: preservar `params_N` 0..14 sempre.
6. ✅ `ppIRInfo`/CRC-8/0x07 confirmado; upload de IR por SysEx com ACK por chunk capturado em campo (§13.7).
7. ⏳ Semântica fina de `ppEXP1`/`ppCtrl` (expCode) — baixa prioridade, dados completos no XML.

### Fase R3 — Executável/driver (✅ CONCLUÍDA)
1. ✅ Stack identificada (CEF shell + MFC launcher); NSIS extraído para `analysis/nsis_app/`.
2. ✅ Binário nativo do Suite desmontado (pefile+capstone) — achados nos `FINDINGS_*.md`.
3. ✅ MSI do driver extraído (`analysis/driver_ext/`); **VID_84EF / PID_0021** confirmados.

### Fase R4 — Protocolo de transporte (✅ CONCLUÍDA EM CAMPO)
- ✅ Hipótese confirmada: USB-MIDI class-compliant + SysEx vendor `F0 21 25 7F 47 50 2D 64`.
- ✅ **Proxy winmm.dll** (129 exports, hooks de 4 APIs) instrumentou o Suite oficial sem risco ao device.
- ✅ **4 sessões de captura** analisadas: boot/scan (S1), upload de IR (S2), edição por UI (S3), save + persistência (S4, confirmada no display da pedaleira).
- ✅ Entregável completo: `docs/PROTOCOL.md` §13.1–13.12 + **`docs/protocol_golden.json`** (40 templates, validação byte-a-byte 100% nos fluxos de knob e boot/scan; IN 100% no accounting).
- ⏳ Apenas firmware-update (bootloader) permanece diferido por política de segurança (V2+).

---

## 4. DESCOBERTAS TÉCNICAS (EVIDÊNCIAS)

### 4.1 Formato do preset (XML)
```xml
<GP-100>
  <preset_info software="1.2.0" firmware="2.1" product="GP-100" count="99" platform="WINDOWS" time="..."/>
  <presets ppBank="0" ppName="It's GP100" ppVolume="50" ppID="0" ppBPM="120"
           ppIRNum="27" ppType="4" ppTypeName="Rock" ppAuthor="" ppNotes="">
    <Effect effectModuleName="RVB" effectName="Hall" effectState="1"
            effectCode="201326593" x="8" y="0" params_0="24" ... params_14="0"/>
    ...
    <ppCtrl c11="1" c12="65535" .../>              <!-- footswitches -->
    <ppEXP1 expTarget="0" expVolume="0" expVolumeMin="0" expVolumeMax="99">
      <ppEXP1_0 expMId="0" expCode="83886088" expIndex="0" expMin="0" expMax="99"/>
      ...
    </ppEXP1>
  </presets>
  <ppIRInfo> <!-- 20 User IRs com CRC --> </ppIRInfo>
</GP-100>
```
- `65535` = "não configurado" (0xFFFF).
- `effectState` = bypass(0)/on(1).
- O software oficial grava `software="1.2.0"` mesmo no app 1.5.1 → versionamento independente do schema.

### 4.2 Mapa módulo↔nibble (derivado de 909 amostras, zero ambiguidade)
| Nibble | Módulo | Nº efeitos no catálogo | Exemplos |
|---|---|---|---|
| 0x00 | NR / PRE | Gate 1–2; Boost, COMP, COMP4 | noise gate / compressor |
| 0x01 | EQ | 3 | EQ 1, EQ 2, Mess EQ |
| 0x03 | DST | 11 | Green OD, SM Dist, Blues OD |
| 0x04 | MOD | 9 | A-Chorus, Flanger, Phaser |
| 0x05 | PRE (wah) | 2 | C-Wah, V-Wah |
| 0x07 | AMP | 28 | Match OD, Bog RedM, UK 900 |
| 0x08 | AMP (pré) | 3 | AC Pre, Bass Pre, Mini Bass |
| 0x0A | CAB | 31 | U-ban 4x12, Bad-KT 1x12 |
| 0x0B | DLY | 10 | T-Echo, Dual Echo, 999 Echo |
| 0x0C | RVB | 9 | Hall, Spring, N-Star, Shimmer* |
| — | PRE extras | Hammy (0x02?), 14 Boost, 1930s/1940s Dynamic, SnapTone1–5 (0x08) | *Shimmer é RVB; SnapTone é AMP |

### 4.3 Correlações firmware ↔ patch (exemplos verificados)
- `Match OD` (AMP, code 117440584 = 0x07000048 → índice 72) e string `Match OD` em `0x0027E701`.
- `U-ban 4x12` (CAB, 167772204 = 0x0A0000CC → índice 204) e string `U-ban 4x12` em `0x0027FAB9`/`0x002B6D9C`.
- `SnapTone1..5` em `0x002803AD..0x002803DD` — **cinco slots NAM**, confirmados pelo release note V2.0.
- Asserts de flash delimitam o banco de dados do usuário: presets, IR WAVs, SnapTones, global.

### 4.4 Dores confirmadas do app oficial (reviews + screenshots)
- Windows-only; UI web embutida pesada (CEF); sem versão para Linux.
- Edições críticas restritas ao hardware (relatos da comunidade GP-5/família).
- Sem undo/redo, sem biblioteca local com versionamento, sem backup automático.
- Fluxo de EXP/footswitch pouco intuitivo (relatos TDPri/Reddit).
- Sem editor de IR (upload de .wav) e sem gestão de SnapTone/NAM no desktop.

---

## 5. ARQUITETURA PROPOSTA — GP-100 NEXTGEN

```
┌────────────────────────────────────────────────────────────┐
│ UI (React 18 + TS, Vite) — temas dark/light, GPU-friendly  │
│  · Signal Chain 2D (drag-and-drop real)                    │
│  ·knobs/curvas SVG, EQ interativo, medidores via WebAudio  │
└───────────────▲────────────────────────────────────────────┘
                │ IPC tipado (Tauri commands) / WebSocket local
┌───────────────┴────────────────────────────────────────────┐
│ NÚCLEO (Rust): gp100-core                                   │
│  · modelo de preset (serde) ↔ XML oficial (round-trip!)     │
│  · dicionário de algoritmos (parameters.json, versionado)   │
│  · transporte USB: hidapi/serial/USB-MIDI SysEx (R3/R4)     │
│  · validação + checksums + modo dry-run                     │
│  · biblioteca SQLite (presets, histórico, snapshots IR/NAM) │
└─────────────────────────────────────────────────────────────┘
```

### Princípios
1. **Round-trip sagrado**: qualquer preset aberto→salvado deve gerar XML byte-idêntico se nada mudou (teste de regressão obrigatório).
2. **Hardware write-path sempre com dupla confirmação + dry-run**: validação de schema, ranges por parâmetro e CRC antes de qualquer escrita.
3. **Dicionário versionado por firmware** (`fw-2.0.json`, `fw-2.1.json`): novas firmwares adicionam arquivos, nunca quebram os antigos.
4. **Linux first-class**: transporte via ALSA USB-MIDI + Audio class nativos (sem driver proprietário).

---

## 6. DECISÃO DE STACK (PRÓS/CONTRAS CORPORATIVOS)

| Critério | Tauri 2 + React + Rust | Electron + React + Node | PySide6 + Python |
|---|---|---|---|
| Peso do binário | ★★★★★ (~10–20 MB) | ★★ (150–250 MB) | ★★★ (60–120 MB empacotado) |
| Latência de UI | ★★★★★ | ★★★ | ★★★★ |
| Acesso USB nativo | ★★★★★ (hidapi/serial/nusb) | ★★★ (node-hid/serialport) | ★★★★ (pyhidapi/pyserial) |
| Round-trip XML/validação | ★★★★★ (serde + quick-xml) | ★★★★ | ★★★★ |
| Manutenibilidade / talento | ★★★★ (2 linguagens) | ★★★★★ (1 linguagem TS) | ★★★★★ (1 linguagem Py) |
| Linux/macOS | ★★★★★ | ★★★★ | ★★★★ |
| Empacotamento/distribuição | ★★★★ (NSIS/MSI/AppImage/dmg) | ★★★★★ | ★★★★ |

**Decisão: Tauri 2 + React/TS + Rust.** Justificativa: binário ~10× menor que Electron (crítico para distribuição comunitária), acesso a HID/serial/midi sem FFI pesado, validação de dados compilada (o mesmo código Rust pode gerar bindings TS via `ts-rs`, eliminando drift de schema). **Plano B** (se a equipe for 100% Python): PySide6 + `python-bytepath` — custo: app ~5× maior e empacotamento mais frágil no Linux.

> Nota: o app **não processa áudio em tempo real** (o DSP fica no hardware); portanto a vantagem de latência nativa do Rust é de I/O de controle, não de áudio — o que torna o Tauri ainda mais adequado. Medidores de nível, se desejados, podem consumir o stream USB Audio do sistema (WASB/ALSA/CoreAudio) ou um tap do próprio ASIO.

---

## 7. SEGURANÇA DE HARDWARE — GESTÃO DE RISCOS (BRICK-PROOF)

### 7.1 Matriz de riscos
| Risco | Prob. | Impacto | Mitigação |
|---|---|---|---|
| Escrita de preset corrompida (XML inválido→flash) | Média | Baixo (preset perdido) | Validação de schema + ranges; CRC32 por bloco; leitura de verificação pós-escrita (read-back compare) |
| Escrita no bloco global errado | Baixa | Médio (config global) | Endereços fixados por constante de firmware (R1.6); nunca inferir endereço em runtime |
| **Firmware update interrompido** | Baixa | **Crítico (brick)** | **V1: nunca gravar firmware.** V2 (se implementado): bootloader-first,Chunks+ACK/NAK+CRC32, retomada por chunk, dupla confirmação, energia AC obrigatória, log completo |
| Sequência de handshaking incorreta | Média | Baixo (device ignora) | Máquina de estados espelhada no firmware; timeouts + retry exponencial; abort limpo |
| IR WAV fora de spec | Média | Baixo | Validação de sample-rate/tamanho contra `FLASH_RSER_IR_WAV_DATA_SIZE` |
| Conflito com app oficial aberto | Média | Médio (device occupancy) | Detecção de dispositivo ocupado; aviso para fechar o editor oficial |

### 7.2 Protocolo de escrita (obrigatório, em ordem)
1. **Modo dry-run default** na primeira execução (flag persistida).
2. Validação completa: schema XML → ranges por parâmetro (dicionário da R1.4) → IDs de algoritmo existentes na firmware-alvo.
3. Escrita **somente em blocos** com CRC32 e numeração; espera ACK por bloco.
4. **Read-back compare** do bloco gravado; divergência = retry ×3 = falha com rollback local.
5. Journaling local (SQLite): toda operação de escrita registrada com diff, timestamp e resultado — permite auditoria.
6. **Nenhuma rotina de firmware-update antes de documentar o bootloader da R1.6 e de implementar modo de recuperação (V2).**

---

## 8. BENCHMARKING UI/UX E MODERNIZAÇÃO

### 8.1 Referências
- **Neural DSP (Quad Cortex/Native):** cadeia visual drag-and-drop, metadados por preset, cloud.
- **Line 6 Helix Native:** editores grandes por bloco, snapping de parâmetros, undo profundo.
- **Boss Tone Studio:** integração de biblioteca + editor por tela única (denso, mas eficiente).
- **Aprendizado:** o padrão vencedor é **cadeia horizontal + painel de parâmetros contextual + biblioteca lateral** — o app oficial do GP-100 já segue isso, mas sem profundidade (sem undo, sem busca semanticamente rica, sem diff).

### 8.2 Modernizações propostas
- Dark/light responsivo, densidade ajustável, escala de UI (acessibilidade).
- **Cadeia de sinal 2D** com bypass por clique, medidores por módulo e reordenação drag-and-drop (simulada na exportação para o layout fixo do GP-100).
- **Diff de presets** (antes/depois) com undo/redo ilimitado por sessão.
- **EQ/CAB interativo**: curva de resposta desenhável sobre o gráfico (nos módulos que permitem mapeamento).
- Biblioteca com tags, busca fuzzy, favoritos, coleções e importação em lote do `all.prst` oficial.
- Internacionalização (pt-BR, en, es, zh já existem no firmware — reutilizamos as strings!).

---

## 9. INOVAÇÃO — FUNCIONALIDADES INÉDITAS (mínimo 5, entrego 10)

1. **Biblioteca versionada local (Git-like):** histórico por preset com diff visual e restauração pontual — nada disso existe no app oficial.
2. **Tone Match por música de referência:** dado um áudio, sugestão de cadeia (AMP+CAB+IR+DST) por similaridade de espectro/estilo usando modelos locais (ONNX runtime) — IA offline.
3. **Gestor de SnapTone/NAM:** importar/exportar/arquivar modelos NAM nos 5 slots da V2.0+ (o app oficial nem expõe isso no desktop).
4. **Laboratório de IRs:** carregar .wav, visualizar espectro/decay, normalizar, trimar e atribuir aos 20 User IRs com CRC correto.
5. **Cloud sync opcional + comunidade:** backup E2E-encrypted e compartilhamento de presets com preview de cadeia (opt-in, self-hostable).
6. **Live Mode:** telão com BPM, setlists, troca por atalho/teclado MIDI, metrônomo visual e tuner estendido.
7. **Exportação universal:** conversão .prst↔JSON↔PDF "tone sheet" (imprimível com diagrama de cadeia e valores).
8. **A/B instantâneo com blind test** entre duas versões do preset (nível calibrado) — refinamento de timbre científico.
9. **Automação de parâmetros via DAW:** ponte MIDI CC/OSC para gravar automação dos knobs do GP-100 no DAW.
10. **Assistente de gain-staging:** análise do preset aponta headroom/clipagem estimada por módulo antes de enviar ao hardware.

---

## 10. LISTA DE TAREFAS INICIAIS (✅ TODAS EXECUTADAS — rev. v1.1)

1. ✅ NSIS extraído (`analysis/nsis_app/`) + binário nativo desmontado — protocolo revelado por RE **e confirmado em campo** (capturas 1–4).
2. ✅ MSI do driver extraído; VID/PID confirmados no `.inf`.
3. ✅ Dicionário `parameters.json` obtido do `algorithm.xml` (via mais confiável que Ghidra) e validado em 3 vias; auditoria Ghidra opcional em `README_GHIDRA.md`.
4. ✅ Manual extraído (`manual_streams.txt`) para cross-check.
5. ✅ `params_0..14` pareados com o dicionário + validados no fio (float32, §13.11).
6. ✅ **Concluída na M0.2 (29/09)**: modelo serde do `.prst` com round-trip
   XML byte-idêntico dos 3 arquivos (`tests/roundtrip_prst.rs`, 8 contratos;
   layout tratado como DADO). Era a última pendência desta lista.
7. ✅ Capturas reais executadas (sessões 1–4, proxy winmm) — protocolo validado em campo, item 11 do BLOCKERS fechado.

---

## 11. ROADMAP (rev. v1.2 — execução detalhada no ROADMAP.md e no UI_PLAN.md)

> A fase de arqueologia terminou: dados, formato e protocolo estão fechados
> e validados. O caminho crítico agora é 100% construção de software — e a
> Fase M0 está **100% concluída (8/8) com CI verde (29/09)**.

- **M0 — gp100-core (Rust) — ✅ 100% (8/8, 29/09):** model (M0.1), preset
  round-trip (M0.2), golden consumer (M0.3), codec de fio (M0.4), transporte
  + MockDevice D1–D8 (M0.5), FSM de sessão com replay byte-a-byte das 4
  fixtures (M0.6), gp100-cli com `--log` no schema P4 (M0.7) e docs do core
  com contrato de exemplos que rodam (M0.8).
  **Execução issue-a-issue, status e achados: `docs/ROADMAP.md`** (aqui fica
  só o panorama).
- **H — gate de hardware (entre M0 e a escrita real):** H1 (leitura real) →
  H2 (escrita dos 3 fluxos capturados) → H3 (golden v1.1 se houver ajuste).
- **M1 — Editor UI (Tauri 2 + React/TS) — ⏳ planejado (`docs/UI_PLAN.md`):**
  biblioteca (import `all.prst`), editor de cadeia 9 slots, knobs com ranges
  reais do dicionário, diff/undo; device mock primeiro, hardware depois
  (set/save/IR já verificados em campo). `WRITE_VERIFIED=true` para os
  3 fluxos capturados; todo fluxo novo nasce atrás de feature-flag até ter
  captura própria. **Pode começar em paralelo (mock); o modo real só após
  H1/H2** — issues M1.0–M1.6 com DoD no `docs/UI_PLAN.md`.
- **M2 — IR lab + SnapTone manager + empacotamento:** laboratório de IRs (upload §13.7
  já funcional; visualização/normalização local), gestor de NAM (5 slots), i18n (reusar
  strings EN/CN da firmware + pt-BR), pacotes MSI/AppImage/dmg.
- **M3 — Diferenciais (§9):** biblioteca versionada git-like, live mode, cloud opt-in,
  tone match IA, A/B blind test.
- **Dívidas técnicas de baixa prioridade** (não bloqueiam nada): layout byte-a-byte da
  página de estado 13xx ↔ params (completar com mais capturas de scan), campo 0x00BC/0x00B4
  e alinhamento de samples do blob de IR, semântica fina de ppEXP1/ppCtrl, gaps G3–G6
  (globals/BPM, knob físico, footswitch — `analysis/capture_gaps.md`). A tabela de
  TIPOS da `12001002` e o schema do `11000007` foram FECHADOS em §13.12 (28/09).

---

## 12. APÊNDICE — COMANDOS/SCRIPTS DE ANÁLISE
```bash
python analysis/extract_strings.py   # strings do firmware -> analysis/fw_strings.txt
python analysis/rtf_text.py          # changelog -> analysis/release_note.txt
python analysis/build_catalog.py     # catalogo de efeitos -> analysis/effect_catalog.csv
python analysis/pdf_text.py          # extracao bruta do manual (precisa pypdf p/ CID)
```

*Fim do documento.*
