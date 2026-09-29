# Gaps de captura (G1–G6)

> ⚠️ Status: parcial — G1/G2 fechados; G3–G6 opcionais (não bloqueiam o gp100-core).
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
