"""Spread and market-impact statistics from the live recorder (standard library only).

Reads data/live/bbo_um/*/*.jsonl[.gz] (top of book) and data/live/depth20_um/*/*.jsonl[.gz]
(20 levels per side) and prints, per symbol: the quoted spread in basis points of mid
(median, p90, p99, max) and the extra cost of a market order of a given notional beyond the
half-spread (walking the book), in basis points.

Usage: python scripts/spread-stats.py [data/live] [--notional 10000,50000,250000]
"""

import gzip
import json
import os
import sys


def lines(root, stream):
    base = os.path.join(root, stream)
    if not os.path.isdir(base):
        return
    for day in sorted(os.listdir(base)):
        d = os.path.join(base, day)
        for name in sorted(os.listdir(d)):
            p = os.path.join(d, name)
            opener = gzip.open if name.endswith(".gz") else open
            try:
                with opener(p, "rt", encoding="utf-8") as f:
                    for ln in f:
                        ln = ln.strip()
                        if ln:
                            try:
                                yield json.loads(ln)
                            except json.JSONDecodeError:
                                continue  # a partly written last line
            except (OSError, EOFError):
                continue  # file still being written


def pct(xs, q):
    if not xs:
        return float("nan")
    xs = sorted(xs)
    k = min(len(xs) - 1, max(0, int(round(q * (len(xs) - 1)))))
    return xs[k]


def walk(levels, notional):
    """Average fill price for buying (asks) `notional` quote units; None if the book is too thin."""
    left = notional
    qty = 0.0
    cost = 0.0
    for p, q in levels:
        p = float(p)
        q = float(q)
        take = min(q, left / p)
        qty += take
        cost += take * p
        left -= take * p
        if left <= 1e-9:
            return cost / qty
    return None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    root = args[0] if args else os.path.join("data", "live")
    notionals = [10_000.0, 50_000.0, 250_000.0]
    if "--notional" in sys.argv:
        i = sys.argv.index("--notional")
        notionals = [float(x) for x in sys.argv[i + 1].split(",")]

    spreads = {}
    first = {}
    last = {}
    for rec in lines(root, "bbo_um"):
        d = rec.get("d", {})
        s = d.get("s")
        try:
            b = float(d["b"])
            a = float(d["a"])
        except (KeyError, TypeError, ValueError):
            continue
        if b <= 0 or a < b:
            continue
        mid = 0.5 * (a + b)
        spreads.setdefault(s, []).append((a - b) / mid * 1e4)
        t = d.get("E", 0)
        first[s] = min(first.get(s, t), t)
        last[s] = max(last.get(s, t), t)

    impact = {}
    for rec in lines(root, "depth20_um"):
        d = rec.get("d", {})
        s = d.get("s") or rec.get("s")
        asks = d.get("a") or d.get("asks")
        bids = d.get("b") or d.get("bids")
        if not s or not asks or not bids:
            continue
        best_a = float(asks[0][0])
        best_b = float(bids[0][0])
        mid = 0.5 * (best_a + best_b)
        for n in notionals:
            buy = walk(asks, n)
            sell = walk([(p, q) for p, q in bids], n)
            if buy is None or sell is None:
                impact.setdefault((s, n), []).append(float("inf"))
                continue
            # beyond the half-spread: (avg fill - best quote) / mid, both sides averaged
            extra = 0.5 * ((buy - best_a) + (best_b - sell)) / mid * 1e4
            impact.setdefault((s, n), []).append(extra)

    print("symbol    samples   from(ms)        to(ms)          spread bps: median   p90     p99     max")
    for s in sorted(spreads):
        v = spreads[s]
        print(
            f"{s:<9} {len(v):>8}  {first[s]:<15} {last[s]:<15} "
            f"{pct(v, 0.5):>14.4f} {pct(v, 0.9):>7.4f} {pct(v, 0.99):>7.4f} {max(v):>7.4f}"
        )
    print()
    print("symbol    notional   snapshots  beyond 20 levels   impact beyond half-spread bps (fillable snapshots): median   p99     max")
    for (s, n) in sorted(impact):
        v = impact[(s, n)]
        fin = [x for x in v if x != float("inf")]
        thin = 100.0 * (len(v) - len(fin)) / max(1, len(v))
        mx = max(fin) if fin else float("nan")
        print(
            f"{s:<9} {n:>9.0f}  {len(v):>9}  {thin:>15.1f}%  {pct(fin, 0.5):>50.4f} {pct(fin, 0.99):>7.4f} {mx:>7.4f}"
        )


if __name__ == "__main__":
    main()
