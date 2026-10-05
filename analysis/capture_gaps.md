# Gaps de captura (G1–G6) + G7–G9

> ⚠️ Status: parcial — G1/G2 fechados; G7–G9 abertos pela #23; G3–G6 opcionais
> (não bloqueiam o gp100-core).
> Roteiros e regras de sessão: `.agents/skills/new-session/SKILL.md`.
> Mapa geral: `docs/INDEX.md`.

## G1. Mapa knob→offset (sessão 3) — ✅ RESOLVIDO (não é offset, é semântico)
- Resultado: writes de knob vão para `10 [slot 1..9] 00 02` com payload
  nibble-expandido `[effectCode BE][ctrl][00][float32 LE]` — ver PROTOCOL.md §13.11
  e analysis/knob_map.json (89 edits, 14 grupos, 13/14 no range do dicionário).
- Exceção descoberta: CAB/Mic = controle oculto (não está no parameters.json).

## G2. WRITE_VERIFIED — editor→hardware (item 11 do BLOCKERS) — ✅ FECHADO
- Sessão 4 (§13.12): ciclo de save capturado = metadados 11xx (nome ASCII) +
  ops 00020000 (op 0/1) + re-sync 11000008 + 12000001; SEM readback 13xx.
- CONFIRMADO EM CAMPO (28/09/2026): o slot de destino recebeu o preset com os
  valores editados da sessão 3 (AMP Gain ~99) ⇒ save persiste o ESTADO AO VIVO.
- Consequência: WRITE_VERIFIED=true para knob set (§13.11), save (§13.12) e
  upload de IR (§13.7). Fluxos novos continuam exigindo captura própria.

## G7. Resposta completa ao `13000000` (dump de boot) — ❌ ABERTO (#23)
- **O maior frame do protocolo, e nunca visto inteiro.** O proxy (Suite) corta
  a linha em 256 bytes; as duas capturas têm o `IN 13000000` truncado nesse
  ponto, sem `F7` final.
- Por isso o golden v1.0 tinha um `push 13000000` com `const` de 242 bytes: o
  **prefixo** da captura cortada. A v1.1 removeu o template em vez de repetir a
  mentira (R1).
- **Fecha com:** o `--log` do `gp100-cli`, que grava o frame inteiro, e o juiz
  `analysis/validate_core_capture.py`. Roteiro: `docs/H3_CHECKLIST.md` §2
  sessão 1.

## G8. Resync do save (o lado IN) — ❌ ABERTO (#23)
- A prova D do `validate_golden` dizia `77/77`. **63 dessas mensagens eram
  fabricaradas**: `build_golden.trim()` cortava no primeiro `f7`, que aqui era
  um **byte de dado** dentro de um buffer `ff`, e o `body()` devolvia
  `010e000000000000000000000000` — 14 bytes, exatamente a forma de um registro
  de usuário, que passava na regra.
- Hoje a prova D é `14/14`: só o lado OUT (derivado do `.prst`), que é
  byte-a-byte. O lado IN (resync `11000008`, resync `12001002`, status
  `12000001`) é gap.
- **Fecha com:** `gp100-cli … save … --log` (precisa da feature `write-verified`
  do #22, ou seja: H2 tem que passar primeiro) e o juiz. Roteiro:
  `docs/H3_CHECKLIST.md` §2 sessão 3.

## G9. Baseline derivada do gp100-core — ❌ ABERTO (#23)
- É o DoD literal do H3 e era **impossível**: `build_golden` aceitava só o
  schema do Suite (`out_long`/`in_long` + `hex`) e carregava **0 eventos** de um
  log do `gp100-core` (P4: `out` + `func`/`addr`/`data`).
- **Resolvido pela maquinaria** (`wirelog.py`, `--log` com `t`,
  `validate_core_capture.py`, `baseline.py`). **Falta a sessão de campo** que
  produz a captura e o bump para v1.2.

## G3. Banco/BIOS/global — pouco explorado
- Trocar de banco (se houver), abrir settings globais (drum/BPM/tuner), mudar BPM da UI
- Identifica endereços fora do bloco 13xx

## G4. Live/physical controls
- MEXER knob FÍSICO com o Suite aberto (1 knob, swing lento) → descobrir se há
  notificação device→host (na sessão 1 não houve, confirmar com scan parado)

## G5. Footswitch/EXP
- Acionar footswitch e mexer expression pedal com o Suite aberto → eventos em tempo real?

## G6. Firmware/dados de fábrica
- Ler tela de info de firmware no Suite (se expuser SysEx de versão)
- SEM tocar em firmware update (política de segurança)
