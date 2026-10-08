# Design: Decode das páginas 13xx — o palco vivo e os nomes reais (ciclo 1 · #155)

> **Status:** Approved (design em chat, 3 seções, 08/10/2026) · **Próximo passo:** skill `writing-plans`
> **Escopo:** issue [#155](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/155) — ciclo 1.
> A [#152](https://github.com/lucascantarelli/gp100-nextgen-editor/issues/152) (meta6) **não** está no escopo; ver §9.
> **Decisões de escopo aprovadas pelo owner (08/10/2026):** (1) estática primeiro, **captura dirigida dentro do mesmo spec**, como confirmação por manipulação; (2) um ciclo por issue, páginas primeiro; (3) decode completo **em duas fases com checkpoint** (fase 1 nome/inventário → fase 2 slots/knobs).

---

## 1. Objetivo

Decifrar o layout das 9 páginas de estado do preset (`13 01 00 03`) para que, com backend `Real`:

- `device_board` deixe de ser **recusado** (`packages/app/api/src/actor.rs` ~L702) e desenhe o **conteúdo vivo do patch editado no aparelho**;
- `device_preset_library` / navbar sirvam **nomes reais** e `DeviceSnapshot.current_name` deixe de ser `"Só no mock"`;
- `validate_golden.py` continue **byte-idêntico**.

**Régua do repo (não negociável):** captura → hipótese → validação → core, com **prova-negativa** — o padrão da trava de faixa (#110) e do inventário (#132). Um offset só entra se casar onde deve **e** falhar onde não deve.

---

## 2. Verdade dos dados (medido, não inferido)

Todos os números abaixo saíram da leitura direta de `analysis/captures/session1.jsonl` + `files/patches/all.prst` + `analysis/parameters.json` em 08/10/2026.

| Fato | Medida |
|---|---|
| Dataset | **198 pps × 9 páginas = 1782** requests `13010004` (OUT) · **1783** respostas `13010003` (IN) |
| Respostas por endereço | `13010001` (meta6) **199 @ 6B** · `13010003` **1783** (1585 @ 196B + 198 @ 32B) · `13010005` **0** |
| Header da página | `[pp u16BE][?][PG]` = **4B**, depois o corpo |
| Tamanho cru | pg 0..7 = **192B**, pg 8 = **28B** |
| **Encoding** | **nibble-expandido** (par `04 09` → `0x49`) — decodificado: pg 0..7 = **96B**, pg 8 = **14B** → **782B/preset** |
| **Nome** | **198/198** em **pg0, offset 2** (expandido) — `00 00 \| nome[16, pad 00] \| 0100 0200 …` — **zero** falsos positivos |
| meta6 `13010001` | `[pp u16BE] 0c 1c 01 40` — **constante em 198/198** → não carrega dado por-preset |
| `11000008` | 1232B com **só** estilos/gêneros: `Metal · Indie · Rock · Funk · Blues · Jazz · Bass` — zero nomes |
| Nome em endereço cru | **0/99** em todos os endereços IN (ASCII, UTF-16LE/BE, nibble, xor/shift) |
| Params do XML como u16 LE cru | **0/127** chaves com vencedor ≥ 90% — falha **porque** tudo é nibble |
| `pp` vs `pp+0x100` | **0/99 idênticos** — os dois bancos têm conteúdo diferente |
| `ppType` do XML na pg0 | **166/198** — não é prova; precisa de offset próprio |

**Prova-negativa embutida.** O próprio DoD da #155 pedia varredura de offsets crus (0..28 × len 6/10/12 = 0 hits). Aquela varredura falha **por causa do nibble** — é a evidência de que a tentativa anterior olhava o formato errado, não de que o dado não existe. O nome em offset cru: 0/99. O mesmo nome decodificado: 198/198 no mesmo lugar.

---

## 3. Escopo

**Dentro (fase 1 — checkpoint):**
1. Corrigir/entregar a análise em `analysis/` (decode nibble antes de votar).
2. Parser do **nome** das páginas, no core, testado contra 198/198. (Só o nome: `ppType` casa em 166/198 na pg0 e portanto **não** é fase 1 — ver §11.2.)
3. Biblioteca/navbar com nomes reais; `current_name` honesto.
4. Captura dirigida de **confirmação** (renomear um patch no aparelho → aparece no app sem re-seed).

**Dentro (fase 2 — o bloqueante):**
5. Decodificar os **9 slots + knobs** → `BoardView`.
6. `device_board` com backend `Real` **sem** a recusa.
7. Captura dirigida de confirmação (girar um knob no hardware → qual byte muda).

**Fora:**
- **meta6** (#152) — ver §9: a hipótese dela está refutada pelos dados.
- OSC, cloud, qualquer mudança no `.prst` ou no golden.
- Reescrever `Session::state_page` — ele já devolve `StatePage { raw }` e está correto.

---

## 4. Arquitetura

```
Session::state_page(0..=8)  →  StatePage { raw }          [já existe, intocado]
        │
        ▼
preset_pages::decode(&[StatePage; 9]) -> Result<Paginas, DecodeError>   ← nibble-decode puro (NOVO)
        │
        ├─ fase 1: Paginas::nome() / Paginas::meta()
        │          └→ Request::Library · DeviceSnapshot.current_name
        │
        └─ fase 2: Paginas::slots(&Dictionary) -> Result<BoardView, DecodeError>
                   └→ Request::Board (backend Real, sem recusa)
```

- **Módulo único:** `packages/core/src/preset_pages.rs`. Função pura, sem fio, sem IO, sem `Session` — é assim que o teste roda contra a captura sem hardware.
- **Dois parser empilhados, não dois módulos.** A fase 2 só é exposta quando os offsets tiverem prova-negativa; até lá `slots()` não existe publicamente. Isso impede que alguém chame meio-decodificado.
- **A análise mora onde o repo já a coloca:** `analysis/`. O `map_state_pages.py` ganha decode nibble antes de votar (corrigir os três defeitos: caminho absoluto `D:\…`, `pp = d[0]` que colapsa 198 pps em 2 balde, e a votação de u8 com `+= 0  # placeholder`).
- **Nenhuma regra nova fora do core (R1).** O dicionário continua sendo a fonte de identidade de efeito; a página só entrega índice/valor.

### 4.1 Interfaces

```rust
/// As 9 páginas de um preset, já nibble-decodificadas.
pub struct Paginas { /* 8 × 96B + 1 × 14B */ }

impl Paginas {
    /// Nome do preset (pg0, offset 2, 16 bytes com pad NUL).
    pub fn nome(&self) -> Result<&str, DecodeError>;
    /// Metadados da pg0. **Fase 2**: os campos aqui ainda não têm offset
    /// provado (`ppType` casa em 166/198, abaixo da barra de prova).
    pub fn meta(&self) -> Result<PresetMeta, DecodeError>;
    /// Fase 2 — só existe quando os offsets estiverem provados.
    pub fn slots(&self, dict: &Dictionary) -> Result<BoardView, DecodeError>;
}

/// O fio → o módulo. Envolvência mínima: 9 páginas entram, sai o domínio.
pub fn decode(paginas: &[StatePage; 9]) -> Result<Paginas, DecodeError>;
```

---

## 5. Fluxo de dados e integração

### 5.1 Fase 1 — nome/inventário (custo zero no fio)

O **boot já baixa as 1782 páginas** (§13.10). O nome não pede tráfego novo: durante o scan o actor decodifica pg0 de cada pp e grava `(pp, nome)` em cache. O `Request::Library` deixa de devolver inventário sem nome.

### 5.2 Fase 2 — palco: **servir do cache, nunca de um `select`**

`Session::state_page` usa `self.current_pp`. Um `device_board(pp)` para **outro** pp exigiria um `select` — ou seja, **mudaria o que o pedal está mostrando** para o app poder ler. Isso é inaceitável num editor.

**Decisão:** `device_board` lê do **cache populado pelo boot** (198 entradas), nunca de um `select` novo. `pp = None` (corrente) é o mesmo caminho. Mesmo desenho do mock, zero efeito colateral no aparelho.

**Consequência declarada:** o cache é do boot. Edição de knob **no hardware depois** do boot não aparece no app sem re-scan — e é isso que a captura mostra (§13.10: knobs físicos não ecoam para o host). O critério "edição no hardware reflete no app" se satisfaz **direto** para o nome (leitura) e para knob **exige read-back**, que é o passo de campo do §8.

### 5.3 Os três pontos de integração

| Ponto | Hoje | Depois |
|---|---|---|
| `actor.rs` ~L702 | `Err("palco do aparelho pendente do decode das páginas 13xx (issue #152)")` | **PREMISSA ERRADA** — o `Request::Board` nunca teve recusa: é projeção pura do `all.prst` nos dois backends (`export_commands.rs:10`). O que passou a existir é o caminho do CACHE, com fallback para a projeção. |
| `actor.rs` ~L225 | *"o nome do pp vem da pagina meta6 cujo layout ainda não foi decifrado"* | substituído pela verdade medida: **pg0 das páginas** |
| `docs/PROTOCOL.md` §13.10 | sem o layout | ganha a seção do layout nibble + nome ✅ |

---

## 6. Tratamento de erro

`DecodeError` tipado (`thiserror`, como `ProtocolError`):

- `SemHeader` — a página não tem os 4B de header.
- `TamanhoInesperado { pagina, len }` — fora de {192, 28}.
- `NomeInvalido { detalhe }` — bytes não-ASCII/NUL no meio, UTF-8 inválido.
- `OffsetsNaoBatem { achado, esperado }` — a prova-negativa falhou.

**Regra:** **nunca um `BoardView` com chute.** Se 197/198 casarem, falha — não se entrega o de 197 e se mente sobre o 198º. A UI já trata erro com `ScreenState.error` + retry; silêncio ou dado inventado é pior que erro, é a mesma lógica do ADR-10.

---

## 7. Estratégia de teste

1. **Dataset inteiro** — `packages/core/tests/preset_pages.rs`: decode das 1782 páginas; nome **198/198 em pg0/offset 2**; recusa página cortada e tamanho errado.
2. **Prova-negativa** — o offset casa em 198/198 **e** é rejeitado quando os bytes são embaralhados, trocados de pp, ou truncados no NUL. Sem este teste o "198/198" só prova que procuramos no lugar certo, não que sabemos onde **não** procurar.
3. **Paridade da fase 2** — contra o artefato `files/patches/all.prst` (slots + knobs das 99 fábrica), no espírito do roundtrip `.prst`.
4. **Gate intocado** — `analysis/validate_golden.py` segue **1782/1782 byte-idêntico**; quirks de boot preservados (select+open duplicados do preset atual).
5. **Fixture + paridade** — se o teste do core precisar de arquivo derivado, o formato é o de `analysis/fixtures/*.jsonl` + `manifest.json` com gate no pytest (`make_fixtures.py` / `test_fixtures_parity`), nunca um binário solto.

---

## 8. Confirmação em campo (dentro do spec, como confirmação)

Hardware disponível. O estatístico propõe, **a manipulação prova**:

- **(a) Nome:** com wire log do FieldDiag ligado, renomear UM user patch no aparelho (SAVE) → o offset do nome é provado por mudança observada, e o mesmo nome tem de aparecer no app **sem re-seed do artefato**.
- **(b) Knob:** girar UM knob no hardware → quais bytes de quais páginas mudam → mapa knob↔offset confirmado. (A sessão 1 não tem edição de knob no fio — §13.10 — então **só** a captura dirigida fecha isto.)
- **(c) Read-back:** prova do critério "edição reflete no app" para knob.

O runbook de campo é entregue junto, no formato dos `H*_CHECKLIST`.

---

## 9. O que o achado faz com a #152

A hipótese central da #152 (**meta6 = nomes**) está **refutada pelos dados**: o payload é `[pp u16BE] 0c 1c 01 40`, **constante em 198/198** — ele não tem onde guardar um nome. E `11000008` (a "tabela de nomes" do §13.10) contém **só estilos/gêneros**, confirmando a reclassificação de 07/10.

O nome está nas **páginas** — que é território da #155. Consequência para o ciclo 2:

- Se a fase 1 entregar nome + meta, **a #152 deve ser reescrita** para o que a meta6 de fato é (flags/constante por pp) — ou fechada com prova-negativa e apontando para a #155.
- Isso **não** é decisão deste spec; fica registrado aqui como recomendação para o ciclo seguinte.

---

## 10. Critérios de aceite (rastreados da #155)

- [x] Layout das 9 páginas decifrado com **prova-negativa** (offset casa em 198/198 e é rejeitado onde não devia) → §7.2
- [x] Nome do pp localizado **nas páginas** (offset 99/99 no `all.prst` + user patches) — **feito e medido: 198/198 em pg0/2** → §2
- [x] Parser no core com teste contra a captura inteira (1.782 páginas) → §7.1
- [x] `device_board` com backend `Real` **sem** a recusa → §5.3 — *a recusa nunca existiu; o que se entregou é o palco do CACHE, sem `select` novo (`board_vem_do_cache_sem_select`)*
- [x] Nome real na biblioteca/navbar sem re-seed; `current_name` honesto → §5.1
- [ ] Campo: edição no hardware (knob/nome) reflete no app → §8 — **ÚNICO PENDENTE: exige o aparelho**
- [x] `analysis/validate_golden.py` segue byte-idêntico → §7.4

---

## 11. Riscos e divergências abertos

1. ~~**`13010005` não existe na S1** (0 mensagens)~~ — **RESOLVIDO (08/10).** O `13010005` é o **ACK de 4B do req PG 8**, e a contagem da S1 filtrava `len >= 20`, que o descartava. Ele existe e não é página: as 9 páginas vêm todas no `13010003` (1 open + 8 reqs), que fecha os 1.782.
2. **`ppType` casa em só 166/198** na pg0 — não serve como prova; precisa de offset próprio na fase 2. Mantido: `slots()` zera `pp_type`/`pp_type_name` de propósito, com a limitação declarada na doc.
3. **Campos ainda não atribuídos** na pg0 (bytes 16..95, fora de `pp`/nome/cadeia/`effectCode`/params). A fase 1 não depende deles; a fase 2 (metadados) sim.
4. ~~**Offsets da fase 2 desconhecidos**~~ — entregues pelo modelo determinístico (`analysis/state_pages_offsets.json`, embutido no binário), com as provas da tabela §10.
5. **Cache vs. live** (§5.2) — declarado como limitação, não escondido.
6. **Endereçamento** — **ACHADO E CORRIGIDO FORA DO ESCOPO ORIGINAL (#156, commit `8b2640c`).** O `ppID` é o índice DECIMAL, mas 6 pontos do core interpretavam como HEX, que só coincide em `'0'..'9'`: 36 dos 99 presets tinham um pp que o fio recusava (`"esperado pp no espaço banco/slot"`) e a UI — que já usava decimal nos artefatos — recebia um pp diferente do backend. Travado por `packages/core/tests/pp_decimal.rs`.
