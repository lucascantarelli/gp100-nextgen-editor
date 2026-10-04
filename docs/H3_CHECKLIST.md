# 🧊 H3_CHECKLIST — Gate de hardware: congelamento pós-captura (baseline v1.2)

> **Status:** ⏳ aguardando a pedaleira + owner · **Última revisão:** 2026-10-04
> · **Responsáveis:** owner no hardware
>
> ⚠️ **Este gate é diferente do H1 e do H2.** O H1 lê, o H2 escreve. O H3
> **congela a especificação**: o golden que o `gp100-core` passa a seguir é
> derivado de uma sessão feita **pelo próprio `gp100-core`**, não pelo Suite.
> Um passo errado aqui não corrompe a pedaleira — corrompe o **contrato** que
> todo o resto do projeto assume.

---

## 0. O que a #23 já fez, e o que sobrou

A #23 entregou a maquinaria e **também encontrou coisas**. O que importa
entender antes de qualquer passo:

### O H3 era impossível até ontem

`analysis/build_golden.py` só aceitava o schema de captura do Suite
(`out_long`/`in_long` + `hex`). O `--log` do `gp100-cli` grava o schema P4
(`out`/`in` + `func`/`addr`/`data`). Medido: **0 eventos** carregados de um log
do core — sem erro, sem aviso, só um golden degenerado. Por isso o `--log`
agora grava `t` (ms desde a abertura) e existe o `analysis/wirelog.py`, que
normaliza os dois formatos.

### A baseline v1.0 afirmava coisas que o fio não sustenta

| Endereço | O que a v1.0 afirmava | Situação real |
|---|---|---|
| `13000000` | `push` com `const` de 242 bytes | É o **prefixo** de uma captura cortada. **Nunca vimos a resposta inteira.** |
| `12001002` resync | 32 ACKs de 4 bytes | **Fabricados.** As 592 instâncias reais de 4 bytes são os ACKs do upload de IR (§13.7). |
| `11000008` resync | 31 "registros de usuário" | **Fabricados** (14 bytes que saíram de um buffer cheio de `ff`). |

Detalhes e a conta inteira em [`PROTOCOL.md`](PROTOCOL.md) §13.14. A v1.1 está
registrada em `analysis/baseline.json`, com motivo e histórico.

> **O número 77 da prova de save virou 14, e isso NÃO é uma regressão.** As 63
> mensagens que sumiram mediam bytes inventados. Se alguém "consertar" esse
> número mexendo no código, está reintroduzindo a fabricação.

---

## 1. Pré-requisitos (TODOS antes de ligar a pedaleira)

**Software:**
- [ ] Binário do **H1** compilado (`--features real-device`) — esta sessão é de
      **leitura**. Nada aqui escreve no device.
- [ ] `uv run pytest -q` verde (inclui os 19 testes do H3)
- [ ] `uv run python analysis/baseline.py --check` → `baseline v1.1 integra`
- [ ] `uv run python analysis/validate_core_capture.py <qualquer>.jsonl` roda
      (use um log do mock para smoke; deve dar `20/20`)

**Com o device ligado:**
- [ ] Suite oficial **fechado** (occupancy de MIDI — armadilha do `knowledge.md`)
- [ ] H1 **passou** e `H1_REPORT.md` arquivado
- [ ] Plenty de disco: cada sessão de boot gera alguns MB de log

---

## 2. O roteiro, em 4 sessões

> Uma por vez, com o log **preservado** entre elas. A ordem importa: as
> sessões 1 e 2 são só leitura e não correm risco; a 3 e a 4 são as que
> fecham os gaps do save.

### Sessão 1 — o dump de boot (`13000000`): o maior gap

O endereço `13000000` responde com mais de 256 bytes e o proxy cortava ali.
Para fechar, é preciso o log **do gp100-core**, que grava o frame inteiro.

- [ ] `gp100-cli --real --i-know-what-im-doing --log analysis/h3/s1_boot/dump.jsonl dump-preset 0x0000`
- [ ] Confirmar que o log tem uma linha `in` com `addr":"13000000"` e que ela
      **não** termina com lixo: a linha do log é o payload puro, então o
      `F7` **não** aparece. O que confirma integralidade é o `validate_core_capture`:
- [ ] `uv run python analysis/validate_core_capture.py analysis/h3/s1_boot/dump.jsonl --markdown`
      - `0` → tudo coberto e batendo. Anote o comprimento do payload de `13000000`.
      - `1` → divergência de payload (**suspeita de bug no core** — anote func/addr)
      - `2` → endereço fora da spec (**captura nova** → ADR, R1)
- [ ] Se sair `2` em `13000000`: **é a boa notícia.** Significa que temos bytes
      reais de um endereço que o golden não descrevia. Pare e leia o §4.

### Sessão 2 — varredura de presets (o resto do boot)

- [ ] O mesmo `dump.jsonl` da sessão 1, com o `dump-preset` rodado em 3 presets
      diferentes (`0x0000`, `0x0031`, `0x0062` — os mesmos do mock de referência)
- [ ] `uv run python analysis/validate_core_capture.py analysis/h3/s2_scan/scan.jsonl`
- [ ] Anote todo endereço `2` (fora da spec) e todo `1` (divergência)

### Sessão 3 — o resync do save (o gap que a v1.0 fingiu provar)

> ⚠️ **AQUI SE ESCREVE NO DEVICE.** Esta é a parte do H3 que precisa do H2
> passar primeiro (binário com `--features real-device,write-verified`) e do
> runbook de `H2_CHECKLIST.md` inteiro respeitado: preset descartável,
> verificação no display entre as etapas. O H3 **não** é mais seguro que o H2.

- [ ] `gp100-cli --real --i-know-what-im-doing --log analysis/h3/s3_save/f1.jsonl save <pp> <type> "<nome>"`
- [ ] **`--log` é obrigatório aqui.** É o único jeito de capturar o lado IN do
      save — justamente a parte que hoje é gap.
- [ ] No display: confirme o nome e a persistência (o veredito é do H2; aqui só
      se registra)
- [ ] `uv run python analysis/validate_core_capture.py analysis/h3/s3_save/f1.jsonl`
- [ ] Os ACKs de resync devem aparecer como frames `in` reais no endpoint que o
      `H2_REPORT.md` indicar. Anote func/addr/len de cada um.

### Sessão 4 — congelamento

Só depois de 1–3, e **não antes**:

- [ ] `uv run python analysis/build_golden.py` (com as capturas novas no lugar)
- [ ] `uv run python analysis/validate_golden.py` → **tem que dar 100%**
- [ ] `uv run python analysis/make_fixtures.py` → paridade 100%
- [ ] `uv run python analysis/baseline.py bump --motivo "<por que>" --issue "#23"`
      — o `--motivo` é **obrigatório** e o bump é recusado sem ele
- [ ] Copiar o mesmo motivo para o §13.14 do `PROTOCOL.md`
- [ ] `uv run pytest -q` · `cargo test --workspace`
- [ ] Se o golden **não** mudou: não faça bump. Sem motivo, sem bump.

---

## 3. Critérios de saída (DoD do H3)

- [ ] As 3 sessões rodaram, uma por vez, com log preservado
- [ ] `validate_golden` em 100%
- [ ] `validate_core_capture` em 0 (tudo coberto) **ou** com os `2`/`1`
      catalogados e tratados um a um
- [ ] Baseline em v1.2 com motivo escrito e histórico mostrando v1.0 → v1.1 → v1.2
- [ ] Os gaps do §13.14 marcados como fechados (ou re-declarados como abertos,
      com o motivo)
- [ ] `docs/H3_REPORT.md` preenchido e arquivado

### Se algum fluxo falhar

- [ ] **PARAR.** Sem repetir às cegas.
- [ ] Guardar o log **antes** de desligar — é a única evidência
- [ ] Classificar a divergência:
  - **protocolo** (o device respondeu diferente do §13) → fluxo R3 do
    [H1_CHECKLIST.md](H1_CHECKLIST.md) §6
  - **comportamento** (respondeu o §13, o efeito não foi o esperado) → issue
    nova; **não** se corrige mexendo no codec (R1)
  - **especificação incompleta** (endereço sem template = `2` do juiz) → o
    caminho é **ADR novo + captura**, não ajuste do golden. Um template novo
    nasce de bytes observados, nunca dededução.

---

## 4. Se o juiz accuse endereço fora da spec (`exit 2`)

Este é o caso mais provável e o mais interessante: o golden v1.1 tem 39
templates e o aparelho tem mais endereços do que isso.

- [ ] Listar os endereços accused com
      `uv run python analysis/validate_core_capture.py <log> --json`
- [ ] Para **cada** endereço novo: existe captura suficiente para descrever o
      payload, ou só 1–2 exemplos?
      - **Vários exemplos** → dá para derivar template (const × var). ADR.
      - **1 exemplo só** → **não dá**. Um template de 1 exemplo é o golden v1.0
        de novo. Fica como "observado, não(caracterizado" e o mock segue sem ele.

---

## 5. Fontes

- [`docs/PROTOCOL.md`](PROTOCOL.md) §13.14 — a conta completa da v1.1 e os gaps
- `analysis/baseline.json` · `uv run python analysis/baseline.py show`
- [`docs/H1_CHECKLIST.md`](H1_CHECKLIST.md) (leitura, fluxo R3) ·
  [`docs/H2_CHECKLIST.md`](H2_CHECKLIST.md) (**escreve** — a sessão 3 depende dele)
- `analysis/wirelog.py` (normalizador) · `analysis/validate_core_capture.py`
  (juiz) · `analysis/tests/test_h3_baseline.py` (19 testes, mutation-provados)