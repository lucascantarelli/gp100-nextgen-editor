# 🎸 GP100_DEVICE — a pedaleira real: funcionalidades e comportamentos

> **Status:** ✅ atual · **Criado:** 10/10/2026 · **Fontes:** manual oficial Valeton
> GP-100 (firmware V1.8/V1.9 texto + V2.1 nas capturas), página do produto
> (valeton.net/product/gp-100), e as **nossas capturas S1–S4 + campo** (o que o
> fio prova). Onde a fonte externa e a medição local divergem, as duas aparecem.
>
> **Para que serve este doc.** É o **contexto canônico do comportamento da
> pedaleira**. Antes de modelar um recurso, traduzir um label da UI ou escrever
> um teste que "conferre" um valor, consulte aqui. A skill `device-reference`
> aponta para este arquivo. Regra da casa (ADR-13): **o aparelho manda** — tudo
> que aqui está marcado como medido vale mais que suposição de fábrica ou
> snapshot de análise.
>
> **Vocabulário importa.** User **IR** ≠ User **Patch** ≠ **Setlist** (que não
> existe). Ver §1 — essa separação já derrubou premissa nossa.

---

## 1. Os três conceitos que não se confundem (e já confundimos)

| Conceito | O que é | Capacidade | Já lemos do fio? |
|---|---|---|---|
| **User Patch** | Um preset gravado no banco USER do aparelho (tom completo) | **99**, slot `P01–P99` | ✅ parcial: o scan de boot lê os 198 (99 F + 99 U) |
| **User IR** | Um arquivo de resposta de impulsão (cabine) de terceiros | **20**, slot 0..19 | ✅ tabela `12001002` (PROTOCOL §13.12) |
| **Setlist** | **NÃO EXISTE na GP-100** | — | ⚠️ ver §5, divergência #1 |

O manual (V1.9) e a página oficial são unânimes: a GP-100 tem **198 presets =
99 user + 99 factory** e **20 slots de User IR**. Não há função "setlist"
documentada. Nossas capturas batem: o scan de boot percorre o espaço de pp
`0000..=0062` (fábrica) + `0100..=0162` (user) — 198 endereços distintos.

---

## 2. Presets e bancos

- **Dois bancos separados:** USER `P01–P99` e FACTORY `F01–F99` (1-based no
  display; o wire usa `0000..0062`/`0100..0162`). Fonte: manual §"two patch
  banks" + `analysis/captures/session{1..4}.jsonl` (796 selects).
- **Nome do preset:** gravado em **pg0 offset 2**, 12 bytes, terminado em NUL.
  O device **não zera o rabo** ao renomear (medido no H4/2026-10-09:
  `H2 TESTE\0` + `'0'` 0x30 stale de "It's GP100") — o objeto preset é o
  prefixo até o 1º NUL (ADR-13).
- **Edição ≠ gravação:** mudar um knob marca o patch com `*`; sair sem SAVE
  perde. SAVE escolhe o destino. (manual §Edit/Save)
- **Não há "mover preset" nativo** no app oficial — o Valeton Sort é
  ferramenta de terceiros (não verificado em primeira mão; behavor de usuário).

## 3. Cadeia de sinal: 9 módulos, um de cada tipo

Ordem padrão (o wire e o manual coincidem):

```
PRE → DST → AMP → NR → CAB → EQ → MOD → DLY → RVB
```

- **9 efeitos simultâneos, um de cada tipo** — não empilha 2 PRE nem 2 DST
  (review TDPRI; consistente com o layout de 9 páginas que lemos no boot).
- A ordem é **reordenável** no device (EDIT segurando PARA).
- **150 efeitos / 45 amp models / 40 cab IRs de fábrica** (manual + produto).
  Nosso `parameters.json` tem 185 algs / 639 controles (firmware V2.1 medido —
  pode diferir do count de marketing).

## 4. Menu GLOBAL (configurações de sistema)

O menu GLOBAL do device tem estas áreas (manual V1.9). **Nenhuma leitura em
hardware implementada hoje** — ver a lacuna #1 em §6.

| Área | Parâmetros documentados |
|---|---|
| **I/O** | Input Level (−20..+20 dB); **No CAB** L/R (bypassa CAB p/ amplitude) |
| **Tap Tempo Mode** | PRE/MOD/DLY sincronizam ao tap, cross-patch |
| **EXP Setting** | Target (até 3 params no pedal de expressão); EXP Range; VOL Range (PRE/POST); Calibrate |
| **USB Audio** | Rec Level; Monitor Level; Recording source (Dry/Effect) |
| **Footswitch Mode** | **PATCH** ou **STOMP** (cada FS controla 1–3 módulos) |
| **Global EQ** | on/off; low-cut/high-cut + 4 bandas paramétricas (não afeta USB out) |
| **Language** · **Factory Reset** · **About** | idioma; reset; firmware info |

**Não é global:** Noise Reduction (é módulo **NR** por preset), tuner bypass
(é opção de entrada do tuner, não menu global), display brightness (não
verificado), MIDI channel (**não existe** no manual — não afirmar).

## 5. Divergências a corrigir (a pesquisa de 10/10 expôs)

> Cada item: o que o repo afirma → o que a realidade mostra → o que fazer.
> Atualizado conforme corrigido (sai daqui e vira issue/commit).

- **#1 — "Setlist" é nome de etapa de boot, não recurso do produto.**
  `BootStage::Setlist` em `packages/core/src/session.rs` nomeia a **leitura de
  `12001012`** (5 entradas de 44B) no boot. O PROTOCOL.md §13.3 já hesita:
  *"provavelmente setlist/loja"*. O manual **não** tem setlist.
  → **Ação:** não expor "setlist" como conceito de produto na UI; renomear o
  estágio/label interno para o que o fio realmente lê (uma tabela de 5 entradas
  de estado — semântica fina pendente de medição) até saber o que é.

- **#2 — User IR ≠ User Patch na UI.** A aba "User" da biblioteca é o banco de
  **presets** do editor (SQLite local, #113) — correto quanto ao conceito, mas o
  rótulo pode sugerir que é o device. As User **IRs** (20 slots) são outro
  feature (§8/X9, Laboratório de IRs). Manter os dois nomes sempre distintos.

- **#3 — Contagem de efeitos.** "150 effects"/"100 patterns" é marketing; o
  firmware medido tem 185 algs e 87 ritmos de drum. Já decidido (MANUAL_COVERAGE
  D7): usar o número real do firmware, não o do marketing.

## 6. Lacunas de leitura do hardware (a fila do dado real)

Estas coisas o aparelho TEM mas o app ainda não LÊ do fio — ou lê e descarta.

| # | Lacuna | O que o device tem | Estado |
|---|---|---|---|
| 1 | **Configs globais** | Menu GLOBAL completo (§4) | 🔴 nenhum endereço de leitura no golden; precisa captura (G3–G6) antes de modelar |
| 2 | **`12001012`** | 5 entradas de 44B lidas no boot e **descartadas** | 🟡 lido e jogado fora; semântica não decifrada (não é "setlist", §5#1) |
| 3 | **User patches do device** | Banco USER `P01–P99` com boards | 🟡 o scan lê os 198 mas a UI User é SQLite local (#113/#150) |

> Regra ADR-13: **nada de deduzir endereço.** Cada lacuna entra por captura
> (skill `capture-analyze`), vira modelo + guarda + teste, antes de a UI
> expor. O golden é spec de FORMA; cada espaço de VALOR ganha modelo próprio
> (precedentes: knobs #110, pp #148).

## 7. O que somos NÓS (inovação, fora do manual oficial)

A app é uma **expansão** do editor oficial, não uma cópia. Recursos nossos
(detals em `docs/VISION.md` §9 e `MANUAL_COVERAGE.md` §8): biblioteca
versionada Git-like · Tone Match offline · gestor SnapTone/NAM · Laboratório de
IRs com CRC · cloud sync opt-in · **Live Mode** (telão BPM + troca por atalho/
MIDI — *isto é* o "setlist" que o hardware não tem: decisão do APP, não do
pedal) · export .prst↔JSON↔PDF · A/B blind · ponte MIDI/DAW · gain-staging.

---

## 8. Fontes

- Manual Valeton GP-100 firmware **V1.9** (Scribd host copy) + **V1.8/V2.1**
  (extração local `analysis/manual_streams.txt`, nem todo texto sai legível).
- [Página do produto Valeton GP-100](https://www.valeton.net/product/gp-100/)
  (99+99 presets, 20 IRs, editor PC/Mac, USB 2.0 Type-B).
- Nossas medições: `docs/PROTOCOL.md` §13 · `docs/protocol_golden.json` (39
  templates) · `analysis/captures/session{1..4}.jsonl` · campo 07/10 e 09/10
  (H4) · `docs/audit_era_real_2026-10-07.md`.
- Reviews de usuário (TDPRI, Reddit) — comportamento relatado, não medido;
  marcado como tal.
