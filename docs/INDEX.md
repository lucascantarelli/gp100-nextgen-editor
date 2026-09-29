# 📇 INDEX — Mapa da documentação (GP-100 NextGen Editor)

> **Comece aqui.** Este índice diz o que cada documento é, quando ler e qual é a
> fonte de verdade de cada assunto. Mantenha-o atualizado a cada novo documento
> ou mudança de status — é o contrato de navegação entre agentes e humanos.

**Última revisão:** 2026-09-29 (auditoria de skills; M0.1–M0.4 ✅)

---

## 1. Rotas rápidas ("quero…")

| Objetivo | Rota de leitura |
|---|---|
| **Entender o projeto** | `README.md` → `docs/VISION.md` |
| **Executar o plano de desenvolvimento** | `docs/ROADMAP.md` (issues P/M0/H em ordem, com DoD) |
| **Implementar o protocolo (gp100-core)** | `docs/protocol_golden.json` (especificação executável) + `docs/PROTOCOL.md` §13 (narrativa) |
| **Decidir arquitetura/estrutura no core (M0)** | `docs/DECISIONS.md` (ADR-1..5, P5) |
| **Escrever/revisar código Rust (M0)** | `.agents/skills/rust-practices/SKILL.md` (gates fmt/clippy/test + estilo de docs) |
| **Modificar o protocolo / analisar nova captura** | `.agents/skills/capture-analyze/SKILL.md` → decoders → `build_golden.py` → `validate_golden.py` |
| **Compilar o proxy / nova captura em campo** | `.agents/skills/proxy-build/SKILL.md` + `.agents/skills/new-session/SKILL.md` |
| **Consumir dados de preset/efeitos** | `analysis/parameters.json` (dicionário) + `docs/PROTOCOL.md` §13.9 |
| **Saber o que falta / riscos** | `docs/BLOCKERS.md` + `analysis/capture_gaps.md` |
| **Contexto de decisões de arquitetura/stack** | `docs/VISION.md` §6–§9 |
| **Armadilhas de ambiente (Windows/Git Bash)** | `knowledge.md` |

## 2. Fonte de verdade por assunto

| Assunto | Fonte única | Não consultar (obsoleto/supersedido) |
|---|---|---|
| Protocolo de fio (bytes) | `docs/protocol_golden.json` | reparsear `captures/*.jsonl` direto |
| Protocolo de fio (semântica/narrativa) | `docs/PROTOCOL.md` §13 | — |
| Formato de ARQUIVO (TLV/CRC-8/objetos) | `docs/PROTOCOL.md` §1–12 | §13 (é fio, não arquivo) |
| Dicionário de efeitos/parâmetros | `analysis/parameters.json` | `algorithm_dict.*` (intermediários) |
| Formato `.prst` | `docs/PROTOCOL.md` §13.9 | — |
| Envelope do knob / save / IR no fio | §13.11 / §13.12 / §13.7 | leituras antigas do §13.4 (marcadas) |
| Estado do projeto / próximos passos | `docs/BLOCKERS.md` + `knowledge.md` (estado vivo) | `docs/CAPTURE_PLAN.md` (histórico) |
| Mapa knob→fio | `analysis/knob_map.json` (regenerável) | — |
| Decisões de implementação do core | `docs/DECISIONS.md` (ADR-1..5) | reabrir debate ad-hoc |
| Armadilhas Windows/ambiente | `knowledge.md` | — |

## 3. Inventário de documentos

### `docs/` — referência do projeto
| Documento | Papel | Status |
|---|---|---|
| `VISION.md` | Visão de produto/arquitetura, stack, features, roadmap M0–M3 (rev. v1.1) | ✅ atual |
| `PROTOCOL.md` | Referência única do protocolo: §1–12 formato de arquivo, §13.1–13.12 fio confirmado em campo | ✅ atual |
| `protocol_golden.json` | 40 templates request→resposta das capturas 1–4; consumir DAQUI no gp100-core | ✅ atual · validado 100% |
| `BLOCKERS.md` | Matriz de 12 subsistemas; 11 resolvidos, firmware-update diferido | ✅ atual |
| `CAPTURE_PLAN.md` | Plano original das rotas de captura | 📜 histórico (cumprido) |
| `ROADMAP.md` | Plano executivo: preparação (P), gp100-core (M0), gate de hardware (H) com issues e critérios de aceite | ✅ atual |
| `DECISIONS.md` | ADR-lite com as 5 decisões estruturais do gp100-core (endian/nibble, erros, transporte, trait, WRITE_VERIFIED) | ✅ atual |
| `skills_audit_2026-09-29.md` | Auditoria das skills: regras que eram prática implícita, agora escritas (5 achados em core-dev/docs-sync/spec-baseline) | ✅ atual |
| `INDEX.md` | Este índice | ✅ manter atualizado |

### `analysis/` — laboratório (scripts + produtos + achados)
| Grupo | Arquivos | Papel |
|---|---|---|
| **Dicionário** | `parameters.json` (+ `build_parameters.py`, `algorithm.xml`, `algorithm_dict.*`) | 185 algs/639 controles, validado 3 vias |
| **Golden/validação** | `build_golden.py`, `validate_golden.py` | gera e prova a especificação executável (5 provas) |
| **Fixtures replay (P4)** | `make_fixtures.py` + `fixtures/` | fatia as 4 capturas por fase (boot/knobs/save/ir) p/ replay do M0.6; paridade no `manifest.json` |
| **Mapa de knobs** | `knob_map.json`, `validate_knob_map.py`, `dump_edit_writes.py`, `map_params_wire.py` | envelope semântico do knob (§13.11) |
| **Capturas** | `captures/session1–4.jsonl`, `ir_slot*.bin` | matéria-prima bruta (append-only!) |
| **Decoders** | `decode_wire.py`, `check_session2.py`, `extract_ir_upload.py`, `recon_session3.py`, `ctx_dump.py`, `tail_dump.py`, `flow_dump.py`, `scan_addr.py`, `probe_session4.py`, `map_state_pages.py`, `scan_prst.py`, `decode_capture.py` | análise dirigida das capturas |
| **RE estática** | `FINDINGS_PROTOCOL/FSM/OBJECTS/COMMANDMAP.md`, `rtti_disasm.py`, `disasm_*.py`, `xrefs_opcodes.py`, `opcode_extract.py`, `find_817_imm.py`, `README_GHIDRA.md` | achados do binário do Suite (corroboram §13) |
| **Proxy** | `build_proxy.py`, `midi_proxy.c`, `forwarders.def`, `winmm.def`, `winmm.dll` (+`suite_local/`) | instrumentação do Suite oficial |
| **Catálogo** | `effect_catalog.csv`, `build_catalog.py`, `parse_*.py` | 909 slots catalogados |
| **Strings/extração** | `extract_strings.py`, `exe_strings.txt`, `fw_strings.txt`, `rtf_text.py`, `pdf_text.py`, `manual_streams.txt`, `release_note.txt`, `screens.html` | matéria-prima textual |
| **IR** | `gen_test_ir.py`, `test_ir_*.wav` | IRs sintéticos p/ teste |
| **Material extraído** | `nsis_app/` (read-only), `driver_ext/`, `suite_local/` | não indexar; ver `.codebuffignore` |

### `files/` — artefatos oficiais de entrada (não modificar)
Instaladores, firmware V2.1, manual, driver, DebugView, screenshots,
`patches/*.prst` (biblioteca) e `prompt_inicial.md` (briefing original).

### Raiz do repo — governança
`SECURITY.md` (política de segurança/secrets/material proprietário),
`.github/ISSUE_TEMPLATE/` (bug, feature, descoberta de protocolo) +
`.github/PULL_REQUEST_TEMPLATE.md` (checklist do gate + R1–R4),
`.gitignore`/`.gitattributes` (o que nunca entra — README §7).

### Infra do agente
`knowledge.md` (estado vivo + armadilhas), `.codebuffignore`,
`.agents/skills/{proxy-build,capture-analyze,new-session,spec-baseline,protocol-validate,core-dev,docs-sync,rust-practices}/SKILL.md`,
`scripts/add_cargo_path.ps1` (fix do PATH do cargo, HKLM).

## 4. Convenções de documentação

1. **Todo documento tem um banner de status** nas primeiras linhas: ✅ atual /
   📜 histórico / ⚠️ parcial — e a data da última revisão.
2. **Descoberta nova de protocolo** → entra no `PROTOCOL.md` (com evidência) +
   no `protocol_golden.json` (via `build_golden.py`) + valida com
   `validate_golden.py`. Nunca só conversa.
3. **Mudança de status de marco** → `BLOCKERS.md` (matriz) + `knowledge.md`
   (estado vivo) + este índice (se criar/mover documento).
4. **Documento obsoleto não é apagado**: ganha banner 📜 HISTÓRICO apontando
   para o substituto.
5. Nomenclatura: docs de referência em `docs/`; produtos de análise em
   `analysis/`; anything fora disso é material de origem (`files/`).

## 5. Pendências de documentação (pequenas, não bloqueiam)

- ~~Tabela de TIPOS (`12001002`)~~ ✅ **FECHADO 28/09**: era a tabela dos 20
  User IRs — layout decifrado em §13.12 (nome 32B + CRC32; vazio = ppIRCRC
  do .prst); pendência menor: CRC de slot ocupado e byte [32]
- ~~Schema do `11000007`~~ ✅ **FECHADO 28/09**: 50B sempre zeros nas 3 amostras
  (provável campo reservado ppAuthor/ppNotes) — documentado em §13.12
- Blob de IR no device (campo 0x00BC/0x00B4, alinhamento int24) → §13.7
- Semântica fina de ppEXP1/ppCtrl (expCode) → §13.9
- Capturas G3–G6 (globals/BPM, knob físico, footswitch) → `capture_gaps.md`
