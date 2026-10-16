#!/usr/bin/env python3
"""check_harness.py — higiene do harness do agente (`AGENTS.md`, `knowledge.md`, `.agents/**`).

POR QUE ISTO EXISTE. O harness e lido por agentes e nao tinha gate nenhum: a
auditoria de 14/10/2026 (#249) provou **duas vezes** por leitura manual que
"estava tudo certo" quando nao estava (caminho morto, regra re-enunciada em 6
arquivos). Leitura manual nao pega regressao: o arquivo que apodrece e o que
ninguem relê. Este script e a versao mecanica das tres sondas que fecharam a
#249 — elas moravam num diretorio de rascunho e por isso nunca rodavam.

QUATRO CHECAGENS:
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
    a dona precisa enunciar a regra e nenhuma outra SKILL pode re-enuncia-la.
    Apontar para a dona e permitido; re-enunciar e o defeito (a regra que mora em
    dois lugares sincroniza em um e apodrece no outro).

Uso: python3 .github/scripts/check_harness.py   (exit 1 se houver violacao)
     python3 .github/scripts/check_harness.py --list
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
# Duas regras tem mais de uma dona por decisao do `.agents/README.md`: a "visao
# por camada" de teste e da `testing`, a regra do Rust e da `rust-practices`.
REGRAS: dict[str, tuple[tuple[str, ...], str]] = {
    "o aparelho e a fonte / medir, nao deduzir": (
        (".agents/skills/hardware-dev/SKILL.md",),
        r"medir, n[ãa]o deduzir|aparelho (é|e) a fonte|\bMEÇA\b",
    ),
    "ciclo issue→branch→PR→CI→merge": (
        (".agents/skills/github-flow/SKILL.md",),
        r"CI verde antes do merge|s[óo] com o merge verde|merge \(squash\)|"
        r"→ CI verde → merge",
    ),
    "conventional commits + footer": (
        (".agents/skills/github-flow/SKILL.md",),
        r"conventional commit|footer Codebuff|tipo\(escopo\)",
    ),
    "onde cada teste vive (src/ x tests/)": (
        (
            ".agents/skills/rust-practices/SKILL.md",
            ".agents/skills/testing/SKILL.md",
        ),
        r"Nunca mova unit[áa]rio|internals (s[ãa]o )?INVIS[ÍI]VEIS|"
        r"mod tests` DENTRO|s[óo] API p[úu]blica → `tests/`",
    ),
    "contadores s[óo] no INDEX §6": (
        (".agents/skills/docs-sync/SKILL.md",),
        r"pertence a `?docs/INDEX\.md`? §6|(vive|fica) s[óo] n[oa] `?(docs/)?INDEX|"
        r"s[óo] no `?INDEX\.md`? §6",
    ),
    "armadilhas de host (cargo PATH, toolchain)": (
        ("knowledge.md",),
        r"add_cargo_path|RUSTUP_TOOLCHAIN=|(n[ãa]o|N[ÃA]O) tem MSVC|"
        r"(n[ãa]o|N[ÃA]O) trocar de alvo|fix permanente",
    ),
}

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
    except ModuleNotFoundError:  # pragma: no cover
        yaml = None  # type: ignore[assignment]
    for sk in skills:
        rel = str(sk.relative_to(RAIZ)).replace("\\", "/")
        txt = sk.read_text(encoding="utf-8")
        bloco = re.match(r"^---\n(.*?)\n---\n", txt, re.DOTALL)
        if not bloco:
            viola("FRONT-MATTER", f"{rel}: sem bloco de front-matter")
            continue
        if yaml is None:  # pragma: no cover
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
        ausentes = [d for d in donas if d not in fontes]
        if ausentes:
            viola("FONTE UNICA", f"{regra}: dona fora do escopo do gate: {', '.join(ausentes)}")
            continue
        if not any(rx.search(b) for d in donas for b in blocos(fontes[d])):
            viola("FONTE UNICA", f"{regra}: nenhuma dona ({', '.join(donas)}) enuncia a regra — orfã ou padrao morto")
        apelidos: set[str] = set()
        for d in donas:
            p = pathlib.PurePosixPath(d)
            apelidos |= {p.name, p.name.replace(".md", ""), p.parent.name}
        ponteiro = re.compile("|".join(re.escape(x) for x in sorted(apelidos) if x))
        for rel, texto in fontes.items():
            if rel in donas or "/skills/" not in rel:
                continue  # prompt/README/knowledge citam a premissa como ENTRADA
            for b in blocos(texto):
                if rx.search(b) and not ponteiro.search(b):
                    viola("FONTE UNICA", f"{regra}: re-enunciada em {rel} — «{b[:110]}»")


def main() -> int:
    fontes = {rel: ler(rel) for rel in FONTES if (RAIZ / rel).exists()}
    faltando = [rel for rel in FONTES if rel not in fontes]
    if faltando:
        viola("INVENTARIO", f"arquivo de harness citado e ausente: {', '.join(faltando)}")
    checa_caminhos(fontes)
    checa_frontmatter()
    checa_papeis()
    checa_fonte_unica(fontes)

    if "--list" in sys.argv:
        print(f"fontes: {len(fontes)} arquivos | regras de fonte unica: {len(REGRAS)}")
        print(f"caminhos de proposito (allowlist): {len(CAMINHOS_DE_PROPOSITO)}")
        return 0
    print(f"harness: {len(fontes)} arquivos | violacoes: {len(VIOLACOES)}")
    for v in VIOLACOES:
        print(f"  {v}")
    return 1 if VIOLACOES else 0


if __name__ == "__main__":
    sys.exit(main())
