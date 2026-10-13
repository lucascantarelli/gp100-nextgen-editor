---
name: hardware-dev
description: Trabalho com a GP-100 REAL — consultar o aparelho antes de deduzir, gate de campo (leitura livre, escrita só pós-gate H2 + write-verified), anti-brick (ADR-5/ADR-10), evidência por wire log e a fila de lacunas de leitura. Use em toda tarefa que toque comportamento do device, boot, sync, escrita de preset/IR ou decisão que o mock não responde.
metadata:
  category: hardware
---

# Desenvolvimento com o aparelho real (Era Hardware)

Complementa `device-reference` (o QUE o aparelho é — vocabulário e
funcionalidades) e `core-dev` (como o código é estruturado). Em conflito,
`docs/GP100_DEVICE.md` e `docs/DECISIONS.md` mandam.

## Regra zero — o aparelho está ligado e é consultável

A GP-100 real está ligada nesta máquina. **Antes de deduzir comportamento do
mock, perguntar ao owner ou inventar hipótese: MEÇA.** Leitura pura:

```bash
cargo build --release -p gp100-cli --features real-device
./target/release/gp100-cli.exe --real --i-know-what-im-doing <subcomando>
```

- `--real --i-know-what-im-doing` é a trava de leitura (política VISION §7).
  Sem ela o CLI fala com o `MockDevice`.
- **Suite oficial FECHADO durante a sessão** (occupancy de MIDI): porta
  aberta por outro programa = `OpenFailed`. E logo após abrir/conectar, o
  Windows **reenumera** o device — o 1º `open` pode falhar com "nenhuma porta
  MIDI"; retry em ~5s resolve (armadilha registrada no `knowledge.md`, seção
  "campo e captura").
- Toda varredura troca o patch no display da pedaleira — avise o owner antes
  de escanear (efeito colateral visível, não é bug).
- O que o aparelho NÃO responde em leitura (sequência temporal, save, upload)
  entra como **gate de campo** com roteiro — é o que o H1/H2/H3 faziam.

## Leitura × escrita (as duas camadas, nunca misturar)

| | Leitura | Escrita |
|---|---|---|
| Feature Rust | `real-device` (default **ON** no app desde #165; OFF no core/CLI) | `real-device,write-verified` |
| Trava | nenhuma além do flag do CLI | **ADR-5**: trava no TRANSPORTE — sem a feature, o binário imprime os frames e não envia; sem o gate H2 passado, a feature não se liga |
| Quem pode | qualquer sessão | só build explícito pós-gate H2, com o roteiro `docs/H2_CHECKLIST.md` |

**Proibido** abrir uma terceira camada (env var, subcomando novo, bypass). Se
uma feature "precisaria" de escrita para ser desenvolvida, isso vira issue +
ADR, não flag ad-hoc (regra 8 da skill `core-dev`).

## Anti-brick (por que as travas existem)

O firmware da GP-100 **asserta e derruba o pedal** em escrita fora de faixa
(ADR-10, risco #110 do ROADMAP). Por isso:

1. **Nada escreve no boot.** O boot é leitura; a sessão de escrita é separada,
   depois de tudo validado.
2. **`write-verified` é feature de compilação**, não runtime: o teste
   `escrita_verificada_segue_a_feature` existe para provar que a trava segue a
   build e não um default silencioso.
3. Escrita real = ADR-5 + gate H2 + roteiro assinado. O binário de leitura que
   sai no zip do campo **não tem** flag que abra escrita.

## Evidência: wire log e o que vale como prova

- O CLI grava wire log com timestamp (`t`, em ms) — sem relógio não há casamento
  de transação. Anexe o log à issue/PR; hex solto em conversa não é evidência.
- **Linha sem `F7` final não é frame** (lição da baseline v1.1): buffer truncado
  ou `ff` de cauda não vira dado. Interpretação de fio é `docs/PROTOCOL.md` §13.
- Fatos sobre o aparelho entram em `docs/GP100_DEVICE.md` (fonte canônica) com
  data e método de medição; o `MANUAL_COVERAGE.md` registra o que já foi
  coberto/testado; `PROTOCOL.md` §13 registra a FORMA do fio.

## Fila de lacunas (não deduzir, encher a fila)

O device TEM coisas que o app ainda não lê do fio: configs globais, `12001012`
(5 entradas, semântica aberta) e o banco USER de presets
(`docs/GP100_DEVICE.md` §6). Cada lacuna entra por: **evidência → modelo +
guard + teste → só então UI**. Nunca invente endereço de leitura nem exponha
recurso que o fio não sustenta.

## Checklist de uma tarefa de hardware

1. `docs/GP100_DEVICE.md` lido (vocabulário: User Patch ≠ User IR ≠ Setlist).
2. Dúvida de comportamento respondida no aparelho (leitura) ou registrada como
   gate de campo (escopo/roteiro).
3. Evidência anexada (wire log/relatório) e fato documentado na fonte certa.
4. Teste de regressão acompanha (skill `testing`); escrita só com as travas.
5. Divergência app × aparelho = **para o fluxo** e abre issue `protocolo` —
   nunca "consertar" o app com palpite.
