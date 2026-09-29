---
name: new-session
description: Prepara e conduz uma sessão de captura MIDI com o usuário (Suite instrumentado), do setup à análise
metadata:
  category: workflow
---

# Sessão de captura GP-100 (roteiro padrão)

Use quando o usuário pedir para capturar tráfego novo (sessão N).

## Preparação (agente, antes de instruir o usuário)
1. Confirmar proxy atualizado: `md5sum analysis/winmm.dll analysis/suite_local/winmm.dll` (iguais?).
2. Limpar log antigo: `rm -f "$LOCALAPPDATA/Temp/midi_trace.jsonl"`.
3. Salvar captura anterior em `analysis/captures/` se ainda não estiver.

## Instruções ao usuário (sempre nesta ordem)
1. **Fechar TODAS as instâncias do GP-100 Suite** (Gerenciador de Tarefas) —
   a trava de instância única fecha a nossa silenciosamente.
2. Pedaleira no USB e ligada.
3. Abrir `D:\GP-100 app\analysis\suite_local\GP-100.exe` (NUNCA a de Program Files).
4. Executar as ações da sessão com **5–10s de pausa** entre cada (separa no timeline).
5. Editar SEMPRE pela UI do Suite — knobs físicos não geram SysEx.
6. Fechar o app normalmente e avisar "pronto".

## Ações conhecidas e o que calibram
- Trocar preset pela UI → transação de scan/paginação (§13.10)
- Editar 1 knob com swing 0↔99 → pinar knob→(página, offset) — É O OBJETIVO DA SESSÃO 3
- Salvar (Write) → sequência de save (§13.4)
- Importar IR → upload nibble-expanded (§13.7)

## Pós-sessão
Invocar a skill `capture-analyze`; documentar descobertas em docs/PROTOCOL.md §13.x
com evidência hex, e atualizar "Estado vivo" no knowledge.md.
