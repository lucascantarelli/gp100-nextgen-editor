# 🎨 UI_DESIGN — Design tokens e sistema visual (FONTE ÚNICA de valores)

> **Status:** ✅ atual (M1.0) · **Última revisão:** 2026-09-29 · **Complementos:** [docs/UI_PLAN.md](UI_PLAN.md) (arquitetura/issues) · [.agents/skills/ui-ux-practices/SKILL.md](../.agents/skills/ui-ux-practices/SKILL.md) (gates/estilo)
>
> Este documento fixa os VALORES (escala, cores, rácios de contraste medidos,
> tipografia, motion). Os valores vivem em `ui/src/design/tokens.ts` +
> `ui/src/design/design.css` — este doc explica o PORQUÊ e as regras de uso.
> Mudou um valor aqui → muda o token → muda o teste de token (os 3 juntos).

## 1. Escala de espaçamento — Fibonacci sobre base 4

Rationale: a escala de Fibonacci cresce ~1,6× a cada passo (ritmo harmônico,
evita o "tudo é igual" do 4px puro e o salto grande do 8pt puro), e a base 4
mantém alinhamento com metades do 8pt grid (badges/ícones pequenos).

| Token | px | F | Uso canônico |
|---|---|---|---|
| `--space-4` | 4 | F4 | respiro dentro de controles (ícone↔texto) |
| `--space-8` | 8 | F6 | gap de listas, padding de botão pequeno |
| `--space-12` | 12 | F7 | padding de inputs, gap de knobs |
| `--space-20` | 20 | F8 | padding de painéis, gap entre controles |
| `--space-32` | 32 | F9 | gap entre seções, respiro de cards |
| `--space-52` | 52 | F10 | respiro de página lateral |
| `--space-84` | 84 | F11 | herói/empty states |

Regras: (1) só estes valores em `padding/margin/gap` (lint bloqueia px cru);
(2) raio de borda usa a mesma escala (4/8/12); (3) espessura de linha (1/2px)
e tamanho de ícone ficam FORA da escala (são métricas, não espaços).

## 2. Cores e contraste (WCAG 2.2 AA medido) — paleta VALETON/PALCO

**Linguagem: "pedalboard ao vivo no palco".** Base = preto-quente de palco
(o produto é vendido em **preto/vermelho/lavanda** — as variantes de accent
descem dessas cores); âmbar = a cor de interface da linha GP (display/LCD).
Tema escuro é o base; light ("papel quente") nasce espelhado. Estado vivo
(LED verde piscando sutil quando conectado) reforça a sensação de pedal
ligado no palco.

Tema escuro ("palco"):

| Papel | Dark (palco) | Light (papel) | Uso | Rácio medido |
|---|---|---|---|---|
| `--bg` | `#141210` | `#f6f3ee` | fundo da app (preto-quente/papel) | — |
| `--bg-raised` | `#1d1a17` | `#ffffff` | painéis/cards (grafite) | — |
| `--text` | `#eef0f2` | `#191512` | texto principal | **16,4:1** / **16,4:1** (AAA) |
| `--text-muted` | `#a89f92` | `#6a6157` | rótulos secundários (quente) | **7,2:1** / **5,5:1** (AA) |
| `--accent` | `#ffa938` | `#9a5200` | âmbar VALETON: ação primária, seleção, foco | **9,8:1**/**5,3:1** |
| `--ok` | `#3ecf8e` | `#0f6b44` | LED verde (conectado/ao vivo) | **9,4:1**/**5,9:1** |
| `--warn` | `#f5b544` | `#7a4d00` | aviso (dry-run ativo) | **10,3:1**/**6,6:1** |
| `--error` | `#ff6b6b` | `#a92222` | erro (com ícone+texto) | **6,7:1**/**6,5:1** |
| `--focus-ring` | `--accent` | `--accent` | contorno de foco | **3:1+ vs adjacente** (2.4.13) |
| `--accent-red` | `#e05252` | `#a92222` | variante VERMELHO (cor de produto) | **4,9:1**/**6,5:1** |
| `--accent-lav` | `#b9a7ff` | `#5b4bb0` | variante LAVANDA (cor de produto) | **8,9:1**/**(medir no uso)** |

As variantes `--accent-red`/`--accent-lav` são TEMAS escolhíveis pelo usuário
(como as cores do pedal), nunca alterando semântica (ok/warn/error fixos).

Regras: (1) NENHUM par fg/bg fora desta tabela pode ser usado em texto —
novo par = novo token + rácio medido aqui; (2) estado NUNCA só pela cor
(ícone + texto juntos); (3) foco: `outline: 2px solid var(--focus-ring)` +
`outline-offset: 2px` (WCAG 2.4.13: ≥2px e ≥3:1 vs adjacente); (4) alvo
clicável mínimo **32×32 px**.

## 3. Tipografia

| Token | Valor | Uso |
|---|---|---|
| `--font-ui` | "Segoe UI", Inter, system-ui | UI geral (nativa do SO) |
| `--font-mono` | Cascadia Code, Consolas, monospace | hex/valores de fio (dump) |
| `--text-xs` | 11px/1.45 | badges, atalhos |
| `--text-sm` | 13px/1.5 | UI padrão (knobs, listas) |
| `--text-md` | 15px/1.55 | corpo de texto corrido |
| `--text-lg` | 21px/1.3 | títulos de seção (×1,6 ≈ φ da base) |
| `--text-xl` | 34px/1.2 | herói/empty state |

Escala modular 1.125–1.25 a partir de 13px (legibilidade desktop). Texto
corrido ≥ `--text-md`; números monoespaçados SEMPRE em `--font-mono`
(valores alinham sem tremer).

## 4. Motion

- Transições: **120ms** (hover/press) e **200ms** (painéis/modais), easing
  `cubic-bezier(0.2, 0, 0, 1)`.
- **`prefers-reduced-motion: reduce`** desliga transições/animações (media
  query no `design.css`; teste de token garante a regra presente).
- Feedback de interação **<100ms** (resposta imediata percebida); progresso
  de operações longas SEMPRE visível (boot/upload).

## 5. Estados de tela (união TS canônica)

```ts
type ScreenState<D> =
  | { kind: "idle" }            // antes da 1ª ação
  | { kind: "loading"; from?: ScreenState<D> }
  | { kind: "empty"; hint: string }
  | { kind: "error"; message: string; retry: () => void }
  | { kind: "ready"; data: D };
```

Toda feature modela seu recurso com esta união (sem booleanos soltos
`isLoading`+`hasError`). Erro SEMPRE oferece ação (`retry`). Empty state
tem hint de próximo passo (nunca tela vazia).

## 6. Identidade: "pedalboard AO VIVO no palco" (princípios de UX)

A app deve dar a sensação de estar **com o pedal ligado no palco** — não de
dialogar com um driver. Princípios vinculantes para todas as telas da M1+:

1. **O palco é a cadeia.** O ChainEditor (9 slots) é a tela-fundamento,
   sempre acessível em 1 clique — como olhar para o pedal. Nada de
   navegação em árvore/menus profundos para chegar nele.
2. **Editar = girar.** Knob/slider afeta o valor imediatamente
   (fire-and-forget D4) — a edição É o feedback. Não existe "modo de
   edição" separado nem diálogo de confirmação para ajuste de parâmetro
   (confirmação só para escrita persistente — save/upload, §5 do UI_PLAN).
3. **Ao vivo = responsivo.** Toda ação tem resposta percebida <100ms
   (§4); boot/longas operações mostram progresso ao vivo, nunca congelam.
4. **LED de palco.** Conexão é um LED verde pulsando no topo (`.live-dot`)
   + texto — nunca um modal bloqueante. Device desconectado = LED apaga
   com explicação e ação, não tela de erro de sistema.
5. **Superfície de palco.** Fundo preto-quente (§2), painéis como módulos
   de pedal com sombra suave, âmbar Valeton para o que é ação/vivo. Nada
   de cinza corporativo: a app deve parecer um equipamento de música.
6. **Profissional, não infantil.** Brilho/glow com parcimônia (só no
   logo-dot e no LED); tipografia disciplinada (§3); hierarquia por
   espaçamento (§1), não por enfeite.
7. **Sem caminhos longos.** Regra dos 2 cliques: qualquer edição comum
   (knob, bypass, troca de efeito, IR slot, nome de preset) a ≤2 cliques
   da tela inicial. Revisão de UI rejeita fluxo mais fundo.

## 7. Checklist de review de UI (gate manual antes do PR)

- [ ] Nenhum valor de espaço/raio fora da escala §1 (lint + olho)
- [ ] Todo texto usa pares de contraste da §2 (novo par = token + rácio)
- [ ] Foco de teclado visível em TODOS os interativos (tab passou por tudo?)
- [ ] Estado nunca só pela cor; erro sempre com ação; empty sempre com hint
- [ ] Alvos clicáveis ≥32×32; janela 800×600 e ultrawide sobrevivem
- [ ] Strings via `t()` (i18n); datas/números via helper (locale)
- [ ] `prefers-reduced-motion` respeitado; `--font-mono` em valores de fio
