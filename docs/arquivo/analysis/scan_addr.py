#!/usr/bin/env python3
"""scan_addr.py — ocorrências de endereços 11xx/12xx/0002xxxx em todas as sessões."""
import json, os, sys, glob
from collections import Counter, defaultdict

HDR = "f021257f47502d64"
TARGETS_PREFIX = ("1100", "1200", "0002")

def trim(hx):
    i = hx.find("f7")
    return hx[: i + 2] if i >= 0 else hx

def load(path):
    evs = []
    for line in open(path, encoding="utf8", errors="replace"):
        s = line.strip()
        if '"hex":"' in s and s.endswith("}") and not s.endswith('"}'):
            s = s[:-1] + '"'
        if s.endswith('}"') and '"hex":"' in s:
            try:
                json.loads(s)
            except json.JSONDecodeError:
                s = s[:-2] + '"}'
        if not s.startswith("{"):
            continue
        try:
            e = json.loads(s)
        except json.JSONDecodeError:
            continue
        hx = e.get("hex", "")
        if e.get("dir") in ("out_long", "in_long") and hx:
            e["hex"] = trim(hx)
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

def main():
    sys.stdout.reconfigure(errors="replace")
    for cap in sorted(glob.glob(os.path.join("analysis", "captures", "session*.jsonl"))):
        evs = load(cap)
        ts0 = evs[0]["ts"]
        hits = []
        for e in evs:
            b = body(e)
            if not b:
                continue
            t, dr, f, a, d = e["ts"] - ts0, e["dir"], b[0], b[1], b[2]
            if a[:4] in ("1100", "1200") or a[:6] == "000200":
                hits.append((t, dr, f, a, d))
        print(f"\n===== {os.path.basename(cap)}: {len(evs)} evs, {len(hits)} hits em 11xx/12xx/0002xx =====")
        # agrupa por addr
        cnt = Counter((h[2], h[3], h[1][:3], len(h[4]) if h[4] else 0) for h in hits)
        for k, c in sorted(cnt.items()):
            print(f"  f={k[0]} a={k[1]} {k[2]} len={k[3]}  x{c}")
        # janelas de tempo distintas (cluster >60s de gap = evento novo)
        if hits:
            clusters, cur = [], [hits[0]]
            for h in hits[1:]:
                if h[0] - cur[-1][0] > 60000:
                    clusters.append(cur)
                    cur = []
                cur.append(h)
            clusters.append(cur)
            print(f"  clusters temporais: {len(clusters)}")
            for cl in clusters:
                t0, t1 = cl[0][0] / 1000, cl[-1][0] / 1000
                kinds = Counter(f"{h[2]}/{h[3]}/{h[1][:3]}" for h in cl)
                tops = "  ".join(f"{k}:{v}" for k, v in kinds.most_common(8))
                print(f"    {t0:9.1f}s..{t1:9.1f}s  ({len(cl)} evs)  {tops}")

if __name__ == "__main__":
    main()
