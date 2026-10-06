# REAL_DEVICE_GAP — o que o app fala com o aparelho, e o que ainda é mock

> **Status:** 🔨 vivo · **Criado:** 05/10/2026 · **Última auditoria:** 06/10/2026 (a §1 foi reescrita: a barreira de código caiu no mesmo dia em que foi levantada) · **Issue:** [#17](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/17) (bloqueio), [#16](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/16) (entrada)
> **Pergunta que responde:** *a aplicação já está completamente conectada? todas as
> funcionalidades já estão integradas com o comportamento real? baseado no manual,
> documentos?*
> **Resposta curta:** **não.** Toda a lógica de protocolo está implementada e
> **validada contra o aparelho real** (gates H1/H2 em campo), e a **ligação já
> existe** (05/10, `d4ad1e8`/`827cbe8`): o `abrir_backend()` escolhe `RealDevice` ou
> mock conforme a feature de compilação. O que falta é do outro lado do mesmo
> par: **o build distribuído é construído SEM a feature** (o `dist-ui` chama
> `tauri build` sem `--features real-device`) e **nenhuma sessão de campo saiu do
> app** — as duas medições são a §6 passo 7 e a §1, item 1.

Este documento é o levantamento pedido pelo owner em 05/10/2026 e é a base do
go/no-go da [#17](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/17):
enquanto a coluna **app → aparelho real** da §2 não for ✅, a release é um build
mock com o nome de release.

---

## 1. A barreira de código caiu em 05/10 — o que segura a release é o BUILD e o CAMPO

> **Auditoria 06/10.** A barreira original (o `run()` instanciando `MockDevice`
> direto, `DeviceActor::spawn` monomórfico, feature `real-device` de letra morta)
> era verdadeira quando este documento nasceu no mesmo dia. Ela **fechou na
> mesma data** (`d4ad1e8`, depois `827cbe8`) e este trecho foi reescrito para
> parar de apontar para código que não existe mais. O que sobra não é código:
> é *build* e *campo* — os três itens no fim da seção.

### Como está hoje

[`packages/app/api/src/lib.rs`](../packages/app/api/src/lib.rs) — o `run()` delega
para [`abrir_backend()`](../packages/app/api/src/lib.rs):

```rust
fn abrir_backend() -> Result<abrir_backend::Escolha, Box<dyn std::error::Error>> {
    #[cfg(feature = "real-device")]
    {
        match gp100_core::transport::real::RealDevice::new() {
            Ok(real) => return Ok(abrir_backend::Escolha {
                actor: actor::DeviceActor::spawn(
                    Box::new(real) as actor::AppDevice,
                    actor::Backend::Real,
                ),
            }),
            Err(e) => { eprintln!("[device] aparelho nao abriu ({e}); caindo no MOCK"); }
        }
    }
    // …ramo do mock, com o MESMO corpo nos dois `cfg` — ver `como_app_device`
}
```

- **`AppDevice`** é um alias `cfg`: sem a feature é o próprio `MockDevice`; com
  ela, `Box<dyn DeviceBackend + Send>`. A coerção é a função `como_app_device()`
  — função e não anotação de tipo, porque `let m: AppDevice = mock` não compila
  no build comum (foi o E0308 que a primeira run do `ui-rust` mostrou nos dois SOs).
- **`DeviceActor::spawn<T: DeviceBackend + 'static>`** é genérico: qualquer
  backend entra. **`Backend::{Mock, Real}`** viaja no spawn e vira
  `DeviceSnapshot.backend` (`"mock" | "real"`) — o `DeviceInfo` não tem mais
  literal, e o `FieldDiagPanel` mostra o badge.
- **A feature `real-device` é referenciada** (`cfg` em `lib.rs` e `actor.rs`), não
  é mais letra morta: a CI compila e clippa o crate do app com ela na matriz
  `ui-rust` dos **três SOs** (`ci.yml` L372 clippy · L446 check `real-device,
  write-verified`) e no container Linux com ALSA (L665).

### O que ainda segura a coluna `app → aparelho real` da §2

1. **O build distribuível não leva a feature.** O job `dist-ui` chama
   `tauri build ${{ matrix.args }}` **sem `--features real-device`**
   ([`ci.yml`](../.github/workflows/ci.yml), ~L1366) — issue
   [#126](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/126),
   que também traz a decisão de qual **face** do build vai para a release
   (leitura primeiro é escrever). Os instaladores de hoje são builds mock **por
   linha de comando, não por limitação de código** — um `nsis`/`dmg` publicado
   assim não falaria com a pedaleira, mesmo o código sabendo. É a diferença
   entre "o caminho está compilado" e "o binário que o dono baixa tem o caminho
   ligado".
2. **Nenhuma sessão de campo partiu do APP.** O H1/H2 provaram o core e o CLI;
   `ir_send`, `tone_send` e o knob do **editor** não foram medidos. É o passo 7
   da §6, e é o que a #17 espera.
3. **O `DeviceInfo` declara sua fonte, mas nem toda ela é do aparelho.**
   `DeviceSnapshot` marca os campos que o fio não traz (`current_name`,
   `current_pp_type`, `ir_slots_with_crc`) como *só no mock*; `ir_slots` é lido
   do aparelho nos dois backends. O que falta não é uma ligação — é medir, na
   sessão do passo 7, o que muda de valor (§4).

## 2. Inventário: core · CLI · app · aparelho real

Legenda: ✅ implementado e exercitado · 🟡 implementado mas **não** com o aparelho
real · 🔴 ausente do app.

| Capacidade | core (`Session`) | CLI de campo | app (actor + command) | app → aparelho real |
|---|---|---|---|---|
| Boot + scan §13.10 | ✅ `boot_with_progress` | ✅ `info`/boot | ✅ `device_boot` | 🟡 idem, mas com inventário fixo (§4) |
| Select de preset §13.10 | ✅ `select_preset` | ✅ | ✅ `device_select_preset` | 🟡 idem |
| **Knob** §13.11 | ✅ `set_param` | ✅ `set-param` | ✅ `device_set_param` | 🟡 idem (+ trava de faixa, ADR-10) |
| Tabela de 20 User IRs §13.12 | ✅ `list_user_irs` | ✅ `list-user-irs` | ✅ `list_user_irs` | 🟡 idem (H1 provou as 20 respostas) |
| Dump de preset (meta6 + páginas) §13.9 | ✅ `state_page`/`scan_state` | ✅ `dump-preset` | ✅ `device_dump_preset` (passo 4) | 🟡 idem |
| **Save de preset** §13.12 | ✅ `save_preset` | ✅ `save` | ✅ `device_save_preset` (passo 4) | 🟡 idem |
| Upload de User IR §13.7 | ✅ `upload_ir` | ✅ `upload-ir` | ✅ `ir_send` | 🟡 idem (H2 F3 verde em campo) |
| Upload de SnapTone §5 | ✅ `upload_snap_tone` | 🔴 não exposto | ✅ `tone_send` | 🟡 idem |
| Log de fio (schema P4) + prévia do envio | ✅ decorator | ✅ `--log` / `--dry-run` | ✅ `device_log_session` / `device_preview` (passo 4b) | 🟡 idem |
| Pushes do device (D7) | ✅ `pending_pushes` | ✅ | ✅ `pending_pushes` | 🟡 idem |
| Trocar o **efeito** do slot (`0x47`) | 🔴 sem formato validado | 🔴 | 🟡 prévia local | 🔴 (bloqueio do item `C2`, §5) |
| Inventário de pps do scan | ✅ `set_inventory` | 🔴 não chama | 🔴 não chama | 🔴 (§4) |

**O que a coluna do meio mostra hoje (05/10): o app é um SUPERSETO ESTRITO do
CLI.** O core tem 12 capacidades; o CLI expõe 9 e o app expõe essas 9 **mais o
SnapTone**. Antes o app não tinha `dump-preset` nem `save`, e o CLI não tinha
SnapTone — dois subconjuntos diferentes do mesmo core. Os passos 4, 4b e 4c
(este último a camada de tela) fecharam a diferença: o que o CLI sabia fazer,
o app agora faz na mão do operador, e o `gp100-cli` deixou de ser dono de
qualquer capacidade que o app não tenha.

As **duas** capacidades que continuam 🔴 para o app — trocar o efeito do slot
(`0x47`) e o inventário de pps (`set_inventory`) — **não** são dívida do
desacoplamento: uma é falta de protocolo e a outra é falta de decisão de campo.
Nenhuma das duas se resolve trazendo o CLI para dentro do app.

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
| macOS (CoreMIDI) | job `lint-rust-clippy` / `build-rust`, entrada `ui-rust` — **desde 05/10 (passo 6b)** |

**O buraco do CoreMIDI fechou, e com ele o último motivo técnico do CLI — e
agora com prova, não com afirmacao de commit.** A run `37389032681` (48 jobs,
zero vermelhos) compila, passa `clippy -D warnings` e roda `cargo test` do crate
do app com `real-device` nos DOIS SOs da matrix do `ui-rust`, Windows e macOS.
Enquanto o `ui-rust` ficou só no Windows, o `gp100-cli` era a ÚNICA cobertura
de CoreMIDI do transporte USB-MIDI — ele roda em 3 SOs, o app não. Esse era o
motivo técnico (não uma preferência) para o CLI continuar existindo, e ele
sumiu quando o `ui-rust` entrou na matriz macOS. Agora o `dmg` sai com um
backend que a CI compila para CoreMIDI.

O Linux continua fora da MATRIZ de propósito: o `ubuntu-24.04` hosted não tem
as libs de sistema do Tauri (WebKitGTK + ALSA), que é o que a imagem
`ci-linux` existe para resolver. O ALSA é coberto no container, junto com o
e2e do webview.

**Falta o miolo, não a peça.**

## 4. Onde o comportamento muda com o aparelho real (e ninguém veria antes)

Estes quatro pontos não são "falta de função" — são **respostas diferentes** do
mesmo código. Uma sessão de teste no aparelho é a única forma de medi-los:

1. **`device_info` não tem fonte.** `MockState` carrega `preset_count`,
   `current_name`, `ir_crcs` — todos vindos do `all.prst` embutido. O aparelho
   real não tem estado local. O `DeviceInfo` precisa de uma **fonte real**
   (provavelmente o `BootReport` + a tabela de IRs + a página meta6), e os
   campos que não tiverem fonte real precisam deixar de ser prometidos.
   > **Estado 06/10:** o segundo meio da frase está feito — `DeviceSnapshot`
   > marca `current_name`/`current_pp_type`/`ir_slots_with_crc` como *só no mock*,
   > `ir_slots` é lido do aparelho nos dois backends, e `backend` não é mais
   > literal. O que sobra é **medir** o que deixa de ser verdade quando a fonte
   > muda — é isso que a sessão do passo 7 preenche.
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

   > **DECISÃO PENDENTE — owner, 05/10: "marcar como pendente no doc".**
   >
   > Não se assume `0..198` nem se implementa a descoberta agora. O que está
   > escrito aqui é o que a sessão de campo precisa **medir**, porque a resposta
   > muda o comportamento do boot de três maneiras distintas:
   >
   > | O que o aparelho responder | O que o `boot()` faz depois | Consequência |
   > |---|---|---|
   > | pps **dentro** de `0..198` | igual ao de hoje | nada — o default acerta e o passo 5 é *documentação*, não código |
   > | pps **acima** de `198` | o scan **não alcança** os que faltam | o app abre num patch que o aparelho não tem, e a biblioteca mostra 199 itens que não são do aparelho |
   > | pps **fora dos dois lados** | idem, e o cursor de pp fica errado | o pior caso: o aparelho tem mais patches e nenhum caminho para eles |
   >
   > **Como medir (passo 7, no aparelho).** O `.jsonl` do painel de diagnóstico
   > já traz a resposta sem código novo: no boot, um pp além do último que
   > responder é um pp que o aparelho tem. Se a sequência `0..198` for
   > completa e o pp 199 não responder, o default acerta.
   >
   > **O que fazer quando a resposta aparecer.** Se houver pps acima de `198`,
   > a correção é chamar `set_inventory` com o que o boot descobriu — mas essa
   > é escrita de código, e código esperando número medido é exatamente o
   > que o ADR-10 (trava de faixa) existe para evitar: regra de parede com
   > número inventado.
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
| `B6` Save/Import/Export/Rename no device | 🔴 | `save_preset` existe no core, no CLI **e no app** (`device_save_preset` + `diagSave`, passo 4/4c). O que ainda falta é produto: nenhum botão da biblioteca/palco grava no aparelho, e a sessão de campo não mediu uma única gravação feita pelo editor |
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
| 1 | `DeviceActor` genérico sobre `Box<dyn DeviceTransport>` + seleção mock/real — **✅ feito** (`d4ad1e8`, 05/10: `AppDevice` alias `cfg` + `abrir_backend()`; a CI clippa e checa com `real-device` nos três SOs) | `packages/app/api/src/{actor,lib}.rs` | os testes do actor passam **sem** mudar (o mock vira o caso padrão); `--features real-device` compila | não |
| 2 | `DeviceInfo` com **fonte real** (e campos que não têm fonte, declarados) — **✅ feito** (05/10: `DeviceSnapshot` marca `current_name`/`current_pp_type`/`ir_slots_with_crc` como *só no mock*; `ir_slots` é lido do aparelho nos dois backends) | `commands.rs` | `DeviceInfo` sai do `BootReport`/tabela/meta6, não do `MockState` | não |
| 3 | Botões de escrita cientes da política (`write-verified` → desabilitado + aviso) — **em aberto**: só o `FieldDiagPanel` lê `writeVerified`; o knob, o IR e o SnapTone mandam e deixam o erro tipado chegar. É parte da face (A) da [#126](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/126) | front + commands | e2e do botão desabilitado no build de leitura | não |
| 4 | `save_preset` e `dump_preset` como commands — **✅ feito** (o que o CLI tinha e o app nao) | `actor.rs` + `commands.rs` | vetor de bytes igual ao do CLI | não |
| 4b | **wire logger (schema P4) + dry-run no app** — **✅ feito**: `packages/core/src/wire_log.rs` (uma implementacao, CLI e app) + `device_log_session`/`device_preview`. O ciclo de campo agora fecha pelo app: sessao no editor → `.jsonl` → juiz | `wire_log.rs` + `commands.rs` | o `.jsonl` que o app grava passa no mesmo juiz que o do CLI | **sim** (para o veredito) |
| 4c | **A camada de UI do diagnóstico** — **✅ feito**: `FieldDiagPanel.tsx` + `useFieldDiag` + `ipc/diag.ts`. As quatro capacidades dos passos 4/4b viraram tela (gravar, ler o dump, ligar/desligar o log, ver o que sairia), com o badge de backend e o aviso de escrita travada na tela. Sem isto a sessao de campo continuava dependendo do binario de terminal | `components/FieldDiagPanel.tsx` · `hooks/useFieldDiag.ts` · `ipc/diag.ts` | o operador de campo nao precisa abrir terminal para dirigir o aparelho | **sim** (para o veredito) |
| 5 | `set_inventory` ligado ao que o boot descobre (ou fixado em campo com justificativa) | `session.rs` + `lib.rs` | o total de transações do report muda conforme o inventário | **sim** |
| 6 | Build de campo **leitura** (`--features real-device`, sem `write-verified`) — **✅ feito** (compilação); **o `dist-ui` ainda não o usa** — issue [#126](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/126) | `scripts/` | CI compila o crate do Tauri com a feature (WinMM no job `ui-rust`; ALSA no container do webview) | não |
| 6b | `ui-rust` na matriz **macOS** — fecha o buraco do CoreMIDI — **✅ feito** (05/10): a matriz do `ui-rust` tem Windows + macOS, e o ALSA segue no container | `scripts/ci_plan.py::matrices` | o backend do app com `real-device` compila para CoreMIDI | não |
| 7 | **Sessão de campo no aparelho**: boot, lista de IRs, dump, e a §4 medida | o painel de diagnóstico (passo 4c) + relatório | relatório com os 4 desvios de §4 preenchidos, com o `.jsonl` gerado **pelo app** | **sim** |
| 8 | Release | #17 | o veredito da sessão de campo assinado | **sim** |

Passos 1, 2, 4, 4b, 4c, 6 e 6b são software e foram feitos. Ficam de pé **o
passo 3** (software também: o knob, o IR e o SnapTone ainda não leem
`writeVerified` — é a face (A) da [#126](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/126)),
**o passo 5** (uma decisão de campo) e **o passo 7** (uma sessão com o
aparelho). **O passo 7 é o que a #17 exige, e a #17 não fecha antes dele.**

**O passo 4c é o que muda o formato do passo 7.** A sessão de campo deixa de ser um
roteiro de terminal e passa a ser um relatório de tela: o operador abre o editor,
grava o patch, lê o dump e entrega o `.jsonl` que o **app** gravou. É a última
peça de software antes do aparelho — depois dela, o que falta é o aparelho, não
o código.

## 7. O que a #17 já tem e o que falta

Já tem: plano de release, empacotamento (`PACKAGING.md`), `simulate_release`
verde, 13 gates, baseline do golden versionada, e o histórico do PR
`#112` provando que a build de campo da CI é sadia.

Falta, e é o que decide o go/no-go:
- o **build distribuível** sair com a feature ligada — hoje o `dist-ui` constrói
  sem `--features real-device`, então o instalador publicado é mock
  ([#126](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/126), §1
  item 1);
- uma **sessão de campo** com o aparelho ligado, com veredito assinado — e ela
  agora acontece **dentro do app** pelo painel de diagnóstico (passo 4c), não
  por roteiro de terminal;
- o `ir_send`/`tone_send`/knob exercitados **de verdade** (o H2 provou o CLI, não
  o app).

**O que NÃO falta mais:** a cobertura de compilação do caminho real está nos
três SOs (WinMM, ALSA, CoreMIDI) e as capacidades que eram só do CLI ganharam
tela. O que ainda segura a #17 é o aparelho, não o código.

**Recomendação: manter a #17 aberta.** O build atual é honesto como *build de
desenvolvimento* — ele fala com um mock que responde como o aparelho, e a UI não
mente sobre isso. O que ele não pode é ser chamado de release do GP-100.
