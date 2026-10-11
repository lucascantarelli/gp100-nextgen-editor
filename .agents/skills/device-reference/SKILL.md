---
name: device-reference
description: Contexto canônico da pedaleira Valeton GP-100 real — funcionalidades, comportamentos, vocabulário (User Patch × User IR × Setlist) e o que ainda não lemos do hardware. Consultar ANTES de modelar recurso, traduzir label da UI, escrever teste que confere valor, ou decidir product feature.
metadata:
  category: knowledge
---

# A pedaleira real GP-100 (contexto sempre em memória)

Use em **qualquer** tarefa que toque funcionalidade, comportamento, vocabulário
ou produto da pedaleira. A fonte canônica é **`docs/GP100_DEVICE.md`** — leia-o
antes de assumir qualquer coisa sobre o aparelho.

## Regras (não negociáveis)

1. **User IR ≠ User Patch ≠ Setlist.** User **Patch** = preset do banco USER
   (99, `P01–P99`). User **IR** = resposta de impulsão de cabine (20 slots).
   **Setlist NÃO EXISTE na GP-100** — o que o repo chama de "setlist" é a etapa
   de boot que lê `12001012` (5 entradas), semântica ainda não decifrada
   (`GP100_DEVICE.md` §5#1). Nunca exponha "setlist" como recurso do produto.

2. **O aparelho manda (ADR-13).** Toda regra de "padrão" nasce de byte OBSERVADO
   (captura + campo), nunca de suposição de fábrica, snapshot de análise ou
   `all.prst`. O golden é spec de **FORMA**; cada espaço de **VALOR** ganha o
   próprio modelo (precedentes: knobs #110, pp #148). Antes de deduzir
   comportamento do mock, **meça no aparelho real** (está ligado nesta máquina;
   CLI de campo `--real --i-know-what-im-doing`, leitura pura).

3. **Números reais > marketing.** "150 effects"/"100 patterns" é marketing; o
   firmware medido tem 185 algs e 87 ritmos de drum. Use o número do firmware.

4. **Lacunas de leitura não se deduzem.** Configs globais, `12001012` e o banco
   USER de presets o device TEM mas o app ainda não lê do fio (`GP100_DEVICE.md`
   §6). Cada uma entra por captura (skill `capture-analyze`), vira modelo +
   guarda + teste, ANTES de a UI expor. Nunca invente endereço de leitura.

## O que é nosso (inovação, fora do manual)

A app é expansão do editor oficial: biblioteca versionada, Tone Match, gestor
NAM, Laboratório de IRs, cloud sync, **Live Mode** (o "setlist" que o hardware
não tem — decisão do APP), export universal, A/B blind, ponte DAW. Detalhe em
`docs/VISION.md` §9 e `docs/MANUAL_COVERAGE.md` §8.

## Onde cada coisa mora

- Comportamento/funcionalidade da pedaleira → `docs/GP100_DEVICE.md`
- Cobertura manual×UI (o que já implementamos/testamos) → `docs/MANUAL_COVERAGE.md`
- Protocolo/fio (endereços, layouts decifrados) → `docs/PROTOCOL.md` §13
- Decisões estruturais → `docs/DECISIONS.md` (ADR-1..13)
- Fila do dado real (o que falta ler do hardware) → `docs/GP100_DEVICE.md` §6 + `docs/audit_era_real_2026-10-07.md`
