#!/usr/bin/env python3
"""baseline.py — a baseline da especificacao, COM VERSAO e JUSTIFICATIVA (#23).

Por que o hash nao pode morar dentro de um teste
------------------------------------------------
Ate o H3, "a baseline esta congelada" era um `assert h == "0426d6..."` dentro
de `analysis/tests/test_protocol.py`. Isso trava a mudanca, mas tem dois furos:

1. **Nao ha motivo.** O golden pode mudar por qualquer motivo — inclusive um
   erro de script — e o unico registro do "por que" e a frase na mensagem de
   falha, que ninguem le depois.
2. **Ninguem checa que a motivacao existe.** Bastava colar o hash novo no
   literal do teste e o gate passava. A regra R2/R3 ("captura -> build ->
   validate -> bump com justificativa") era um texto, nao uma verificacao.

O `docs/PROTOCOL.md` §13 e a memoria do protocolo; este arquivo e a memoria
**da baseline**, com as duas coisas que faltavam: numero de versao e motivo.

    uv run python analysis/baseline.py --check        # gate (CI)
    uv run python analysis/baseline.py show
    uv run python analysis/baseline.py bump --motivo "..." --issue "#23"

`bump` recusa escrever sem `--motivo`. Nao e burocracia: e o que separa
"corrigi um bug de terminador SysEx, com 100 linhas cortadas descartadas" de
"rodei o script e o hash mudou".
"""
from __future__ import annotations

import hashlib
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
GOLDEN = os.path.join(ROOT, "docs", "protocol_golden.json")
REGISTRO = os.path.join(ROOT, "analysis", "baseline.json")


def sha256(path: str) -> str:
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def carregar() -> dict:
    with open(REGISTRO, encoding="utf-8") as f:
        return json.load(f)


def salvar(reg: dict) -> None:
    with open(REGISTRO, "w", encoding="utf-8", newline="\n") as f:
        json.dump(reg, f, ensure_ascii=False, indent=2)
        f.write("\n")


def versao_n(v: str) -> tuple:
    """'1.10' > '1.9' (ordena por numero, nao por string)."""
    try:
        return tuple(int(p) for p in str(v).split("."))
    except ValueError:
        return (0,)


def problemas(reg: dict, hash_atual: str) -> list[str]:
    """Tudo que esta errado no registro. Vazio = integro."""
    p: list[str] = []
    if not reg.get("versao"):
        p.append("registro sem 'versao'")
    if reg.get("hash") != hash_atual:
        p.append(
            f"docs/protocol_golden.json mudou sem bump: registro diz "
            f"{str(reg.get('hash'))[:12]}…, arquivo e {hash_atual[:12]}…\n"
            f"    Fluxo: captura -> build_golden -> validate 100% -> "
            f"baseline.py bump --motivo '...' --issue '#N' (R2/R3)"
        )
    hist = reg.get("historico") or []
    if not isinstance(hist, list):
        p.append("'historico' nao e lista")
        hist = []
    # versoes do historico tem de ser crescentes e anteriores a atual
    vs = [h.get("versao") for h in hist if isinstance(h, dict)]
    if any(v is None for v in vs):
        p.append("historico com entrada sem 'versao'")
    limpo = [v for v in vs if v is not None]
    if limpo != sorted(limpo, key=versao_n):
        p.append(f"historico fora de ordem: {limpo}")
    if reg.get("versao") and limpo and versao_n(limpo[-1]) >= versao_n(reg["versao"]):
        p.append(
            f"historico tem {limpo[-1]} que nao e anterior a versao atual "
            f"{reg['versao']}"
        )
    for h in hist:
        if not isinstance(h, dict):
            p.append("historico com entrada que nao e objeto")
            continue
        for campo, rotulo in (("motivo", "motivo"), ("hash", "hash")):
            if not str(h.get(campo) or "").strip():
                p.append(f"historico {h.get('versao')}: {rotulo} vazio")
        if not str(h.get("data") or "").strip():
            p.append(f"historico {h.get('versao')}: data vazia")
    if not str(reg.get("motivo") or "").strip():
        p.append("versao atual sem 'motivo' — nada explica o hash atual")
    if not str(reg.get("data") or "").strip():
        p.append("versao atual sem 'data'")
    return p


def cmd_check() -> int:
    reg = carregar()
    atual = sha256(GOLDEN)
    probs = problemas(reg, atual)
    if probs:
        print("BASELINE REPROVADA:")
        for x in probs:
            print(f" - {x}")
        return 1
    n_hist = len(reg.get("historico") or [])
    print(
        f"baseline v{reg['versao']} integra: {atual[:12]}… · "
        f"{n_hist} versao(oes) no historico, todas com motivo"
    )
    return 0


def cmd_show() -> int:
    reg = carregar()
    print(f"baseline atual: v{reg.get('versao')}  {reg.get('hash','')[:16]}…")
    print(f"  data   : {reg.get('data')}")
    print(f"  motivo : {reg.get('motivo')}")
    if reg.get("issue"):
        print(f"  issue  : {reg['issue']}")
    hist = reg.get("historico") or []
    if not hist:
        print("  (sem historico)")
        return 0
    print("\nhistorico:")
    for h in hist:
        print(f"  v{h.get('versao')}  {str(h.get('hash',''))[:12]}…  {h.get('data')}")
        print(f"      {h.get('motivo')}")
        if h.get("issue"):
            print(f"      ({h['issue']})")
    return 0


def cmd_bump(motivo: str, issue: str, data: str) -> int:
    if not motivo.strip():
        print(
            "bump sem --motivo recusado.\n"
            "  A pergunta que o motivo responde e 'por que o golden mudou?', e "
            "'rodei o script' nao e resposta.\n"
            "  Se a mudanca e um bug de script, escreva QUAL bug."
        )
        return 2
    reg = carregar() if os.path.exists(REGISTRO) else {
        "versao": "1.0", "historico": []
    }
    reg = dict(reg)
    hist = list(reg.get("historico") or [])
    nova = "%d.%d" % (versao_n(reg.get("versao", "1.0"))[0],
                      versao_n(reg.get("versao", "1.0"))[-1] + 1)
    if reg.get("versao"):
        hist.append({
            "versao": reg["versao"],
            "hash": reg.get("hash"),
            "data": reg.get("data"),
            "motivo": reg.get("motivo"),
            "issue": reg.get("issue"),
        })
    reg.update({
        "versao": nova,
        "hash": sha256(GOLDEN),
        "data": data,
        "motivo": motivo.strip(),
        "issue": issue.strip(),
        "historico": hist,
    })
    salvar(reg)
    print(f"baseline v{reg['versao']}  {reg['hash'][:16]}…")
    print(f"  data   : {data}")
    print(f"  motivo : {motivo.strip()}")
    if issue.strip():
        print(f"  issue  : {issue.strip()}")
    print(f"  {len(hist)} versao(oes) no historico.")
    print("\nAgora atualize o §13 do docs/PROTOCOL.md com a mesma justificativa.")
    return 0


def main(argv=None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    cmd = argv[0] if argv else "--check"
    if cmd in ("--check", "check"):
        return cmd_check()
    if cmd == "show":
        return cmd_show()
    if cmd == "bump":
        motivo = issue = ""
        data = "2026-10-04"
        i = 1
        while i < len(argv):
            if argv[i] == "--motivo" and i + 1 < len(argv):
                motivo = argv[i + 1]; i += 2
            elif argv[i] == "--issue" and i + 1 < len(argv):
                issue = argv[i + 1]; i += 2
            elif argv[i] == "--data" and i + 1 < len(argv):
                data = argv[i + 1]; i += 2
            else:
                print(f"argumento desconhecido: {argv[i]}")
                return 2
        return cmd_bump(motivo, issue, data)
    print(__doc__.strip())
    return 2


if __name__ == "__main__":
    if not sys.stdout.encoding or "utf" not in sys.stdout.encoding.lower():
        try:
            sys.stdout.reconfigure(errors="replace")
        except Exception:
            pass
    raise SystemExit(main())