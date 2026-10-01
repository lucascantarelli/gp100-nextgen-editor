# Manual V1.8 vs Firmware V2.1 — Afinador (TUNER)

## Fontes

> ⚠️ **Material proprietário (não versionado).** O PDF do manual oficial da
> Valeton e a sua transcrição completa **não** entram no repositório (política
> do README §7) — são regeneráveis localmente com `analysis/manual_*.py`.
> Versionado aqui: apenas esta NOTA de análise (excertos curtos + conclusões).

- **Manual V1.8** (local): `analysis/manual_v18.pdf` (GP-100_Online Manual_EN_Firmware V1.8, 2022-12-06)
- **Extrato do manual** (local): `analysis/manual_v18.txt` (páginas 1–28 extraídas)
- **Firmware V2.1** (local): `analysis/fw_strings.txt` + decompilação em `analysis/*.py`

---

## O que o manual V1.8 diz sobre o TUNER (página 7)

### Entrada no modo tuner

> "Press and hold both footswitches at any time to enter the tuner mode."

Dois footswitches segurados simultaneamente entram no modo tuner.

### Display do tuner (p.7)

> "In tuner mode, the LED screen will display the tuning interface.
> When you pluck a string, the note will appear in the center.
> Left of center is flat, and right of center is sharp.
> As you tune your instrument towards the middle,
> the color of the scale will change from red (out of tune)
> to yellow (near pitch) to green (in tune)."

Resumo visual:
- **Nota no CENTRO** do display
- **Flat (♭)** à esquerda
- **Sharp (♯)** à direita
- **Escala de cores**: vermelho → amarelo → verde conforme a afinação melhora

### REF PITCH (p.7)

> "Quick access knob 3 adjusts the pitch calibration (REF PITCH),
> ranging from 435Hz to 445Hz. Standard pitch is set at 440Hz."

Range: 435–445 Hz, padrão 440 Hz.

### Modos do tuner (p.7)

> "Quick access knob 1 lets you select the tuner mode from
> Bypass (for dry signal through), Thru (for effect signal through)
> or Mute (for silent tuning)."

Três modos:
1. **Bypass** — sinal seco (dry signal through)
2. **Thru** — sinal com efeito (effect signal through)
3. **Mute** — silencioso para afinação (silent tuning)

### Saída do tuner (p.7)

> "You can exit the tuner either by pressing any footswitch
> or by pressing the EXIT button."

---

## Firmware V2.1 — strings relevantes do binário

Do `analysis/fw_strings.txt` e decompilação:

### TunerMode<=2 (assert do firmware)

```
TunerMode<=2
```

O firmware valida que o modo do tuner está sempre em 0..2 (3 modos).
Isso confirma os 3 modos do manual: Bypass (0), Thru (1), Mute (2).

### Assert de frequência válida

```
(Freq>=435) && (Freq<=445)
```

O firmware rejeita frequências fora do range 435–445 Hz,
confirmando o REF PITCH do manual.

### Debug strings

```
TUNER:
Encode:
Key:
```

Presentes no binário, indicando que o tuner grava logs de:
- Entrada do tuner
- Codificação/processamento
- Nota detectada (Key)

### Confirmação nos strings do firmware

```
E44
```

Provavelmente referência a E4 = 440 Hz (padrão de afinação).

---

## Comparação: V1.8 vs V2.1

| Aspecto | Manual V1.8 | Firmware V2.1 | Status |
|---------|-------------|---------------|--------|
| Entrada no tuner | 2 footswitches segurados | (a confirmar no bin) | Esperado |
| Display da nota | Centro, flat esq, sharp dir | (UI independente) | ✅ Implementado |
| Escala de cores | Vermelho→Amarelo→Verde | (UI independente) | ✅ Implementado |
| REF PITCH | 435–445 Hz, padrão 440 | Assert (Freq>=435) && (Freq<=445) | ✅ Implementado |
| Modos | Bypass/Thru/Mute (3 modos) | TunerMode<=2 (3 modos) | ✅ Implementado |
| Saída do tuner | Qualquer footswitch ou EXIT | (a confirmar) | — |
| LED próprio |Não mencionado | (device tem LED integrado) | ✅ Implementado |
| Monitor contínuo | Não mencionado | UI pode ficar sempre ativo | N/A |

---

## Decisões de implementação da UI

### 1. Display sempre visível (não colapsa)

**Motivo**: O afinador pode ficar ativo durante o uso da pedaleira
para monitorar a afinação em tempo real.

**Decisão**: O TunerPanel ocupa lugar fixo no cabeçalho do palco
(ao lado do display LED do patch e do cadeado de mover),
sempre visível.

### 2. LED próprio para o afinador

**Motivo**: Evitar conflito com o LED do display do patch
(pp/nome/preset).

**Decisão**: O TunerPanel tem seu próprio LED (data-tuner-led):
- Cinza: monitor desligado
- Âmbar: monitor ligado, ouvindo, sem leitura
- Verde: nota no ponto (band ok)
- Vermelho: fora de afinação (band error)

O LED do patch segue intocado — zero conflito.

### 3. Estilo do LED do palco reutilizado

**Motivo**: Evitar nova implementação de estilo.

**Decisão**: O TunerPanel usa a mesma caixa de hardware:
- Fundo `#0a0d10` (mesmo do LED do palco)
- Borda `#1d242c`
- Sombras internas
- Mono âmbar para textos/controles
- Tokens do tema para cores da escala (var(--ok)/var(--warn)/var(--error))

### 4. Funcionalidades no painel

**Decisão**: O TunerPanel expõe:
- **Monitor on/off**: botão "♪ monitorar" (aria-pressed)
- **Modo**: Bypass/Thru/Mute em ciclo (botão com aria-label)
- **REF PITCH**: slider 435–445 Hz com display numérico
- **Demo**: botão "▶ demo" que alimenta o motor real com senoide sintética

### 5. Botão mover vira cadeado 🔒/🔓

**Motivo**: Reutilizar o espaço onde era o botão mover para
uma trava visual mais intuitiva.

**Decisão**:
- Botão com ícone 🔒 (trancado) / 🔓 (destrancado)
- Tooltip: "Destravar para arrastar pedais (protege o ajuste dos knobs)"
- Posicionado **abaixo** do display do patch (número+nome+estilo)
- Alinhado com o cabeçalho do palco

---

## Notas técnicas do motor de afinação

### `src/tuner/pitch.ts`

- **f0 por autocorrelação** no domínio do tempo (mais confiável que FFT para guitarra)
- **Interpolação parabólica** do pico para precisão sub-amostra
- **Gate de RMS**: silêncio = sem leitura (honesto)
- **Janela**: 2048 amostras cobre E2 (82.4 Hz) em 44.1 kHz
- **Conversão nota/cents** contra REF PITCH (435–445 Hz)
- **Bandas de cor**: verde (|cents|≤3), âmbar (|cents|≤15), vermelho (acima)

### Limiares calibrados nos testes

- `BAND_OK_CENTS = 3` (verde: no ponto)
- `BAND_WARN_CENTS = 15` (âmbar: perto)
- Acima: vermelho (fora)

---

## Referências

- Manual V1.8: `analysis/manual_v18.pdf`, `analysis/manual_v18.txt`
- Extração: `analysis/manual_tuner.py`, `analysis/manual_decode.py`
- PDF original em código: este arquivo + `analysis/manual_v18.pdf`
- Firmware strings: `analysis/fw_strings.txt`
- Motor de afinação: `packages/app/ui/src/tuner/pitch.ts`
- Painel: `packages/app/ui/src/components/TunerPanel.tsx`
- Componente: `packages/app/ui/src/components/EmptyBoard.tsx`
