---
name: capture-analyze
description: Copia o log MIDI do %TEMP%, repara JSON se preciso e roda os decoders da captura GP-100, salvando em analysis/captures
metadata:
  category: analysis
---

# Análise de captura MIDI (GP-100)

Use quando o usuário terminar uma sessão de captura no Suite instrumentado.

## Passos
1. Copiar o log:
```bash
cp "$LOCALAPPDATA/Temp/midi_trace.jsonl" analysis/captures/sessionN.jsonl
```
2. Rodar os decoders (venv):
```bash
uv run python analysis/decode_wire.py analysis/captures/sessionN.jsonl
uv run python analysis/check_session2.py analysis/captures/sessionN.jsonl
```
3. Para uploads de IR: `analysis/extract_ir_upload.py` (gera `captures/ir_slot*.bin`).

## Regras de interpretação (não reinventar)
- Trim no **1º F7** de cada mensagem (buffers MIM_LONGDATA têm cauda stale).
- F7 embutido em respostas longas = paginação, NÃO fim de mensagem.
- Header de fio: `F0 21 25 7F 47 50 2D 64 | FUNC | ADDR(4B BE) | data | F7`
  (0x11=read req, 0x12=dados/write). Docs: PROTOCOL.md §13.
- Log pode ser do proxy velho (aspa faltando no hex): os loaders reparam sozinhos.
- Formas conhecidas = `KNOWN_SHAPES` em check_session2.py; formas NOVAS são o sinal.

## Timeline
Sempre plotar histograma de eventos por janela de 5s: as pausas manuais do usuário
separam as fases da sessão (scan / import / save).
