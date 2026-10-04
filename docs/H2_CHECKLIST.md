# 🎛️ H2_CHECKLIST — Gate de hardware: escrita real dos 3 fluxos capturados

> **Status:** ⏳ aguardando a pedaleira + owner · **Última revisão:** 2026-10-04
> · **Responsáveis:** owner no hardware
>
> ⚠️ **O QUE ESTE GATE É.** A escrita real no GP-100 é a única operação do
> projeto que **muda o estado de um aparelho do usuário**. Tudo o resto — boot,
> scan, leitura de preset, leitura de IR — é seguro repetir. Aqui, um frame
> errado grava um preset errado ou occupy um slot de IR, e o preset do usuário
> não volta sozinho.
>
> **Por isso este gate é deliberadamente mais lento que o H1.** O H1 é leitura
> pura e lasted ~30 min; o H2 tem **3 fluxos, um por vez, com verificação no
> display entre eles**. Não existe o "vou testar os 3 de uma vez".

---

## 0. Por que a escrita está trancada por padrão

Enquanto `WRITE_VERIFIED` (ADR-5) não é ligado, **escrever no device real é
impossível** — não há flag, variável de ambiente ou argumento que destrave.
Isto não é metáfora: é a feature de compilação.

```bash
# H1 (leitura) — o binário do gate de LEITURA, sem escrita:
cargo build --release -p gp100-cli --features real-device

# H2 (escrita) — o binário deste gate:
cargo build --release -p gp100-cli --features real-device,write-verified
```

O mecanismo, em uma frase: **cada frame que sai pela trait `DeviceTransport`
declara se é `WireKind::Read` ou `WireKind::Write`**, e o `RealDevice` recusa
`Write` antes de tocar no driver. A classificação é declarada por quem conhece
a semântica (a `Session`), nunca deduzida do byte FUNC — porque `FUNC 0x12` é
usado tanto para escrita quanto para a leitura de página do §13.10, e um gate
por FUNC recusaria o próprio caminho de leitura.

> **Efeito colateral registrado pelo PR:** o keepalive de boot (`12/00020001`,
> §13.12) é um frame OUT que não pede resposta — ou seja, uma **escrita** pelo
> critério acima. Então o **boot completo (B5 do H1) exige a feature do H2**.
> O `H1_CHECKLIST.md` §3 oferece o B5 como opcional dentro de uma sessão
> declarada "LER é seguro"; isso não é mais verdade, e este documento registra.
> A decisão de fundo (pular o keepalive no B5, ou promover o B5 para o H2)
> é do owner com a pedaleira na mão.

---

## 1. Pré-requisitos (TODOS antes de ligar a pedaleira)

**Software:**
- [ ] Binário de campo do H2 compilado (`--features real-device,write-verified`)
- [ ] `gp100-cli --help` mostra `upload-ir` e cita `write-verified`
- [ ] `gp100-cli set-param ... ` **sem** `--dry-run` responde com a recusa do
      ADR-5 (e NÃO envia) — prova de que a trava funciona neste binário
- [ ] CI verde no commit do binário

**Antes de qualquer escrita, com o device ligado:**
- [ ] O H1 passou (`docs/H1_REPORT.md` arquivado) — escrever em um device cuja
      leitura ainda diverge é o pior cenário possível
- [ ] **Inventário do device anotado:** quais slots de IR estão ocupados e
      quais presets são seus. Sem isto, um upload de IR pode sobrescrever algo
      que não dá para recuperar
- [ ] Suite oficial FECHADO (occupancy de MIDI, armadilha do `knowledge.md`)

---

## 2. Os 3 fluxos — regras que valem para todos

1. **UM por vez.** Não rode dois fluxos na mesma sessão sem ler o display entre
   eles.
2. **Preset alvo descartável.** Todo `set-param` e todo `save` vai para um
   preset que você não vai sentir falta de perder. O save persiste o **estado
   ao vivo** do preset (§13.12, confirmado em campo na S4) — o `set-param`
   mexe no preset selecionado, então salvar "para testar" grava o que você
   acabou de mexer.
3. **Soltar tudo entre as etapas.** O comando termina, a conexão fecha
   (`std::process::exit` fecha a porta). Abrir e fechar a porta MIDI em
   sequência é seguro; deixar aberta enquanto se pensa é o que ocupa o
   device para o próximo programa.
4. **Log de fio SEMPRE.** `--log` em toda escrita. É a única evidência se algo
   der errado, e é matéria-prima para o H3 (baseline v1.1).
5. **O display é a verdade.** Nenhum "parece que salvou" vale. Se o display
   não mostrar, não salvou.

---

## 3. Fluxo 1 — `set-param` (knob, §13.11)

O mais simples dos três: 1 frame, sem resposta, sem read-back no fio (D4).

- [ ] Anote no display o valor ANTES do knob escolhido
- [ ] Escolha um knob de efeito com valor visível no display (ex.: AMP Gain —
      o mesmo que a S3 usou, gain ~99)
- [ ] Escolha um valor **diferente** do que está no display
- [ ] `gp100-cli --real --i-know-what-im-doing --log analysis/h2/<sessão>/f1_setparam.jsonl set-param <slot> <code> <ctrl> <valor>`
      - `slot` = 1..=9, a posição na **cadeia** (1=PRE … 9=RVB) — *não* o slot de IR
      - `code` = `effectCode` do algoritmo, u32 hex (ex.: Bog RedM = `0x0700006e`)
      - `ctrl` = índice do controle (AMP: 0=Gain, 1=PRES, 2=Master)
- [ ] **Confirme no display** que o valor mudou para o que você pediu
- [ ] Veredito: `mudou` · `não mudou` · `device reagir estranho`

> **Não há read-back no fio** (D4: a captura não tem resposta, e a FSM não
> inventa uma). A verificação deste fluxo é **exclusivamente visual**. Se você
> precisa de read-back programático, isso é captura nova (R2/R3), não um
> ajuste no código.

## 4. Fluxo 2 — `save` (§13.12)

9 frames: 5 de metadado + o ciclo de ops (op0 ×2 → op1 ×2). **Zero IN
esperado** (D3 — fire-and-forget; burst tardio NÃO é confirmação).

- [ ] Escolha um preset **descartável** (regra 3.2)
- [ ] Anote o nome atual do preset no display
- [ ] `gp100-cli --real --i-know-what-im-doing --log analysis/h2/<sessão>/f2_save.jsonl save <pp> <pp-type> "<nome-novo>"`
      - `pp` = id do preset (hex `0x0000` ou decimal)
      - `pp-type` = 4=Rock, 6=Pop (o dicionário do `.prst`)
      - nome = até 12 caracteres (o `.prst` trunca e preenche)
- [ ] Confirme no display: **o nome do preset mudou** para o novo
- [ ] Troque o preset no display (sair e voltar) e confirme que o nome voltou
      — isto testa **persistência**, não só cache
- [ ] Veredito: `persistiu` · `nome mudou mas não persiste` · `não mudou`

## 5. Fluxo 3 — `upload-ir` (§13.7)

O mais pesado: ~296 frames + 296 ACKs por IR, ~5 s cada (§13.7). O fim do
upload é a **duplicação do último chunk** (idx `0x226`), não um commit.

- [ ] Escolha um slot de IR **VAZIO** (confirme no display + com
      `list-user-irs` ANTES de escrever). Regra dura: **não subir em slot
      ocupado** — o conteúdo do slot anterior não é recuperável.
- [ ] O blob tem de ser **múltiplo de 15 bytes** (§13.7 rev.2: a FSM
      *rejeita*, não padroniza). O `analysis/test_ir_mono.wav` do repo serve
      de insumo, mas o upload é do **blob** de 4.425 B (§13.7: o device
      armazena representação reduzida, não o WAV cru)
- [ ] `gp100-cli --real --i-know-what-im-doing --log analysis/h2/<sessão>/f3_upload.jsonl upload-ir <slot> <arquivo-do-blob>`
- [ ] Confirme no display: o slot mostra o nome do IR
- [ ] Read-back de software (a única verificação programática dos 3 fluxos):
      `gp100-cli --real --i-know-what-im-doing list-user-irs` → o nome do slot
      não pode estar vazio nem `0xFF`
- [ ] Veredito: `slot ocupado com o nome` · `slot vazio` · `device travou`

---

## 6. Critérios de saída (DoD do H2)

- [ ] Os 3 fluxos rodaram, **um por vez**, com verificação no display
- [ ] Cada uma das 3 verificações tem um veredito escrito (não "pareceu que")
- [ ] `sessionH2.jsonl` (ou os 3 logs `f1`/`f2`/`f3`) preservados e com
      `-text` no `.gitattributes`
- [ ] `docs/H2_REPORT.md` preenchido e arquivado
- [ ] Nenhuma anomalia no device (travamento, reboot, display incoerente)

### Se os 3 passarem
- [ ] ROADMAP: H2 ✅ com data
- [ ] O `write-verified` vira **o padrão do binário de campo**? **Decisão do
      owner.** O padrão atual (feature OFF) é o que mantém o H1 seguro; virar
      padrão é uma mudança de política e merece ADR, não um flip silencioso.

### Se algum falhar
- [ ] **PARAR.** Não repetir o fluxo às cegas, não trocar de slot, não "só mais
      uma vez"
- [ ] Preservar o log e o texto da saída **antes** de desligar
- [ ] Registrar a divergência: é divergência de **protocolo** (o device
      respondeu diferente do §13) ou de **comportamento** (respondeu o §13 mas
      o efeito não foi o esperado)? As duas têm caminhos diferentes:
  - **protocolo** → fluxo R3 do [H1_CHECKLIST.md](H1_CHECKLIST.md) §6
  - **comportamento** → nova issue; **não** se resolve mexendo no codec
    (R1: protocolo adivinhado não entra)

---

## 7. Watchlist do H2 (o que se espera dar errado)

- **O `00020001` de boot e escrita.** Se a sessão incluir um boot, ele precisa
  da feature do H2 (§0).
- **D3: save não tem resposta.** Se você ficar esperando o device "confirmar",
  o timeout não é bug — é o protocolo. A confirmação é o display.
- **Pacing do save.** A S4 mediu op0 em +578 ms e op1 em +593 ms do op0. O
  CLI envia os 9 frames sem espera entre eles; se o device da S4 for o mesmo,
  isso **pode** estar rápido demais. Não é erro do CLI por si — é o primeiro
  achado candidato do H2, e vale registrar antes de "corrigir" qualquer coisa.
- **Flush de fim de sessão.** As capturas S2 e S4 encerram com burst IN
  espontâneo (§13.7, quirk fechado). Se aparecer no seu log **depois** da
  operação, é o mesmo fenômeno — não é erro de escrita e não é confirmação.
- **Occupancy.** Qualquer programa MIDI aberto → `OpenFailed` no `open()`.
  O objeto reconecta (`open()` de novo), mas primeiro feche o Suite.

---

## 8. Fontes

- `docs/DECISIONS.md` ADR-5 (`WRITE_VERIFIED`) e ADR-6 (D3/D4, semântica de
  escrita) · ADR-2 (erro tipado, sem panic) · ADR-3 (janela de 3 s)
- `docs/PROTOCOL.md` §13.7 (upload de IR) · §13.11 (knob) · §13.12 (save e
  metadados) · §13.2 (tabela de FUNC, e por que ela **não** serve para
  classificar escrita)
- `docs/VISION.md` §7 (brick-proof, política de hardware)
- `docs/H1_CHECKLIST.md` (o gate de leitura, e o fluxo R3)
- `packages/core/tests/write_gate.rs` (a trava, mutation-provada)