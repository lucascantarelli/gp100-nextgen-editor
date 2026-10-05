#!/usr/bin/env python3
"""validate_core_capture.py — juiz de uma captura feita pelo PROPRIO gp100-core (#23).

O problema que este script resolve
----------------------------------
`analysis/validate_golden.py` valida as **4 capturas do Suite** e so elas: os
enderecos, as janelas de tempo e as provas estao hard-coded por sessao. Ele nao
sabe o que fazer com uma sessao nova, e menos ainda com uma sessao vinda do
gp100-core (que e justamente o DoD do H3: "nova captura com o gp100-core no fio
-> regenerar o golden").

Este juiz e o geral: nao conhece senao, senao nem janela. Ele diz, frame a
frame, se o que o gp100-core fez no fio esta coberto pela especificacao e se os
bytes batem.

    uv run python analysis/validate_core_capture.py <log.jsonl> [--markdown]
    uv run python analysis/validate_core_capture.py <log.jsonl> --json

Saidas (exit codes)
------------------
    0  100% dos frames cobertos E batendo com a spec
    1  algum frame bate no endereco mas DIVERGE no payload  (suspeito de bug)
    2  algum frame NAO tem template nenhum                (fora da spec)
    3  o arquivo nao carregou (formato errado, ou 0 eventos — o bug do H3)

As tres situacoes de erro sao distintas de proposito. "Divergiu no payload" e
"o gp100-core inventou um endereco" sao problemas de Naturezas opostas: o
primeiro e bug no codigo, o segundo e captura nova que exige ADR (R1).
"""
from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wirelog  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
GOLDEN = os.path.join(ROOT, "docs", "protocol_golden.json")

# Um payload 'var' casa com qualquer valor. Aceitar um payload 100% variavel
# como 'ok' seria exatamente o erro do golden v1.0 (validar um padrao derivado
# de dado fabricado contra dado fabricado, e chamar o resultado de prova). Por
# isso `pad_so_var` separa os dois casos no relatorio: divergencia de payload
# (suspeita de bug) e cobertura generica (falta de spec).
MIN_CONST_BYTES = 0


def casa_segmento(seg: dict, data: bytes, pos: int) -> int | None:
    """Consome um segmento. Devolve a nova posicao, ou None se nao casa."""
    if seg.get("kind") == "const":
        hx = seg.get("hex", "")
        if not hx:
            return pos
        alvo = bytes.fromhex(hx)
        if data[pos : pos + len(alvo)] != alvo:
            return None
        return pos + len(alvo)
    n = int(seg.get("count", 0))
    if pos + n > len(data):
        return None
    return pos + n


def casa_padrao(pad: dict, data: bytes) -> bool:
    """O payload casa com o padrao do golden?

    Trata os tres formatos de `payload_notation`: `const`, `var`/`mixed`
    (lista de segmentos) e `by-len` (escolhe o subpadrao pelo comprimento).
    """
    if pad is None:
        return False
    kind = pad.get("kind")
    if kind == "by-len":
        alvo = pad.get("by_len", {}).get(str(len(data)))
        return casa_padrao(alvo, data) if alvo else False
    segs = pad.get("segments")
    if segs is None:
        # formato simples: const com hex, ou var com len
        if "hex" in pad:
            return data == bytes.fromhex(pad["hex"])
        return len(data) == int(pad.get("len", len(data)))
    pos = 0
    n_const = 0
    for seg in segs:
        novo = casa_segmento(seg, data, pos)
        if novo is None:
            return False
        pos = novo
        if seg.get("kind") == "const":
            n_const += len(bytes.fromhex(seg.get("hex", ""))) // 2
    if pos != len(data):
        return False
    return n_const >= MIN_CONST_BYTES


def endpoint_de(t: dict, lado: str) -> tuple[str, str] | None:
    ep = t.get(lado)
    if not ep:
        return None
    return ep.get("func", ""), ep.get("addr", "")


def carregar_golden() -> list[dict]:
    with open(GOLDEN, encoding="utf-8") as f:
        return json.load(f)["transactions"]


def julgue(eventos: list[dict], templates: list[dict]) -> dict:
    """Junta cada frame ao template que o cobre. Devolve o relatorio."""
    ok = []
    divergem = []
    fora = []
    for e in eventos:
        func, addr = e["func"], e["addr"]
        cands = []
        for t in templates:
            ep = endpoint_de(t, "in" if e["dir"] == "in" else "out")
            if ep and ep[0] == func and ep[1] == addr:
                cands.append(t)
        if not cands:
            fora.append({**e, "motivo": "nenhum template neste endpoint"})
            continue
        pad_key = "response_payload" if e["dir"] == "in" else "request_payload"
        if any(casa_padrao(t.get(pad_key), bytes.fromhex(e["data"])) for t in cands):
            ok.append(e)
            continue
        # Endereco casa, payload nao. Distingue de "so havia padrao generico"
        # — se algum template do endpoint e 100% var, entao NAO ha evidencia
        # para dizer que divergiu; e falta de spec, nao bug.
        todos_genericos = all(
            t.get(pad_key) is not None
            and pad_so_var(t[pad_key])
            for t in cands
        )
        divergem.append({
            **e,
            "motivo": "payload nao casa com nenhum template do endpoint",
            "generico": todos_genericos,
        })
    total = len(eventos)
    return {
        "total": total,
        "ok": len(ok),
        "divergem": divergem,
        "fora": fora,
        # frames cobertos por um template, independente do payload
        "cobertos": total - len(fora),
    }


def pad_so_var(pad: dict) -> bool:
    """True se o padrao nao tem NENHUM byte const (nao afirma nada sobre valor)."""
    segs = pad.get("segments") or []
    return bool(segs) and all(s.get("kind") != "const" for s in segs)


def texto(rel: dict, fonte: str, diag: dict) -> str:
    L = []
    L.append("== JULIZ DA CAPTURA DO GP100-CORE (H3) ==")
    L.append(f"  fonte   : {fonte}")
    L.append(f"  eventos : {rel['total']}")
    L.append(f"  cobertura: {rel['cobertos']}/{rel['total']} frame(s) tem template")
    L.append(f"  batendo  : {rel['ok']}/{rel['total']}")
    if diag.get("linhas_ilegiveis") or diag.get("nao_eventos"):
        L.append(
            f"  [!] {diag['linhas_ilegiveis']} linha(s) ilegivel(is) e "
            f"{diag['nao_eventos']} linha(s) fora do schema de fio"
        )
    L.append("")
    if rel["fora"]:
        L.append(f"-- {len(rel['fora'])} frame(s) FORA DA SPEC --")
        for e in rel["fora"][:20]:
            L.append(f"   {e['dir']:3} {e['func']}/{e['addr']} t={e['t']} {e['motivo']}")
    if rel["divergem"]:
        L.append(f"-- {len(rel['divergem'])} frame(s) DIVERGEM no payload --")
        for e in rel["divergem"][:20]:
            marca = " (padrao generico: e FALTA DE SPEC, nao bug)" if e["generico"] else ""
            L.append(f"   {e['dir']:3} {e['func']}/{e['addr']} t={e['t']} {e['motivo']}{marca}")
    if not rel["fora"] and not rel["divergem"]:
        L.append("  tudo coberto e batendo com a spec.")
    return "\n".join(L)


def markdown(rel: dict, fonte: str) -> str:
    L = ["### Captura do gp100-core vs spec (H3)", ""]
    L.append(f"- arquivo: `{os.path.basename(fonte)}` · {rel['total']} frames")
    L.append(f"- cobertura: **{rel['cobertos']}/{rel['total']}** · "
             f"batendo: **{rel['ok']}/{rel['total']}**")
    if rel["fora"]:
        L.append(f"- ⚠️ **{len(rel['fora'])} frame(s) fora da spec** (exige ADR, R1):")
        for e in rel["fora"][:10]:
            L.append(f"  - `{e['dir']} {e['func']}/{e['addr']}`")
    if rel["divergem"]:
        bugs = [e for e in rel["divergem"] if not e["generico"]]
        gen = [e for e in rel["divergem"] if e["generico"]]
        if bugs:
            L.append(f"- 🔴 **{len(bugs)} divergência(s) de payload** (suspeito de bug):")
            for e in bugs[:10]:
                L.append(f"  - `{e['dir']} {e['func']}/{e['addr']}` t={e['t']}")
        if gen:
            L.append(f"- ⚪ {len(gen)} frame(s) casam só com padrão genérico (falta de spec)")
    return "\n".join(L)


def main(argv=None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    args = [a for a in argv if not a.startswith("--")]
    flags = [a for a in argv if a.startswith("--")]
    if not args:
        print(__doc__.strip())
        return 3
    fonte = args[0]
    eventos, diag = wirelog.load(fonte)
    if not eventos:
        print(f"{fonte}: 0 EVENTOS. Formato inesperado? Use analysis/wirelog.py "
              f"para inspecionar.")
        print(f"  diag: {diag}")
        return 3
    rel = julgue(eventos, carregar_golden())
    if "--json" in flags:
        print(json.dumps({"arquivo": fonte, **rel}, ensure_ascii=False, indent=1))
    elif "--markdown" in flags:
        print(markdown(rel, fonte))
    else:
        print(texto(rel, fonte, diag))
    if rel["fora"]:
        return 2
    if rel["divergem"]:
        return 1
    return 0


if __name__ == "__main__":
    if not sys.stdout.encoding or "utf" not in sys.stdout.encoding.lower():
        try:
            sys.stdout.reconfigure(errors="replace")
        except Exception:
            pass
    raise SystemExit(main())