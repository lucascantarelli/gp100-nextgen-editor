#!/usr/bin/env python3
"""build_golden.py — extrai templates de transação (request→resposta) de todas
as capturas e emite docs/protocol_golden.json (especificação executável).

Método:
  1. Carrega as 4 sessões (loaders padrão: repair de aspa, trim no 1º F7).
  2. Segmenta por gaps >30s (fases: boot / edição / save).
  3. Emparelha OUT→IN por fila (dir,func,addr) com timeout 3s; para pushes
     13xx usa fallback de família (prefixo de 3 bytes do endereço, FIFO global).
  4. Templates = (dir_out, func_out, addr_out, func_in, addr_in, padrão de
     payload). Padrões: {const hex} e {var:count} p/ bytes que variam entre
     instâncias do mesmo template.
  5. Dedup por assinatura; conta ocorrências por sessão; guarda 1 exemplo.

Saída: docs/protocol_golden.json  (header, convenções, endereços, transações,
fluxos observados) — pronto para virar fixtures do gp100-core.
"""
import json, os, sys, glob
from collections import OrderedDict, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wirelog

HDR = "f021257f47502d64"
CAPTURES = sorted(glob.glob(os.path.join("analysis", "captures", "session*.jsonl")))
OUT = os.path.join("docs", "protocol_golden.json")
PAIR_TIMEOUT_MS = 3000
GAP_MS = 30000

# Contador de linhas cortadas (sem F7 final) por sessao. O golden precisa
# DIZER que as descarta — antes elas eram engolidas em silencio.
TRUNCADOS = [0]
CORTADOS_POR_SESSAO = {}

def trim(hx):
    """Valida o terminador SysEx (F7 final). `None` = linha cortada.

    Antes isto era `hx[:hx.find("f7")+2]` — o PRIMEIRO par de nibbles "f7" —
    que trunca frames completos cujo payload contem o par ("World" =
    `576f726c64` tem "f7" em `6f72"; um byte de DADO `f7` tambem). **16 frames
    completos** das 4 capturas eram atingidos, e o nome "Acoustic" virava 5
    bytes de 11.

    A segunda metade do bug e maior: as capturas tem **100 linhas sem F7
    final** (o proxy morreu no flush; todas param no prefixo de 242 bytes).
    Adivinhar o fim delas com `rfind` produz frames de 196 bytes que nunca
    existiram no fio — o golden passaria a descrever um aparelho que nao e
    este. Entao: sem F7 final, `None`. Cortado se conta e segue (R1).

    A regra mora em `analysis/wirelog.py` (fonte unica — #23); aqui delegamos.
    """
    return wirelog.trim(hx)

def repair(line):
    s = line.rstrip()
    if '"hex":"' in s and s.endswith("}") and not s.endswith('"}'):
        s = s[:-1] + '"'
    if s.endswith('}"') and '"hex":"' in s:
        try:
            json.loads(s)
        except json.JSONDecodeError:
            s = s[:-2] + '"}'
    return s

def load(path):
    evs = []
    for line in open(path, encoding="utf8", errors="replace"):
        line = repair(line.strip())
        if not line.startswith("{"):
            continue
        try:
            e = json.loads(line)
        except json.JSONDecodeError:
            continue
        hx = e.get("hex", "")
        if e.get("dir") in ("out_long", "in_long") and hx:
            # Linha cortada nao entra: entrar com ela inflaria as contagens
            # de todo template cujo payload ela "combina" por acidente.
            cut = trim(hx)
            if cut is None:
                TRUNCADOS[0] += 1
                continue
            e["hex"] = cut
            evs.append(e)
    return evs

def body(e):
    hx = e["hex"]
    if not hx.startswith(HDR) or len(hx) < 30 or len(hx) % 2:
        return None
    b = hx[16:-2]
    if len(b) % 2:
        b = b[:-1]
    try:
        return b[:2], b[2:10], bytes.fromhex(b[10:])
    except ValueError:
        return None

def segment(evs):
    """corta em fases por gap >GAP_MS"""
    if not evs:
        return []
    ts0 = evs[0]["ts"]
    segs, cur = [], [evs[0]]
    for e in evs[1:]:
        if e["ts"] - cur[-1]["ts"] > GAP_MS:
            segs.append(cur)
            cur = []
        cur.append(e)
    segs.append(cur)
    return segs

def pair_transactions(evs):
    """emparelha requests OUT -> respostas IN por (func,addr) com timeout;
    fallback de família 13xx: pushes IN com prefixo de 3B casam com o OUT
    13xx aberto mais recente (FIFO)."""
    rows = []
    for e in evs:
        b = body(e)
        if b:
            rows.append({"t": e["ts"], "dir": e["dir"], "func": b[0],
                         "addr": b[1], "data": b[2]})
    txs = []
    queues = defaultdict(list)          # (func,addr) -> [row IN pendente? nao]
    # Estratégia: fila de OUTs pendentes por (func,addr); cada IN com o mesmo
    # (func,addr) consome o OUT mais antigo da fila (dentro do timeout).
    pending = defaultdict(list)         # (func,addr) -> [row OUT]
    for r in rows:
        key = (r["func"], r["addr"])
        if r["dir"] == "out_long":
            pending[key].append(r)
        else:
            q = pending.get(key)
            if q and r["t"] - q[0]["t"] <= PAIR_TIMEOUT_MS:
                out = q.pop(0)
                txs.append({"out": out, "in": r, "match": "exact"})
            else:
                # fallback de família 13xx: IN 13 0X 00 YY casa com OUT
                # 13xx mais recente dentro do timeout
                if r["addr"][:2] == "13":
                    best = None
                    for k, q2 in pending.items():
                        if not q2 or k[0] != r["func"]:
                            continue
                        if k[1][:2] == "13" and r["t"] - q2[0]["t"] <= PAIR_TIMEOUT_MS:
                            if best is None or q2[0]["t"] > best["t"]:
                                best = q2[0]
                    if best is not None:
                        # remove da fila certa
                        q2 = pending[(best["func"], best["addr"])]
                        q2.remove(best)
                        txs.append({"out": best, "in": r, "match": "family13"})
                        continue
                txs.append({"out": None, "in": r, "match": "push"})
    # OUTs sem resposta = writes fire-and-forget
    for key, q in pending.items():
        for out in q:
            txs.append({"out": out, "in": None, "match": "write"})
    txs.sort(key=lambda x: (x["out"] or x["in"])["t"])
    return txs

def payload_pattern(datas):
    """padrão do payload a partir de N amostras: {const hex} / {var:count}
    const = byte igual em TODAS as instâncias; var = byte que varia.
    Comprimentos distintos => divide em sub-padrões por len (kind=by-len)."""
    if not datas:
        return {"kind": "empty", "len": 0}
    n = len(datas[0])
    if any(len(d) != n for d in datas):
        groups = defaultdict(list)
        for d in datas:
            groups[len(d)].append(d)
        if len(groups) > 1:
            by_len = {str(L): payload_pattern(ds) for L, ds in sorted(groups.items())}
            return {"kind": "by-len", "lens": sorted(int(x) for x in by_len),
                    "by_len": by_len}
    fixed, cur = [], None          # fixed = trechos CONSTantes
    for i in range(n):
        same = all(d[i] == datas[0][i] for d in datas)
        if same and cur is None:
            cur = i
        elif not same and cur is not None:
            fixed.append((cur, i))
            cur = None
    if cur is not None:
        fixed.append((cur, n))
    segs, pos = [], 0
    for a, b in fixed:
        if a > pos:
            segs.append({"kind": "var", "count": a - pos})
        segs.append({"kind": "const", "hex": datas[0][a:b].hex()})
        pos = b
    if pos < n:
        segs.append({"kind": "var", "count": n - pos})
    kinds = {s["kind"] for s in segs}
    kind = "const" if kinds == {"const"} else ("var" if kinds == {"var"} else "mixed")
    return {"kind": kind, "len": n, "segments": segs}

def main():
    sys.stdout.reconfigure(errors="replace")
    all_txs = []
    sess_stats = OrderedDict()
    for cap in CAPTURES:
        name = os.path.basename(cap)
        TRUNCADOS[0] = 0
        evs = load(cap)
        txs = []
        for seg in segment(evs):
            txs.extend(pair_transactions(seg))
        for t in txs:
            t["session"] = name.replace("session", "S").replace(".jsonl", "")
        cortados = TRUNCADOS[0]
        CORTADOS_POR_SESSAO[name] = cortados
        sess_stats[name] = {"events": len(evs), "transactions": len(txs),
                            "linhas_cortadas_descartadas": cortados}
        all_txs.extend(txs)
        sufixo = f"  ({cortados} linhas cortadas DESCARTADAS)" if cortados else ""
        print(f"{name}: {len(evs)} evs -> {len(txs)} transações{sufixo}")

    # ---- agrupa em templates ----
    templates = OrderedDict()
    for t in all_txs:
        out, inc = t.get("out"), t.get("in")
        if out is None:
            key = ("push", inc["func"], inc["addr"], None, None)
        elif inc is None:
            key = ("write", out["func"], out["addr"], None, None)
        else:
            key = ("req", out["func"], out["addr"], inc["func"], inc["addr"])
        templates.setdefault(key, []).append(t)

    out_tx = []
    for key, ts in templates.items():
        kind = key[0]
        entry = {"type": kind, "count": len(ts),
                 "sessions": sorted({t["session"] for t in ts})}
        if kind == "push":
            entry["in"] = {"func": key[1], "addr": key[2]}
            datas = [t["in"]["data"] for t in ts]
            entry["response_payload"] = payload_pattern(datas)
            entry["example"] = {"session": ts[0]["session"],
                                "hex": ts[0]["in"]["data"].hex()}
        elif kind == "write":
            entry["out"] = {"func": key[1], "addr": key[2]}
            datas = [t["out"]["data"] for t in ts]
            entry["request_payload"] = payload_pattern(datas)
            entry["example"] = {"session": ts[0]["session"],
                                "hex": ts[0]["out"]["data"].hex()}
        else:
            entry["out"] = {"func": key[1], "addr": key[2]}
            entry["in"] = {"func": key[3], "addr": key[4]}
            reqs = [t["out"]["data"] for t in ts if t["out"]["data"] is not None]
            rsps = [t["in"]["data"] for t in ts if t["in"]["data"] is not None]
            entry["request_payload"] = payload_pattern(reqs)
            entry["response_payload"] = payload_pattern(rsps)
            entry["example"] = {
                "session": ts[0]["session"],
                "request_hex": ts[0]["out"]["data"].hex() if ts[0]["out"]["data"] else "",
                "response_hex": ts[0]["in"]["data"].hex() if ts[0]["in"]["data"] else "",
            }
        entry["match"] = {m: sum(1 for t in ts if t["match"] == m)
                          for m in {t["match"] for t in ts}}
        out_tx.append(entry)

    ANNOT = {
        "13010000": "READ req: seleciona preset pp (payload=[pp]); abre download de estado (§13.10)",
        "13010001": "IN: meta 6B do preset pp selecionado (§13.10)",
        "13010002": "OUT: [pp] 01 = abre bloco de paginas / avanca pagina [pp][PG] 01 (§13.10)",
        "13010003": "IN: pagina PG do preset (196B/32B), d[3]=NUMERO DA PAGINA (§13.10)",
        "13010004": "OUT (f=11): READ de pagina; ou write 5B 01 xx 00 00 v (pp addrs internos, §13.4)",
        "13010005": "IN: pagina complementar do preset (§13.3)",
        "13020002": "OUT: proximo preset do scan (pp seguinte, §13.10)",
        "13000000": "IN: dump de boot do preset atual (§13.4)",
        "10xx0002": "OUT: SET de parametro: 20B nibble-exp [effectCode u32 LE][ctrl][00][f32 LE] (§13.11)",
        "10050001": "OUT: BEGIN/reserva de upload de IR: 8B cru `00 [slot] 00 00 01 00 00 0a`, "
                    "emitido ANTES do burst de chunks (não é commit; fim = último chunk duplicado) (§13.7)",
        "12001002": "READ/IN 75B nibble-exp: tabela de TIPOS de preset (20 paginas, §13.3)",
        "12001012": "READ/IN 44B nibble-exp: 5 entradas (setlist/loja) (§13.3)",
        "11000008": "READ/IN 14B: tabela de nomes por chave 2B; slots 0x0000-0x000F = fabrica (§13.12)",
        "11000000": "OUT write: metadados do preset atual: 8B zeros + nome ASCII [8..17] (§13.12)",
        "11000004": "OUT write: metadados 20B zeros (autor/notas?) (§13.12)",
        "11000005": "OUT write: `00 04 00 00` = ppType/icon u32 BE (§13.12)",
        "11000007": "OUT write: 50B zeros (reservado) (§13.12)",
        "12000002": "OUT write: 8B zeros (fecha bloco de metadados) (§13.12)",
        "12000001": "IN: status `01 00 00` = pronto (fecha boot e ciclos de op) (§13.12)",
        "12000000": "IN: status 2B no boot (§13.3)",
        "00020000": "OUT op: 8B, [4..5]=nº da op u16 BE: 1=entrar modo edicao, 2=etapa 2, 0=sair/commit (§13.12)",
        "00020001": "OUT: keepalive/ping `00 00 00 00` (§13.12)",
    }

    def annot(addr):
        if addr in ANNOT:
            return ANNOT[addr]
        if addr.startswith("10") and addr.endswith("0002"):
            return ANNOT["10xx0002"]
        if addr.startswith("13"):
            return "bloco de estado de preset 13xx (§13.10)"
        return None

    for e in out_tx:
        addr = (e.get("out") or e.get("in"))["addr"]
        e["semantic"] = annot(addr)

    doc = {
        "_meta": {
            "title": "GP-100 protocol golden-file (especificação executável)",
            "generated_by": "analysis/build_golden.py",
            "wire_header": "F0 21 25 7F 47 50 2D 64 | FUNC | ADDR(4B BE) | DATA | F7",
            "func": {"0x11": "READ request", "0x12": "data/write (respostas e writes)"},
            "pairing": f"OUT->IN por (func,addr), timeout {PAIR_TIMEOUT_MS}ms; "
                       "pushes 13xx casados por família (prefixo 3B); writes sem resposta = fire-and-forget",
            "payload_notation": {"const": "bytes fixos em todas as instâncias",
                                 "var": "bytes que variam (valor/campo)",
                                 "mixed": "const + var"},
            "sources": sess_stats,
            "excluded": {
                "linhas_cortadas": (
                    "linhas de log sem F7 final NAO sao frames: o proxy morreu "
                    "no flush. Descartadas em vez de reconstruidas por rfind, "
                    "que fabricaria payloads que nunca existiram no fio (R1). "
                    f"Total nas 4 capturas: {sum(CORTADOS_POR_SESSAO.values())}"
                ),
            },
            "reference": "docs/PROTOCOL.md §13.1–13.12",
        },
        "transactions": out_tx,
    }
    # newline="\n": sem isso o Python no Windows escreve CRLF e o golden versionado
    # passa a ter 2165 linhas com \r — diff gigante e ruido em toda revis��o.
    # O .gitattributes resolve no commit, mas o arquivo em disco ja chega
    # sujo para quem roda o script e abre o diff.
    with open(OUT, "w", encoding="utf8", newline="\n") as fh:
        json.dump(doc, fh, ensure_ascii=False, indent=2)
    print(f"\n{OUT}: {len(out_tx)} templates")
    for e in out_tx:
        if e["type"] == "req":
            print(f"  REQ  {e['out']['func']}/{e['out']['addr']} -> {e['in']['func']}/{e['in']['addr']}  x{e['count']}")
        elif e["type"] == "write":
            print(f"  WRT  {e['out']['func']}/{e['out']['addr']}  x{e['count']}")
        else:
            print(f"  PSH  {e['in']['func']}/{e['in']['addr']}  x{e['count']}")

if __name__ == "__main__":
    main()
