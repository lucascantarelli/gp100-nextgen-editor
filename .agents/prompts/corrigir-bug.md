# Prompt: corrigir um bug

> **Entrada:** `#N` (issue `🐞 Bug`) ou descrição do sintoma · **Saída:** PR com
> o fix + teste de regressão que reproduce o bug antes do fix.
> **Skills:** `testing` (onde o teste vive), `hardware-dev` (se o sintoma é do
> aparelho), `core-dev`/`ui-ux-practices` (camada), `github-flow` (ciclo).

```text
Você é o agente do GP-100 NextGen Editor. Corrija o bug descrito em #N (ou no meu relato).

DIAGNÓSTICO (antes de mexer):
1. Reproduza: comando/teste/e2e que falha. Sem reprodução escrita, não há fix — há palpite.
2. Localize a camada (core Rust / shell Tauri / front / CI / doc) e a fonte da verdade
   violada (GP100_DEVICE.md, PROTOCOL.md §13, UI_TEST_PLAN.md, DECISIONS.md).
3. Se o sintoma envolve o aparelho real, aplique a premissa e o procedimento de campo da
   skill `hardware-dev` antes de concluir quem erra (app × aparelho); divergência = issue
   `protocolo`.

FIX:
4. Teste de regressão PRIMEIRO: escreva o teste que reproduz (vermelho), depois o fix
   (verde). O teste fica — é a garantia contra volta.
5. Corrija a CAUSA (não o sintoma): sem skip de assert, sem espera mágica, sem "consertar"
   expectativa para o comportamento errado. Se a correção muda comportamento de propósito,
   isso vira decisão (ADR) e comunicação, não efeito colateral.
6. Se o bug é sintoma de dívida maior, registre a issue de dívida e deixe o fix mínimo aqui.

FECHAMENTO: ciclo completo da skill `github-flow`; na PR, a tabela Antes→Depois diz o QUE
mudou de comportamento e a PROVA (teste que falhava e agora passa). docs-sync se afetou
doc/estado.
```
