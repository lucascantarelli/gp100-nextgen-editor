---
name: app-dev
description: Implementação do crate gp100-ui (packages/app/api) — a ponte Tauri 2 entre o front e o gp100-core, com o DeviceActor (dono único da Session), os commands de device/biblioteca/IR/tom, a sincronização bilateral com read-back (#164) e a política de escrita na porta (write_verified → escritaLiberada). Use ao tocar packages/app/api — actor, commands, DTOs, backend real/mock/Desligado, boot do app ou o evento de sync — mesmo que o pedido fale só de backend, IPC, o app não conecta ou a tela não atualiza.
metadata:
  category: development
---

# Desenvolvimento do gp100-ui (Tauri 2)

Use ao executar issue que toque `packages/app/api/` (crate **gp100-ui**), UMA por
vez; o ciclo issue→branch→PR→merge é a skill `github-flow` e o formato de execução
é o prompt `.agents/prompts/implementar-issue.md`. Complementa `core-dev` (o core
Rust) e `ui-ux-practices` (o front); em conflito, `docs/dev/DECISIONS.md` manda.

Este crate é a **ponte**: commands, actor e DTOs. Ele orquestra o `gp100-core` e
não reimplementa nada dele.

## O que NÃO entra aqui (a fronteira)

| Assunto | Dono | Motivo |
|---|---|---|
| Regra de protocolo / byte de fio | `gp100-core` (`core-dev`) | o crate do app não conhece endereço nem layout |
| Armazenamento (versão, seed, busca) | `gp100-library` | ADR-9 — a casca delega; o `rusqlite` não entra no MSVC |
| Política do aparelho e travas (ADR-5/ADR-10) | skill `hardware-dev` | aqui só se **consome** o sinal, nunca se recria |
| Geometria/tokens/estados de tela | `packages/app/ui` (`ui-ux-practices`) | o app Rust não decide layout nem texto |

Se você está prestes a escrever um `if` de endereço, de faixa de valor ou de
layout neste crate, o código está no lugar errado.

## Os dois eixos: backend e escrita (por compilação, nunca por env)

`packages/app/api/Cargo.toml` define duas features **separadas de propósito**:

- `real-device` — **default ON** (#165): `tauri dev`/`tauri build` abrem o
  `RealDevice`; `--no-default-features` gera um app **sem transporte** (estado
  `Backend::Desligado`, motivo próprio).
- `write-verified` — **espelha a do core** (ADR-5): sem ela, qualquer escrita é
  recusada **antes** do driver. A inversão de default (#165) **não** abriu escrita.

O `Backend` (`Real` | `Desligado`) viaja no spawn e vira `DeviceSnapshot.backend`
(`"real"`/`"none"`) — **o `DeviceInfo` nunca tem literal** (`REAL_DEVICE_GAP.md` §1).

**Não introduza uma env que troque backend** (`GP100_BACKEND` foi recusado): um
`.exe` que passa a falar com o que estiver na USB conforme a máquina tornaria a
abertura do app um vetor de escrita de surpresa. A escolha é a feature; o CLI
mantém a política explícita (`--real --i-know-what-im-doing` / `--mock-device`).

O sinal de escrita chega ao front em `DeviceInfo.write_verified`, e o front tem
**um único** ponto que o lê — `escritaLiberada()` em
`packages/app/ui/src/ipc/device.ts` (#126). Não reimplemente a política em
command nem em outro hook.

## O actor — a peça central (`packages/app/api/src/actor.rs`)

O `DeviceActor` é **dono único** da `Session` (D8 do ADR-6): a FSM é `&mut self`,
então compartilhar `Mutex<Session>` com a UI permitiria interleave de transações.
O actor roda UMA thread com uma fila `mpsc` de `Request`; cada command envia a
requisição + um canal de resposta e bloqueia no `recv` (o command síncrono do
Tauri roda fora da main thread — ADR-3 preservado).

- **Nunca toque a `Session` direto de um command** — entre pela fila.
- `Request` é o contrato da fila: `Info`, `Conectar`, `Boot`, `Board`, `Library`,
  `Select`, `SetParam`, `Globals`, `SetGlobal`, `ListIrs`, `UploadIr`,
  `UploadSnapTone`, `Save`, `Dump`, `DrainPushes`, `SyncNow`, `LogSession`,
  `LogStop`, `LogPath`, `Shutdown`.
- **Eventos** (`device://`): `progress` (boot — barra, nunca trava), `sync`
  (#164) e `push` (backlog D7). O front escuta na porta de IPC
  (`packages/app/ui/src/ipc/`).

### Sincronização bilateral (#164 / ADR-15)

`SyncStatus` = `idle | sync | pending | failed | offline` (o literal do fio saiu
de `SyncStatus::as_str`; o front não switcha em `Debug`). O read-back relê as 9
páginas pelo caminho do **boot** (`reload_paginas_correntes` — tudo
`WireKind::Read`) e **o aparelho vence**: `SyncEvent.changed = true` só quando as
páginas divergem do cache.

- `READBACK_DEBOUNCE = 800 ms` — uma **rajada** de knob vira UMA releitura
  (ADR-15: RTT mediano 3,05 ms no H1 → 9 páginas ≈ 27 ms; ~3% do fio numa rajada).
- `REFRESH_INTERVAL = 10 s` — a ÚNICA janela em que o app vê mudança feita no
  **pedal**: o knob físico não emite SysEx (poll, não notificação espontânea).

> Este read-back roda na mesma thread do `SetParam` — a latência do caminho de
> escrita é dívida aberta (**#239**); não aumente o trabalho dentro do loop sem
> pensar na fila.

## Boot e log de fio (`packages/app/api/src/lib.rs`)

- `abrir_backend()` monta o `Backend` e o actor; **sem aparelho o app sobe no
  estado `Desligado`** com `MOTIVO_DESLIGADO` legível e o botão "Reconectar"
  (`device_conectar`) refazendo o open — nunca o mock fingindo ser o aparelho (#150).
- O log de fio automático vale **só no `Backend::Real`** (`log_automatico`): sem
  aparelho não há fio a registrar. O nome leva carimbo (`nome_wire_log`) porque o
  `WireLogger` **trunca** ao abrir — nome fixo apagaria o log do incidente.
- O seed da fábrica usa `include_bytes!` do `files/patches/all.prst` embutido.

## Orçamento de tamanho e dívida

`actor.rs` tem **2702 linhas** (**#243**) e o `check_module_size.py` cobre **só o
front** (**#248**) — ou seja, não há freio automático aqui. Regra prática: **coorte
nova de `Request`/evento vira módulo próprio**; não cresça o `actor.rs`.
`commands.rs` (`packages/app/api/src/commands.rs`, 869 linhas) já é o segundo.

## Checklist de uma tarefa no crate do app

1. Ler a issue (DoD define "pronto"); branch/PR pela `github-flow`.
2. **DTO mudou?** Espelhe em `packages/app/ui/src/ipc/types.ts` (os DTOs são o
   contrato do fio IPC, camelCase via serde) e cubra com teste do front que lê o
   campo.
3. **Command novo?** Entre pela fila do actor (nunca `Session` direto) e registre
   no `generate_handler!` de `packages/app/api/src/lib.rs` — commands vivem em
   submódulo porque o macro
   oculto `__cmd__<name>` colidiria (E0255).
4. **Evento novo?** Prefixo `device://`; a leitura fica na porta
   `packages/app/ui/src/ipc/`.
5. Doc-comment PT-BR com a FONTE (ADR/§13/issue) — o crate compila com
   `#![deny(missing_docs)]`, item público sem doc é erro de build.
6. Gates locais: `cargo fmt --check` · `cargo clippy --workspace --all-targets --
   -D warnings` · `cargo test` (detalhe em `rust-practices`). O crate é do
   workspace **MSVC**; se mexeu no fio, rode também o core.
7. Fechar pelo ciclo da skill `github-flow`.

## Armadilhas conhecidas

- **Windows/MSVC obrigatório** (ADR-7/ADR-8): o Tauri 2 não suporta
  windows-gnu — o build script morre com `STATUS_ACCESS_VIOLATION`. O crate tem
  workspace próprio (o `tauri-cli` exige).
- O `gp100-cli` mantém mock por default; **no app** o default é o aparelho — não
  confunda as políticas ao comparar comportamentos.
- `DeviceSnapshot` marca seus campos: `current_name`, `current_pp_type` e
  `ir_slots_with_crc` são **só do mock** (o fio não os traz); `ir_slots` vem do
  aparelho nos dois backends. Vazio é "o app não sabe" — nunca invente valor.
- `AppState.library` é `Mutex` por exigência do Tauri para estado compartilhado,
  **não** por política: a biblioteca é disco, não fio, e não passa pelo actor.

## Onde cada coisa mora (fontes únicas)

- Comportamento do aparelho e travas → skill `hardware-dev` + `docs/referencias/GP100_DEVICE.md`
- O que já fala com o aparelho (capacidade por capacidade) → `docs/referencias/REAL_DEVICE_GAP.md`
- Estrutura do front e orçamento de módulo → `docs/dev/ARCHITECTURE.md`
- Decisões (ADR-3 timeout, ADR-4 transporte, ADR-5 escrita, ADR-6 actor/D8,
  ADR-7/8 toolchain, ADR-9 biblioteca, ADR-15 poll/read-back) → `docs/dev/DECISIONS.md`
- Protocolo/fio → `docs/tecnico/PROTOCOL.md` §13
