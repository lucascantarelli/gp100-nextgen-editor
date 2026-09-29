---
name: proxy-build
description: Regenera, compila e valida o proxy winmm.dll sniffer do GP-100 Suite (Rota 1 de captura)
metadata:
  category: build
---

# Build do proxy winmm.dll

Use esta skill sempre que precisar rebuildar o proxy após editar
`analysis/midi_proxy.c`, `analysis/forwarders.def` ou `analysis/winmm.def`.

## Comando único (nunca invocar zig diretamente)
```bash
uv run python analysis/build_proxy.py
```
O script: gera `forwarders_impl.c` + `midi_proxy_build.c` a partir de
`forwarders.def` (macros STUB) e `midi_proxy.c` (marcador `__FORWARDERS_INCLUDE__`),
cruza winmm.def ↔ C ↔ exports reais (SysWOW64) ↔ imports do GP-100.exe,
compila com zig (via **ziglang do venv uv** — sem dependência de python global
desde 28/09) e valida exports com pefile.

## Pós-build obrigatório
```bash
cp analysis/winmm.dll analysis/suite_local/winmm.dll
```
(senão o usuário testa a DLL velha)

## Regras
- Hooks de captura: `midiOutShortMsg`, `midiOutLongMsg`, `midiInAddBuffer`, `midiInOpen` (trampolim de callback p/ tráfego de entrada). Tudo com `__attribute__((used))`.
- Log em `%TEMP%\midi_trace.jsonl`, formato JSONL: `{"ts","pid","dir","len","hex"}` — linha termina com `"}"` (bug da aspa já corrigido; loaders antigos reparam logs velhos).
- `resolve()` com fallback no-op: nunca derrubar o host.
- Sanity check rápido: abrir `analysis/suite_local/GP-100.exe` (fechar TODAS as instâncias antes — instância única mata a nova silenciosamente) e conferir `"proxy loaded"` no log.
