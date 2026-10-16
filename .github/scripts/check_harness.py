#!/usr/bin/env python3
"""check_harness.py — higiene do harness do agente (`AGENTS.md`, `knowledge.md`, `.agents/**`).

POR QUE ISTO EXISTE. O harness e lido por agentes e nao tinha gate nenhum: a
auditoria de 14/10/2026 (#249) provou **duas vezes** por leitura manual que
"estava tudo certo" quando nao estava (caminho morto, regra re-enunciada em 6
arquivos). Leitura manual nao pega regressao: o arquivo que apodrece e o que
ninguem relê. Este script e a versao mecanica das tres sondas que fecharam a
#249 — elas moravam num diretorio de rascunho e por isso nunca rodavam.

CINCO CHECAGENS:
 1. CAMINHO — todo caminho do repo citado num code span existe. Caminho curto
    (`actor/estado.rs`) passa quando o proprio arquivo declara a rota-cheia de
    uma base que existe (`packages/app/api/src/actor/`): apontar para base real
    e navegacao, nao caminho morto.
 2. FRONT-MATTER — cada `.agents/skills/*/SKILL.md` tem YAML parseavel, `name`
    igual ao nome da pasta e `description` que dispara (>= 40 chars); e os 6
    prompts existem.
 3. CONTRATO DE PAPEL — todo `.agents/agents/*.md` e citado no README da equipe
    e todo papel citado ali existe: nem orfao (contrato que ninguem acha), nem
    fantasma (README apontando para arquivo que nao existe).
 4. FONTE UNICA — as regras transversais da tabela de fontes unicas tem UMA casa:
    a dona precisa enunciar a regra e nenhum outro arquivo do harness pode
    re-enuncia-la. Apontar para a dona e permitido; re-enunciar e o defeito (a
    regra que mora em dois lugares sincroniza em um e apodrece no outro).
    O ESCOPO e todo o harness, nao so `/skills/`: a #269 mediu o defeito no lugar
    onde ele acontece de verdade — o `AGENTS.md` nasceu repetindo a lista de
    "nao fazer" do `knowledge.md` e o "modo ausente" do `.agents/README.md`, e
    nenhum dos tres e skill. Guarda que so olha skill nao pega o proximo arquivo
    de entrada que copiar (foi o pedido explicito da #269).
 5. TABELA — a tabela de fontes unicas do `.agents/README.md` e GERADA do REGRAS
    (abaixo): o gate emite o bloco entre os marcadores `fontes-unica:` e reprova a
    tabela editada a mao. Escrita a mao ela listava 9 regras enquanto o gate cobria
    6 — a divergencia era invisivel ate alguem contar as duas colunas.

Uso: python3 .github/scripts/check_harness.py   (exit 1 se houver violacao)
     python3 .github/scripts/check_harness.py --list
     python3 .github/scripts/check_harness.py --tabela   (reescreve a tabela do README)
"""
from __future__ import annotations

import os
import pathlib
import re
import sys

# O console do Windows abre em cp1252 e o relatorio tem seta/acento: sem isto o
# proprio gate morre com UnicodeEncodeError (aconteceu na sonda de 14/10).
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
except (AttributeError, OSError):  # pragma: no cover — streams sem `reconfigure`
    pass

# `parents[2]`: este arquivo mora em `.github/scripts/` (armadilha do #168).
RAIZ = pathlib.Path(__file__).resolve().parents[2]
AGENTS = RAIZ / ".agents"

# Os arquivos que ASSUMEM o trabalho de agente. O escopo e o harness, nao o repo:
# doc de area (`docs/**`) tem os gates de doc (issue templates, INDEX, baseline).
FONTES = ["AGENTS.md", "knowledge.md"] + sorted(
    str(p.relative_to(RAIZ)).replace("\\", "/") for p in AGENTS.rglob("*.md")
)

# Caminhos citados DE PROPOSITO que nao existem aqui — com o motivo obrigatorio.
CAMINHOS_DE_PROPOSITO: dict[str, str] = {
    ".agents/types/": "layout do `/init` do Codebuff: citado para dizer que NAO existe neste repo",
    "tests/x.rs": "nome ILUSTRATIVO (a tabela compara pytest × cargo test)",
    "tests/test_x.py": "nome ILUSTRATIVO (par do `tests/x.rs` na mesma tabela)",
}

EXTENSOES = (
    "md", "py", "json", "rs", "toml", "yml", "yaml", "ts", "tsx",
    "ps1", "sh", "prst", "jsonl", "txt", "exe", "css",
)
RAIZES_DE_CAMINHO = ("docs/", ".github/", "packages/", "tools/", "packages", ".agents/",
                     "files/", "packaging/", "analysis/", "assets/")
PREFIXOS_QUE_NAO_SAO_CAMINHO = (
    "http", "--", "cargo ", "pnpm ", "gh ", "git ", "uv ", "python3 ", "npm ",
    "core/", "ui/", "src-tauri",
)

# ── 4. FONTE UNICA ────────────────────────────────────────────────────────────
# regra -> (arquivos DONOS, padrao que caracteriza a ENUNCIACAO da regra).
#
# A CHAVE e a linha da tabela de fontes unicas do `.agents/README.md`: o gate EMITE
# a tabela (`--tabela`) e reprova a que foi editada a mao. Antes disso a tabela era
# escrita a mao e listava 9 regras enquanto o gate cobria 6 — o REGRAS era a copia
# escondida dela; agora a tabela E o REGRAS.
#
# A dona pode ser um doc de area (`docs/dev/ARCHITECTURE.md`), nao so skill: o gate
# le a dona do disco — as varreduras de caminho e front-matter seguem no harness.
# Duas regras tem mais de uma dona por decisao do `.agents/README.md`: a "visao
# por camada" de teste e da `testing`, a regra do Rust e da `rust-practices`.
REGRAS: dict[str, tuple[tuple[str, ...], str]] = {
    "O aparelho é a fonte; medir, não deduzir (ADR-13)": (
        (".agents/skills/hardware-dev/SKILL.md",),
        r"medir, n[ãa]o deduzir|aparelho (é|e) a fonte|\bMEÇA\b",
    ),
    "Ciclo issue→branch→PR→CI→merge": (
        (".agents/skills/github-flow/SKILL.md",),
        r"CI verde antes do merge|s[óo] com o merge verde|merge \(squash\)|"
        r"→ CI verde → merge",
    ),
    "Conventional commits + footer Codebuff": (
        (".agents/skills/github-flow/SKILL.md",),
        r"conventional commit|footer Codebuff|tipo\(escopo\)",
    ),
    "Onde cada teste vive (unitário `src/` × contrato `tests/`)": (
        (
            ".agents/skills/rust-practices/SKILL.md",
            ".agents/skills/testing/SKILL.md",
        ),
        r"Nunca mova unit[áa]rio|internals (s[ãa]o )?INVIS[ÍI]VEIS|"
        r"mod tests` DENTRO|s[óo] API p[úu]blica → `tests/`",
    ),
    "Contadores (testes/cobertura/jobs) só no `INDEX.md` §6": (
        (".agents/skills/docs-sync/SKILL.md",),
        r"pertence a `?docs/INDEX\.md`? §6|(vive|fica) s[óo] n[oa] `?(docs/)?INDEX|"
        r"s[óo] no `?INDEX\.md`? §6",
    ),
    "Armadilhas de host (cargo/PATH, toolchain pinada)": (
        ("knowledge.md",),
        r"add_cargo_path|RUSTUP_TOOLCHAIN=|(n[ãa]o|N[ÃA]O) tem MSVC|"
        r"(n[ãa]o|N[ÃA]O) trocar de alvo|fix permanente",
    ),
    "Cadeia de release rc→promote→tag": (
        (".agents/skills/release/SKILL.md",),
        r"rc\s*→[^\n]{0,40}promote",
    ),
    "Camada do app (`packages/app/api`: actor, commands, DTO/IPC)": (
        (".agents/skills/app-dev/SKILL.md",),
        r"commands, actor e DTOs|DeviceActor|dono( único)? da Session",
    ),
    "Estrutura e limites das camadas do front": (
        ("docs/dev/ARCHITECTURE.md",),
        r"Cada camada é definida pelo que ela \*\*proíbe\*\*",
    ),
    "Papéis da equipe + estado da demanda (`flow:*`)": (
        (".agents/agents/README.md",),
        r"labels `?flow:\*|Estado da demanda",
    ),
    "Modo ausente do dono (#202)": (
        (".agents/README.md",),
        r"modo ausente do dono|oper[ao] sem perguntar|fila aut[ôo]noma",
    ),
    # A regra aqui e a EXISTENCIA de uma lista unica: o padrao e o cabecalho da
    # secao, nao o texto de cada item (cada item ja aponta a casa dele, e o
    # `hardware-dev` e o `github-flow` enunciam as regras deles nas proprias
    # casas). Duas secoes "nao fazer" = duas listas que divergem.
    "O que NÃO fazer (lista única)": (
        ("AGENTS.md",),
        r"^#+ ?\d*\.? ?(o )?(que )?n[ãa]o fazer",
    ),
    "Gates locais por área (antes do PR)": (
        ("docs/CONTRIBUTING.md",),
        r"^## 4\. Gates locais por área|gates locais por área",
    ),
}

# Nao servem de ponteiro: casam em qualquer bloco e o check aprovaria por engano.
APELIDOS_GENERICOS = {"docs", "src", "tests", "skills", "README", "README.md", "md"}

VIOLACOES: list[str] = []


def viola(secao: str, msg: str) -> None:
    VIOLACOES.append(f"[{secao}] {msg}")


def ler(rel: str) -> str:
    return (RAIZ / rel).read_text(encoding="utf-8")


def code_spans(texto: str) -> list[str]:
    return [m.group(1).strip() for m in re.finditer(r"`([^`\n]+)`", texto)]


def parece_caminho(s: str) -> bool:
    if not s or s.startswith(PREFIXOS_QUE_NAO_SAO_CAMINHO):
        return False
    if any(x in s for x in ("*", "<", "{", "…", " ")) and not re.fullmatch(
        rf"[\w./\- ]+\.({'|'.join(EXTENSOES)})", s
    ):
        return False
    if re.fullmatch(rf"[\w./\-]+\.({'|'.join(EXTENSOES)})", s):
        return True
    return "/" in s and s.startswith(RAIZES_DE_CAMINHO) and len(s) < 80


def bases_do_arquivo(texto: str) -> list[pathlib.Path]:
    """Diretorios que o PROPRIO arquivo declara (rota-cheia citada e existente)."""
    bases: list[pathlib.Path] = []
    for s in code_spans(texto):
        alvo = (RAIZ / s.rstrip("/")).resolve()
        if alvo.is_dir() and (alvo == RAIZ or RAIZ in alvo.parents):
            bases.append(alvo)
    return bases


def resolve(caminho: str, bases: list[pathlib.Path]) -> bool:
    if (RAIZ / caminho).exists():
        return True
    for base in bases:
        atual = base
        while True:  # a base e cada ancestral dela: `actor/x.rs` perto da rota-cheia
            if (atual / caminho).exists():
                return True
            if atual == RAIZ:
                break
            atual = atual.parent
    return False


def checa_caminhos(fontes: dict[str, str]) -> None:
    vistos: dict[str, list[str]] = {}
    for rel, texto in fontes.items():
        bases = bases_do_arquivo(texto)
        for s in code_spans(texto):
            if not parece_caminho(s):
                continue
            if "/" not in s:  # nome curto: a rota-cheia aparece na 1a mencao
                continue
            if s in CAMINHOS_DE_PROPOSITO:
                continue
            if not resolve(s, bases):
                vistos.setdefault(s, []).append(rel)
    for caminho, arquivos in sorted(vistos.items()):
        viola("CAMINHO", f"{caminho} nao existe (citado em {', '.join(sorted(arquivos))})")


def checa_frontmatter() -> None:
    skills = sorted((AGENTS / "skills").glob("*/SKILL.md"))
    if not skills:
        viola("FRONT-MATTER", "nenhuma skill encontrada em .agents/skills/")
    try:
        import yaml  # noqa: PLC0415 — dependencia declarada no pyproject
    except ImportError:
        # Sem `pyyaml` a checagem de front-matter seria PULADA em silencio e o gate
        # imprimiria `violacoes: 0`: "verde por omissao", o pior modo de falhar de um
        # verificador — o defeito passa a morar nele. ImportError (nao so
        # ModuleNotFoundError) porque qualquer falha de import cega a checagem.
        viola(
            "FRONT-MATTER",
            "`pyyaml` ausente: a checagem de front-matter seria pulada em silencio "
            "(verde por omissao) — rode `uv sync --all-groups`",
        )
        yaml = None  # type: ignore[assignment]
    for sk in skills:
        rel = str(sk.relative_to(RAIZ)).replace("\\", "/")
        txt = sk.read_text(encoding="utf-8")
        bloco = re.match(r"^---\n(.*?)\n---\n", txt, re.DOTALL)
        if not bloco:
            viola("FRONT-MATTER", f"{rel}: sem bloco de front-matter")
            continue
        if yaml is None:  # a violacao ja foi registrada acima; nao ha o que parsear
            continue
        try:
            dados = yaml.safe_load(bloco.group(1)) or {}
        except Exception as e:  # noqa: BLE001 — o gate reporta, nao aborta
            viola("FRONT-MATTER", f"{rel}: YAML invalido ({e})")
            continue
        if dados.get("name") != sk.parent.name:
            viola("FRONT-MATTER", f"{rel}: name={dados.get('name')!r} != pasta={sk.parent.name!r}")
        desc = (dados.get("description") or "").strip()
        if len(desc) < 40:
            viola("FRONT-MATTER", f"{rel}: description curta/ausente ({desc!r}) — ela e o gatilho")
    prompts = sorted((AGENTS / "prompts").glob("*.md"))
    if len(prompts) != 6:
        viola("FRONT-MATTER", f".agents/prompts: {len(prompts)} arquivos (esperado 6)")


def checa_papeis() -> None:
    contratos = sorted((AGENTS / "agents").glob("*.md"))
    if not contratos:
        viola("PAPEL", ".agents/agents/ sem contratos de papel")
        return
    indice = AGENTS / "agents" / "README.md"
    if not indice.exists():
        viola("PAPEL", ".agents/agents/README.md (o indice da equipe) nao existe")
        return
    texto = indice.read_text(encoding="utf-8")
    for arq in sorted(AGENTS.glob("agents/*.md")):
        if arq.name == "README.md":
            continue
        if arq.name not in texto:
            viola("PAPEL", f"{arq.name} existe mas nao e citado em .agents/agents/README.md")
    for citado in sorted(set(re.findall(r"`?\.agents/agents/([\w.\-]+\.md)`?", texto))):
        if not (AGENTS / "agents" / citado).exists():
            viola("PAPEL", f".agents/agents/README.md cita {citado}, que nao existe")


def blocos(texto: str) -> list[str]:
    """Agrupa linhas em blocos logicos (paragrafo/bullet/linha de tabela).

    Os arquivos sao escritos em ~80 colunas: o ponteiro cai na 2a linha do
    bullet. Medir linha a linha acusaria ponteiro como enunciado.
    """
    out, atual = [], []
    for linha in texto.splitlines():
        if not linha.strip():
            if atual:
                out.append(" ".join(atual))
                atual = []
            continue
        if re.match(r"^\s*(- |\* |\d+\. |[|>])", linha) and atual:
            out.append(" ".join(atual))
            atual = []
        atual.append(linha.strip())
    if atual:
        out.append(" ".join(atual))
    return out


def checa_fonte_unica(fontes: dict[str, str]) -> None:
    for regra, (donas, padrao) in REGRAS.items():
        rx = re.compile(padrao, re.IGNORECASE)
        textos: dict[str, str] = {}
        for d in donas:
            caminho = RAIZ / d
            if not caminho.exists():
                viola("FONTE UNICA", f"{regra}: dona declarada nao existe: {d}")
                continue
            # a dona pode ser doc de area, fora da varredura do harness: le do disco
            textos[d] = fontes.get(d) or caminho.read_text(encoding="utf-8")
        if len(textos) != len(donas):
            continue
        if not any(rx.search(b) for d in donas for b in blocos(textos[d])):
            viola("FONTE UNICA", f"{regra}: nenhuma dona ({', '.join(donas)}) enuncia a regra — orfã ou padrao morto")
        apelidos: set[str] = set()
        for d in donas:
            p = pathlib.PurePosixPath(d)
            apelidos |= {p.name, p.name.replace(".md", ""), p.parent.name}
        # Nome generico nao e ponteiro: "docs"/"README"/"skills" casariam em quase
        # todo bloco e o check passaria a aprovar re-enunciacao por engano.
        apelidos -= APELIDOS_GENERICOS
        ponteiro = re.compile("|".join(re.escape(x) for x in sorted(apelidos) if x))
        for rel, texto in fontes.items():
            if rel in donas:
                continue  # a dona enuncia; ela nao precisa apontar para si mesma
            for b in blocos(texto):
                if rx.search(b) and not ponteiro.search(b):
                    viola("FONTE UNICA", f"{regra}: re-enunciada em {rel} — «{b[:110]}»")


# ── 5. TABELA ────────────────────────────────────────────────────────────────
# A tabela de fontes unicas do README e SAIDA do gate, nao texto de doc: quem manda
# e o REGRAS acima. Editar a mao reprova; `--tabela` reescreve.
TABELA_DOC = ".agents/README.md"
TABELA_INICIO = "<!-- fontes-unica:inicio"
TABELA_FIM = "<!-- fontes-unica:fim -->"


def apelido_dona(rel: str) -> str:
    '''`rust-practices` para uma skill; o caminho inteiro para qualquer outra casa.'''
    p = pathlib.PurePosixPath(rel)
    if p.name == "SKILL.md" and p.parent.parent.name == "skills":
        return p.parent.name
    return rel


def linhas_da_tabela() -> list[str]:
    linhas = ["| Assunto | Casa (dono) | Os demais |", "|---|---|---|"]
    for regra, (donas, _) in REGRAS.items():
        casas = " + ".join(f"`{apelido_dona(d)}`" for d in donas)
        linhas.append(f"| {regra} | {casas} | apontam |")
    return linhas


def bloco_gerado() -> str:
    return "\n".join(
        [
            f"{TABELA_INICIO} (gerada por `.github/scripts/check_harness.py --tabela`; "
            "editar a mao reprova) -->"
        ]
        + linhas_da_tabela()
        + [TABELA_FIM]
    )


def bloco_no_readme(texto: str) -> str | None:
    '''O bloco entre os marcadores — `None` quando eles nao existem (ou estao tortos).'''
    i = texto.find(TABELA_INICIO)
    fim = texto.find(TABELA_FIM)
    if i < 0 or fim < i:
        return None
    return texto[i : fim + len(TABELA_FIM)]


def checa_tabela() -> None:
    caminho = RAIZ / TABELA_DOC
    if not caminho.exists():
        viola("TABELA", f"{TABELA_DOC}: ausente (a tabela de fontes unicas mora nele)")
        return
    atual = bloco_no_readme(caminho.read_text(encoding="utf-8"))
    if atual is None:
        viola("TABELA", f"{TABELA_DOC}: sem os marcadores `fontes-unica:inicio/fim`")
        return
    if atual != bloco_gerado():
        viola(
            "TABELA",
            f"{TABELA_DOC}: a tabela divergiu do REGRAS do gate — rode "
            "`check_harness.py --tabela` (a tabela nao se edita a mao)",
        )


def gera_tabela() -> int:
    '''Escreve o bloco no README. LF explicito: o repo e `* text=auto eol=lf` (docs-sync 10).'''
    caminho = RAIZ / TABELA_DOC
    texto = caminho.read_text(encoding="utf-8")
    atual = bloco_no_readme(texto)
    if atual is None:
        print(f"{TABELA_DOC}: marcadores `fontes-unica:` ausentes — nada a gerar")
        return 1
    caminho.write_text(texto.replace(atual, bloco_gerado()), encoding="utf-8", newline="\n")
    print(f"{TABELA_DOC}: tabela gerada ({len(REGRAS)} regras)")
    return 0


def main() -> int:
    if "--tabela" in sys.argv:
        return gera_tabela()

    fontes = {rel: ler(rel) for rel in FONTES if (RAIZ / rel).exists()}
    faltando = [rel for rel in FONTES if rel not in fontes]
    if faltando:
        viola("INVENTARIO", f"arquivo de harness citado e ausente: {', '.join(faltando)}")
    checa_caminhos(fontes)
    checa_frontmatter()
    checa_papeis()
    checa_fonte_unica(fontes)
    checa_tabela()

    if "--list" in sys.argv:
        print(f"fontes: {len(fontes)} arquivos | regras de fonte unica: {len(REGRAS)}")
        print(f"tabela de fontes unicas: {TABELA_DOC} — gerada de REGRAS, {len(REGRAS)} regras")
        print(f"caminhos de proposito (allowlist): {len(CAMINHOS_DE_PROPOSITO)}")
        return 0
    print(f"harness: {len(fontes)} arquivos | violacoes: {len(VIOLACOES)}")
    for v in VIOLACOES:
        print(f"  {v}")
    return 1 if VIOLACOES else 0


if __name__ == "__main__":
    sys.exit(main())
