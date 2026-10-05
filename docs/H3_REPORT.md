# 🧊 H3_REPORT — Relatório do gate H3 (congelamento pós-captura)

> **Status:** 📝 template · preencher durante/após a sessão
> ([H3_CHECKLIST.md](H3_CHECKLIST.md)) e arquivar como evidência da issue #23.
>
> ⚠️ Abaseline desta sessão é **o contrato de todo o `gp100-core`**. Um bump
> errado aqui não quebra um teste: quebra a correspondência entre o que o
> aparelho faz e o que o software acha que ele faz, e o sintoma aparece em
> campo, muito depois.

## 1. Identificação da sessão

| Campo | Valor |
|---|---|
| Data/hora | ____ |
| Commit do repo no binário | ____ |
| sha256 do `gp100-cli.exe` | ____ |
| Build | `cargo build --release -p gp100-cli --features real-device` |
| Baseline de partida | v____ (`uv run python analysis/baseline.py show`) |
| H1 anterior | arquivado em ____ |
| H2 anterior | arquivado em ____ (necessário para a sessão 3) |

## 2. Sessões executadas

| # | Sessão | Arquivo de log | Frames | Veredito do juiz (exit) |
|---|---|---|---|---|
| 1 | dump de boot (`13000000`) | `analysis/h3/s1_boot/…` | ____ | ____ |
| 2 | varredura de presets | `analysis/h3/s2_scan/…` | ____ | ____ |
| 3 | save (escreve!) | `analysis/h3/s3_save/…` | ____ | ____ |

> **exit 0** = tudo coberto e batendo · **1** = divergência de payload (bug no
> core) · **2** = endereço fora da spec (captura nova) · **3** = o log não
> carregou (formato).

## 3. Gap 1 — resposta ao `13000000`

| | |
|---|---|
| Comprimento do payload observado | ____ bytes |
| A v1.1 tinha template para este endereço? | ____ (não tinha — veio da captura cortada) |
| O log do gp100-core gravou o frame INTEIRO? | ____ |
| Deriva template de vários exemplos? | ____ (se sim, quantos exemplos) |

> Se a resposta da última for "1 exemplo", **não** derivar template. Um padrão
> de um exemplo só é exatamente o que a v1.0 fez de errado.

## 4. Gap 2 — resync do save

| | Valor |
|---|---|
| ACKs de resync observados (func/addr/len) | ____ |
| Status pós-save (`12000001`?) observado? | ____ |
| A v1.0 "provava" isto com bytes fabricados? | sim |
| Verificação no display (do H2) | ____ |

## 5. Gap 3 — o golden a partir do gp100-core

| | Valor |
|---|---|
| `build_golden` leu a captura do core? | ____ (antes da #23: **0 eventos**) |
| `validate_golden` final | ____ % |
| `make_fixtures` paridade | ____ % |
| Baseline antes → depois | v____ → v____ |
| Motivo do bump (o MESMO que vai no §13.14) | ____ |

> Se o golden **não** mudou: **não faça bump.** O gate recusa motivo vazio de
> propósito; "rodei o script e não mudou" não é bump.

## 6. Endereços fora da spec (`exit 2`)

| # | func/addr | Nº de exemplos | Tem padrão derivável? | Destino |
|---|---|---|---|---|
| 1 | | | | ADR #__ · ou "observado, não caracterizado" |

> Endereço novo **não** entra no golden por dedução. Ou tem exemplos suficientes
> para separar `const` de `var`, ou fica de fora documentado.

## 7. Divergências de payload (`exit 1`)

| # | func/addr | Esperado | Obtido | Tipo |
|---|---|---|---|---|
| 1 | | | | protocolo · comportamento · bug do core |

- **protocolo** → fluxo R3 do [H1_CHECKLIST.md](H1_CHECKLIST.md) §6
- **comportamento** → issue nova; **não** se corrige mexendo no codec (R1)
- **bug do core** → issue nova no `gp100-core`, com o log como evidência

## 8. Fechamento

- [ ] **H3 OK** — baseline v1.2, `validate_golden` 100%, gaps fechados
- [ ] **H3 parcial** — registre o que passou; o que passou **não** habilita o
      que falhou
- [ ] **H3 BLOQUEADO** — nada de bump até a causa ser tratada

### Gaps que continuam abertos (e por quê)

| Gap | Por que continua aberto |
|---|---|
| | |

> Um gap que continua aberto é um resultado honesto. Deixar o golden afirmar
> algo sem evidência é o erro que a v1.1 consertou — e voltar a ele agora
> desfaz o trabalho inteiro.