# REAL_DEVICE_GAP — o que o app fala com o aparelho, e o que ainda é mock

> **Status:** 🔨 vivo · **Criado:** 05/10/2026 · **Issue:** [#17](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/17) (bloqueio), [#16](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/16) (entrada)
> **Pergunta que responde:** *a aplicação já está completamente conectada? todas as
> funcionalidades já estão integradas com o comportamento real? baseado no manual,
> documentos?*
> **Resposta curta:** **não.** Toda a lógica de protocolo está implementada e
> **validada contra o aparelho real** (gates H1/H2 em campo). O que não existe é a
> **ligação**: o app do Tauri é *monomórfico* no `MockDevice` e não tem como falar
> com a pedaleira. Nenhum byte do build distribuível toca hardware.

Este documento é o levantamento pedido pelo owner em 05/10/2026 e é a base do
go/no-go da [#17](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/17):
enquanto a coluna **app → aparelho real** da §2 não for ✅, a release é um build
mock com o nome de release.

---

## 1. A barreira, em três linhas de código

[`packages/app/api/src/lib.rs`](../packages/app/api/src/lib.rs) — o `run()`:

```rust
let mut mock = MockDevice::new()?;
let actor = actor::DeviceActor::spawn(mock);
```

[`packages/app/api/src/actor.rs`](../packages/app/api/src/actor.rs) — o spawn:

```rust
pub fn spawn(device: MockDevice) -> Self {
    // ...
    let mut session = Some(Session::new(device));
```

E o detalhe que fecha a porta: a feature `real-device` está **declarada** em
`packages/app/api/Cargo.toml` e **nunca referenciada** em `packages/app/api/src/`.
Ela é letra morta — compilar com `--features real-device` não muda nada.

Não é um bug de plumbing, é **monomorfismo**: `DeviceActor::spawn` aceita
`MockDevice` (tipo concreto), não `Box<dyn DeviceTransport>`. A `Session<T>` do
core já é genérica e já tem o impl de `Box<dyn DeviceTransport>` em
[`transport/mod.rs`](../packages/core/src/transport/mod.rs) — **o CLI de campo
já faz exatamente esse dispatch** ([`packages/cli/src/main.rs`](../packages/cli/src/main.rs),
`LoggingTransport<Box<dyn DeviceTransport>>`, `--real` + `--i-know-what-im-doing`).
O caminho está escrito e testado; só o app não o usa.

E há um segundo furo, mais silencioso: mesmo com a trait object,
[`DeviceActor::info`](../packages/app/api/src/actor.rs) devolve `MockState`, um
tipo que **só o mock produz**. `DeviceInfo::from_mock` embute `backend: "mock"`
como literal. Com um aparelho real esse campo não tem de onde sair.

## 2. Inventário: core · CLI · app · aparelho real

Legenda: ✅ implementado e exercitado · 🟡 implementado mas **não** com o aparelho
real · 🔴 ausente do app.

| Capacidade | core (`Session`) | CLI de campo | app (actor + command) | app → aparelho real |
|---|---|---|---|---|
| Boot + scan §13.10 | ✅ `boot_with_progress` | ✅ `info`/boot | ✅ `device_boot` | 🟡 idem, mas com inventário fixo (§4) |
| Select de preset §13.10 | ✅ `select_preset` | ✅ | ✅ `device_select_preset` | 🟡 idem |
| **Knob** §13.11 | ✅ `set_param` | ✅ `set-param` | ✅ `device_set_param` | 🟡 idem (+ trava de faixa, ADR-10) |
| Tabela de 20 User IRs §13.12 | ✅ `list_user_irs` | ✅ `list-user-irs` | ✅ `list_user_irs` | 🟡 idem (H1 provou as 20 respostas) |
| Dump de preset (meta6 + páginas) §13.9 | ✅ `state_page`/`scan_state` | ✅ `dump-preset` | 🔴 **não exposto** | 🔴 |
| **Save de preset** §13.12 | ✅ `save_preset` | ✅ `save` | 🔴 **não exposto** | 🔴 |
| Upload de User IR §13.7 | ✅ `upload_ir` | ✅ `upload-ir` | ✅ `ir_send` | 🟡 idem (H2 F3 verde em campo) |
| Upload de SnapTone §5 | ✅ `upload_snap_tone` | 🔴 não exposto | ✅ `tone_send` | 🟡 idem |
| Pushes do device (D7) | ✅ `pending_pushes` | ✅ | ✅ `pending_pushes` | 🟡 idem |
| Trocar o **efeito** do slot (`0x47`) | 🔴 sem formato validado | 🔴 | 🟡 prévia local | 🔴 (bloqueio do item `C2`, §5) |
| Inventário de pps do scan | ✅ `set_inventory` | 🔴 não chama | 🔴 não chama | 🔴 (§4) |

O que a coluna do meio mostra: **o core tem 11 capacidades, o CLI expõe 8, o app
expõe 8 — e são conjuntos diferentes.** O app não tem `dump-preset` nem `save`;
o CLI não tem SnapTone. Não é o app "atrasado" no mesmo caminho, são **dois
subconjuntos diferentes do mesmo core**.

## 3. O que já está pronto (e é mais do que parece)

Isto não precisa ser refeito para o app falar com a pedaleira:

- **`RealDevice` está implementado e completo** —
  [`packages/core/src/transport/real.rs`](../packages/core/src/transport/real.rs),
  253 linhas: RX por callback com trim no 1º `F7`, TX por SysEx completo,
  despacho de porta por nome contendo `gp-100`, reconexão no mesmo objeto, e a
  trava `WRITE_VERIFIED` do ADR-5 **antes do driver**.
- **As leituras foram provadas em campo** — a sessão H1 real
  ([`analysis/captures/sessionH1/`](../analysis/captures/sessionH1/)) tem as 20
  respostas de `list-user-irs` e as 5 transações de `dump-preset` gravadas.
- **As escritas foram provadas em campo** — o gate H2 (05/10) fechou os 3 fluxos:
  F1 knob (1 frame OUT), F2 save (9 frames, persistiu), F3 upload de IR (295
  chunks / 296 ACKs).
- **A trava de faixa do knob existe** ([#110](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/110),
  ADR-10) e é de **conteúdo**, não de política: vale igual no mock e no aparelho
  real. Uma build de campo não precisa de defesa extra contra o `99.5`.
- **A política de escrita do CLI funciona**: `--real` exige
  `--i-know-what-im-doing` **e** a feature de compilação, e a CI já constrói e faz
  smoke do binário com e sem `write-verified`.


### A cobertura do caminho real (e por que o CLI ainda existe)

O `RealDevice` usa o `midir`, que resolve para **WinMM** (Windows),
**CoreMIDI** (macOS) e **ALSA** (Linux). Isso significa que o mesmo `cfg` do
app compila codigo diferente em cada SO — e uma classe de bug que o build de
Windows nao pega.

| build do app com `real-device` | onde e compilado |
|---|---|
| Windows (WinMM) | job `lint-rust` / `build-rust`, entrada `ui-rust` |
| Linux (ALSA) | job `test-e2e-webview`, no container `ci-linux` (que ja tinha ALSA) |
| **macOS (CoreMIDI)** | **ninguem** — o `ui-rust` saiu da matriz macOS por custo |

O transporte real do macOS continua coberto pelo `cli`, que roda em 3 SOs.
**Esse e o motivo tecnico, e nao uma preferencia, para o CLI ainda existir:**
ele e a unica cobertura de CoreMIDI do transporte USB-MIDI. Se o CLI sair antes
de o `ui-rust` ganhar macOS, o `dmg` sai com um backend que ninguem nunca
compilou para CoreMIDI.

**Falta o miolo, não a peça.**

## 4. Onde o comportamento muda com o aparelho real (e ninguém veria antes)

Estes quatro pontos não são "falta de função" — são **respostas diferentes** do
mesmo código. Uma sessão de teste no aparelho é a única forma de medi-los:

1. **`device_info` não tem fonte.** `MockState` carrega `preset_count`,
   `current_name`, `ir_crcs` — todos vindos do `all.prst` embutido. O aparelho
   real não tem estado local. O `DeviceInfo` precisa de uma **fonte real**
   (provavelmente o `BootReport` + a tabela de IRs + a página meta6), e os
   campos que não tiverem fonte real precisam deixar de ser prometidos.
2. **O palco (`device_board`) mostra o arquivo embutido, não o aparelho.**
   [`Request::Board`](../packages/app/api/src/actor.rs) faz
   `embedded_document()` — projeção pura de `all.prst`, **zero tráfego de fio**.
   Consequências com o aparelho real: um preset de usuário (`U01`…) não existe no
   arquivo embutido; e o valor do knob que o palco mostra é o do arquivo, não o
   que o aparelho tem depois de um ajuste.
3. **O boot assume um inventário fixo de 199 pps.** Nem o app nem o CLI chamam
   `Session::set_inventory` — os dois usam o default `0..198` (2297 transações). O
   `H1_CHECKLIST` §B5 já sinalizou isso como R3 em aberto: *"Device real com pps
   fora de `0..198` → boot() com inventário default diverge"*. O método já
   existe; ninguém o chama.
4. **Escrita vai exigir o destravamento.** Com `--features real-device` **sem**
   `write-verified` (decisão do owner em 05/10/2026: leitura primeiro), as quatro
   escritas do app — knob, select, IR, SnapTone — são **recusadas com erro
   tipado** antes do driver. O app precisa mostrar isso como estado, não como
   falha: o botão fica desabilitado com "build de leitura".

## 5. O que o manual exige e ainda não existe

Cruzeando [MANUAL_COVERAGE.md](MANUAL_COVERAGE.md) com esta §2 — a matriz de
cobertura mede **UI contra o manual** (35 ✅ · 5 🟡 · 5 🔴), e é honesta. Mas
cinco itens dela são **🔴/🟡 por falta de canal de escrita**, e "canal de escrita"
agora significa duas coisas:

| Item do manual | O que a matriz diz | Por quê, em termo de canal |
|---|---|---|
| `B6` Save/Import/Export/Rename no device | 🔴 | `save_preset` existe no core e no CLI; **não há command no app** |
| `C2` Effects List (trocar o efeito) | 🟡 | `0x47` sem formato validado (BLOCKERS 10b) — bloqueia **em qualquer backend** |
| `T6` Stomp Mode · `P2/P3` Patch BPM/EXP | 🟡 | mesmo bloqueio de `C2` |
| `S2/S3/S4` escritas do menu GLOBAL | 🟡 | idem |
| `T5/T7` Master VOL · Kill switch · `D5/D6` DRUM · `L1..L5` Looper | ✅ na UI | ✅ **mas em prévia local** (`gp100.master.v1`, `gp100.drum.v2`, `gp100.looper.v1`, `gp100.settings.general.v1`, `gp100.tuner.v1` no `localStorage`) — não têm endereço no §13 |

O último item é o que o dono precisa ouvir antes de qualquer release: **um ✅ na
matriz de cobertura significa "a tela existe e o e2e passa", não "o pedal gira
esse botão"**. Para o pedal girar, o item precisa existir na §2 acima.

## 6. Caminho até a release real

Ordem escolhida por risco: cada passo é verificável antes do próximo, e nenhum
deles precisa do aparelho para ser provado.

| # | Passo | Onde | DoD | Precisa do aparelho? |
|---|---|---|---|---|
| 1 | `DeviceActor` genérico sobre `Box<dyn DeviceTransport>` + seleção mock/real | `packages/app/api/src/{actor,lib}.rs` | os testes do actor passam **sem** mudar (o mock vira o caso padrão); `--features real-device` compila | não |
| 2 | `DeviceInfo` com **fonte real** (e campos que não têm fonte, declarados) | `commands.rs` | `DeviceInfo` sai do `BootReport`/tabela/meta6, não do `MockState` | não |
| 3 | Botões de escrita cientes da política (`write-verified` → desabilitado + aviso) | front + commands | e2e do botão desabilitado no build de leitura | não |
| 4 | `save_preset` e `dump_preset` como commands (os 2 que faltam) | `actor.rs` + `commands.rs` | vetor de bytes igual ao do CLI | não |
| 5 | `set_inventory` ligado ao que o boot descobre (ou fixado em campo com justificativa) | `session.rs` + `lib.rs` | o total de transações do report muda conforme o inventário | **sim** |
| 6 | Build de campo **leitura** (`--features real-device`, sem `write-verified`) | `scripts/` | CI compila o crate do Tauri com a feature (WinMM no job `ui-rust`; ALSA no container do webview) | não |
| 6b | `ui-rust` na matriz **macOS** — fecha o buraco do CoreMIDI | `scripts/ci_plan.py` | o backend do app com `real-device` compila para CoreMIDI | não |
| 7 | **Sessão de campo no aparelho**: boot, lista de IRs, dump, e a §4 medida | `scripts/h2_field.sh` + relatório | `docs/H3`/novo relatório com os 4 desvios de §4 preenchidos | **sim** |
| 8 | Release | #17 | o veredito da sessão de campo assinado | **sim** |

Passos 1–4 e 6 são software e podem ser feitos e provados agora. O passo 5 é o
primeiro que precisa de uma decisão do campo. **O passo 7 é o que a #17 exige,
e a #17 não fecha antes dele.**

## 7. O que a #17 já tem e o que falta

Já tem: plano de release, empacotamento (`PACKAGING.md`), `simulate_release`
verde, 13 gates, baseline do golden versionada, e o histórico do PR
`#112` provando que a build de campo da CI é sadia.

Falta, e é o que decide o go/no-go:
- o binário distribuível **falar com o aparelho** (§1);
- uma **sessão de campo** com o aparelho ligado, com veredito assinado;
- o `ir_send`/`tone_send`/knob exercitados **de verdade** (o H2 provou o CLI, não
  o app).

**Recomendação: manter a #17 aberta.** O build atual é honesto como *build de
desenvolvimento* — ele fala com um mock que responde como o aparelho, e a UI não
mente sobre isso. O que ele não pode é ser chamado de release do GP-100.
