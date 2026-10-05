# app — aplicativo desktop (Tauri 2 + React)

Aplicativo GP-100 NextGen Editor. Divisão:

```
app/
├── ui/         FRONTEND (React 19 + TS + Vite/Vitest): telas, design tokens,
│               hooks e a porta única de IPC em `src/ipc/` (regra R1 —
│               nenhum componente importa @tauri-apps direto).
└── api/        BACKEND Rust do shell (crate gp100-ui, Tauri 2): commands de
                device, DeviceActor (dono único da Session) e eventos
                (`device://progress`, `device://push`). Toda regra de protocolo
                vive no [gp100-core](../core) — aqui só orquestra.
```

**Contrato de fio:** os DTOs Rust (serde camelCase) espelham manualmente
`ui/src/ipc/types.ts` — os testes serde do backend travam os dois lados.

## Como rodar

```bash
# Front sozinho (browser, com fallback mock de IPC — valores do MockDevice):
cd app/ui && pnpm install && pnpm dev     # http://localhost:5173

# Janela desktop real (exige MSVC no Windows — ADR-7):
cd app/api && ../ui/node_modules/.bin/tauri dev
```

Gates do front: `pnpm lint` · `pnpm test` (vitest) · `pnpm build` (tsc + vite).
Gates do backend: dentro de `app/api`, `cargo fmt -- --check`,
`cargo clippy -p gp100-ui --all-targets -- -D warnings`, `cargo test`.

## Commands disponíveis

`device_info` · `device_boot` (com progresso) · `list_user_irs` ·
`pending_pushes`. O backend default é o **mock** — o modo real entra como build
de campo pós-gate H1 (feature `real-device` espelhada do core).
