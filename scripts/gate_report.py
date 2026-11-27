"""Build reports/gate-report.md, reports/win-rate-80.md and the equity/drawdown SVGs from the
Phase 4 research outputs (standard-library Python only).

Inputs (written by `candlescope research`): reports/research/{discovery,validation,holdout}.json,
reports/trial-ledger.csv, reports/research/equity/*.csv, reports/holdout-seal.json, and the two
hand-written parts reports/research/gate-head.md and reports/research/gate-warnings.md.

Usage: python scripts/gate_report.py   (from the project root)
"""

import csv
import json
import math
import os
import statistics as st
from datetime import datetime, timezone

R = "reports"
RD = os.path.join(R, "research")
CH = os.path.join(R, "charts")

DISC_START = int(datetime(2024, 11, 1, tzinfo=timezone.utc).timestamp() * 1000)
DISC_END = int(datetime(2025, 11, 1, tzinfo=timezone.utc).timestamp() * 1000)
VAL_END = int(datetime(2026, 5, 27, tzinfo=timezone.utc).timestamp() * 1000)
DATA_END = int(datetime(2026, 11, 26, tzinfo=timezone.utc).timestamp() * 1000)
LADDER = ["0.25", "0.5", "0.75", "1", "1.5", "2", "3"]


def load(name):
    p = os.path.join(RD, name)
    if not os.path.exists(p):
        return None
    with open(p, encoding="utf-8") as fh:
        return json.load(fh)


def text(name, default):
    p = os.path.join(RD, name)
    if os.path.exists(p):
        with open(p, encoding="utf-8") as fh:
            return fh.read().rstrip() + "\n"
    return default


def fl(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return float("nan")


def nf(x, d=3, sign=False):
    if x is None:
        return "-"
    x = fl(x)
    if math.isnan(x):
        return "-"
    if math.isinf(x):
        return "inf" if x > 0 else "-inf"
    return f"{x:+.{d}f}" if sign else f"{x:.{d}f}"


def pct(x, d=1):
    x = fl(x)
    return "-" if math.isnan(x) else f"{100 * x:.{d}f} %"


def sci(x):
    x = fl(x)
    return "-" if math.isnan(x) else f"{x:.2e}"


def table(head, rows):
    out = ["| " + " | ".join(head) + " |", "|" + "|".join("---" for _ in head) + "|"]
    for r in rows:
        out.append("| " + " | ".join(str(c) for c in r) + " |")
    return "\n".join(out) + "\n"


def short(r):
    s = f"{r['setup']} {r['direction']} {r['tf']} {r['rule']} {r['filter']}"
    if r.get("conf_min", "0") not in ("", "0"):
        s += f" conf>={r['conf_min']}"
    return s


# ------------------------------------------------------------------ SVG charts

CSS = """<style>
svg{--surface:#fcfcfb;--ink:#0b0b0b;--ink2:#52514e;--grid:#e3e2dc;--s1:#2a78d6;--s2:#eb6834;--band:#f1f0ec}
@media (prefers-color-scheme: dark){svg{--surface:#1a1a19;--ink:#f0efec;--ink2:#c3c2b7;--grid:#3a3a37;--s1:#3987e5;--s2:#d95926;--band:#252523}}
text{font-family:system-ui,-apple-system,'Segoe UI',sans-serif;fill:var(--ink2);font-size:11px}
.t{fill:var(--ink);font-size:13px;font-weight:600}.lab{fill:var(--ink);font-size:11px}
</style>"""


def esc(s):
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def read_equity(path):
    rows = []
    with open(path, encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            rows.append((int(r["exit_ms"]), fl(r["net_r_1x"]), fl(r["net_r_2x"]), r["period"]))
    rows.sort()
    return rows


def nice_ticks(lo, hi, n=5):
    if hi - lo < 1e-9:
        hi = lo + 1
    raw = (hi - lo) / n
    mag = 10 ** math.floor(math.log10(raw))
    step = min((m * mag for m in (1, 2, 2.5, 5, 10) if m * mag >= raw), default=mag * 10)
    a = math.floor(lo / step) * step
    ticks = []
    v = a
    while v <= hi + 1e-9:
        ticks.append(round(v, 10))
        v += step
    return ticks


def time_ticks(t0, t1):
    out = []
    y, m = 2024, 11
    while True:
        t = int(datetime(y, m, 1, tzinfo=timezone.utc).timestamp() * 1000)
        if t > t1:
            break
        if t >= t0 and m in (1, 4, 7, 10):
            out.append((t, f"{y}-{m:02d}"))
        m += 1
        if m == 13:
            y, m = y + 1, 1
    return out


def panel(x0, y0, w, h, t0, t1, series, ylabel, title=None, band=None, zero=True, fill=None):
    """series: list of (points [(t, y)], css var, label). Returns SVG fragment."""
    ys = [p[1] for s in series for p in s[0]] + ([0.0] if zero else [])
    lo, hi = (min(ys), max(ys)) if ys else (0.0, 1.0)
    if hi - lo < 1e-9:
        hi = lo + 1
    pad = 0.06 * (hi - lo)
    lo, hi = lo - pad, hi + pad
    X = lambda t: x0 + (t - t0) / (t1 - t0) * w
    Y = lambda v: y0 + h - (v - lo) / (hi - lo) * h
    g = []
    if band:
        a, b, lab = band
        g.append(f'<rect x="{X(a):.1f}" y="{y0}" width="{X(b) - X(a):.1f}" height="{h}" fill="var(--band)"/>')
        g.append(f'<text x="{X(a) + 4:.1f}" y="{y0 + 12}">{esc(lab)}</text>')
    for v in nice_ticks(lo, hi, 4):
        if lo <= v <= hi:
            g.append(f'<line x1="{x0}" x2="{x0 + w}" y1="{Y(v):.1f}" y2="{Y(v):.1f}" stroke="var(--grid)" stroke-width="1"/>')
            g.append(f'<text x="{x0 - 6}" y="{Y(v) + 4:.1f}" text-anchor="end">{v:g}</text>')
    if zero and lo < 0 < hi:
        g.append(f'<line x1="{x0}" x2="{x0 + w}" y1="{Y(0):.1f}" y2="{Y(0):.1f}" stroke="var(--ink2)" stroke-width="1"/>')
    g.append(f'<text x="{x0 - 40}" y="{y0 + h / 2:.1f}" transform="rotate(-90 {x0 - 40} {y0 + h / 2:.1f})" text-anchor="middle">{esc(ylabel)}</text>')
    if title:
        g.append(f'<text class="t" x="{x0}" y="{y0 - 8}">{esc(title)}</text>')
    labels = []
    for pts, var, lab in series:
        if not pts:
            continue
        d = " ".join(f"{'M' if i == 0 else 'L'}{X(t):.1f},{Y(v):.1f}" for i, (t, v) in enumerate(pts))
        if fill:
            base = Y(0) if lo < 0 < hi else Y(hi if hi <= 0 else lo)
            g.append(f'<path d="{d} L{X(pts[-1][0]):.1f},{base:.1f} L{X(pts[0][0]):.1f},{base:.1f} Z" fill="var({var})" fill-opacity="0.25" stroke="none"/>')
        g.append(f'<path d="{d}" fill="none" stroke="var({var})" stroke-width="2" stroke-linejoin="round"/>')
        if lab:
            tx, tv = pts[-1]
            labels.append([Y(tv) + 4, X(tx) + 6, lab])
    # End labels at least 13 px apart vertically (no collisions when the curves end close).
    labels.sort()
    for i in range(1, len(labels)):
        labels[i][0] = max(labels[i][0], labels[i - 1][0] + 13)
    for y, x, lab in labels:
        g.append(f'<text class="lab" x="{x:.1f}" y="{y:.1f}">{esc(lab)}</text>')
    return "\n".join(g), X


def equity_svg(path_out, title, rows, t0, t1, band_end):
    w, h = 760, 400
    c1, c2, d1 = [], [], []
    a = b = peak = 0.0
    for t, x1, x2, _ in rows:
        a += x1
        b += x2
        peak = max(peak, a)
        c1.append((t, a))
        c2.append((t, b))
        d1.append((t, a - peak))
    g = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="{esc(title)}">', CSS,
         f'<rect width="{w}" height="{h}" fill="var(--surface)"/>']
    band = (DISC_END, band_end, "validation (out of sample)")
    top, X = panel(70, 50, 560, 200, t0, t1,
                   [(c1, "--s1", f"1x {c1[-1][1]:+.1f} R" if c1 else ""),
                    (c2, "--s2", f"2x {c2[-1][1]:+.1f} R" if c2 else "")],
                   "cumulative net R", title, band)
    bot, _ = panel(70, 290, 560, 70, t0, t1, [(d1, "--s1", "")], "drawdown R", "Drawdown from peak, net R at 1x costs", band, fill=True)
    g += [top, bot]
    for t, lab in time_ticks(t0, t1):
        g.append(f'<text x="{X(t):.1f}" y="378" text-anchor="middle">{lab}</text>')
    # legend
    g.append('<rect x="70" y="386" width="14" height="3" fill="var(--s1)"/><text x="88" y="391">net R, 1x costs</text>')
    g.append('<rect x="200" y="386" width="14" height="3" fill="var(--s2)"/><text x="218" y="391">net R, 2x costs</text>')
    g.append("</svg>")
    with open(path_out, "w", encoding="utf-8") as fh:
        fh.write("\n".join(g))


def overview_svg(path_out, items, t0, t1):
    cols, pw, ph = 2, 330, 120
    rows_n = (len(items) + cols - 1) // cols
    w, h = 40 + cols * (pw + 80), 30 + rows_n * (ph + 60) + 20
    g = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="Cumulative net R of the exploratory rules">', CSS,
         f'<rect width="{w}" height="{h}" fill="var(--surface)"/>',
         '<text class="t" x="20" y="20">Cumulative net R at 1x costs, months 1-18 (shaded = validation). Exploratory rules only: none passed FDR.</text>']
    for k, (label, rows) in enumerate(items):
        cx = 70 + (k % cols) * (pw + 80)
        cy = 60 + (k // cols) * (ph + 60)
        a, pts = 0.0, []
        for t, x1, _, _ in rows:
            a += x1
            pts.append((t, a))
        frag, X = panel(cx, cy, pw, ph, t0, t1, [(pts, "--s1", f"{a:+.1f}")], "R", label, (DISC_END, VAL_END, ""))
        g.append(frag)
        for t, lab in time_ticks(t0, t1):
            g.append(f'<text x="{X(t):.1f}" y="{cy + ph + 14}" text-anchor="middle" style="font-size:9px">{lab}</text>')
    g.append("</svg>")
    with open(path_out, "w", encoding="utf-8") as fh:
        fh.write("\n".join(g))


# ------------------------------------------------------------------ report


def main():
    disc = load("discovery.json")
    val = load("validation.json")
    hold = load("holdout.json")
    with open(os.path.join(R, "holdout-seal.json"), encoding="utf-8") as fh:
        seal = json.load(fh)
    lock = os.path.exists(os.path.join(R, "holdout.lock"))
    with open(os.path.join(R, "trial-ledger.csv"), encoding="utf-8") as fh:
        ledger = list(csv.DictReader(fh))
    D = [r for r in ledger if r["stage"] == "discovery"]
    D30 = [r for r in D if int(r["trades"]) >= 30]
    os.makedirs(CH, exist_ok=True)
    WR = {}
    wp = os.path.join(RD, "winrates-discovery.csv")
    if os.path.exists(wp):
        with open(wp, encoding="utf-8") as fh:
            WR = {r["trial_id"]: r for r in csv.DictReader(fh)}

    entries = val["entries"] if val else []
    passed = set(val["passed"]) if val else set()
    survivors = [x for x in (hold["results"] if hold else []) if x["survives"]]

    md = []
    md.append(text("gate-head.md", "# Gate report\n\n(head missing)\n"))

    # ---- survivors
    md.append("## 1. Survivors\n")
    head = ["setup", "rule", "expectancy R, validation (80 % interval)", "win rate", "trades (val)", "max drawdown R", "holdout result"]
    if survivors:
        rows = []
        for s in survivors:
            e = next(x for x in entries if x["trial"]["trial_id"] == s["trial_id"])
            v = e["validation"]
            ci = e["validation_ci80"]
            h = s["holdout"]
            rows.append([e["trial"]["setup"], e["trial"]["label"], f"{nf(v['exp_net_1x'], 3, True)} ({nf(ci[0], 3, True)} .. {nf(ci[1], 3, True)})",
                         pct(v["win_rate"]), v["trades"], nf(v["max_dd_r"], 1),
                         f"{h['trades']} trades, {nf(h['exp_net_1x'], 3, True)} R at 1x, {nf(h['exp_net_2x'], 3, True)} R at 2x"])
        md.append(table(head, rows))
    else:
        md.append("**None.** No setup passed the discovery FDR test (S1), so no rule could be frozen as a finalist and the table is empty:\n\n")
        md.append(table(head, [["(none)", "-", "-", "-", "-", "-", "holdout not opened (still sealed)"]]))
    md.append("\n")

    # ---- rejected setups
    md.append("## 2. Rejected setups and the reason\n")
    md.append(f"All 59 setups are rejected. The reason is the same for every one: **S1 failed**, no discovery trial passed Benjamini-Hochberg FDR control at q = 0.10 over m = {disc['m']:,} trials (needed p <= {0.10 / disc['m']:.1e} for the best trial; the lowest p of all trials was {disc['min_p']:.2e}, q = 1.00 for every trial). The columns show how far each setup was from an edge: its trials, the share of its trials (with at least 30 trades) that were positive after 1x costs, its median gross and cost per trade, and its best discovery trial by t (which, for the 10 exploratory rules, was also run on validation).\n\n")
    order = []
    for r in D:
        if r["setup"] not in order:
            order.append(r["setup"])
    expl = {e["trial"]["label"]: e for e in entries}
    rows = []
    for s in order:
        T = [r for r in D if r["setup"] == s]
        T30 = [r for r in T if int(r["trades"]) >= 30]
        pos = sum(fl(r["exp_net_1x"]) > 0 for r in T30)
        if T30:
            best = max(T30, key=lambda r: fl(r["t_stat"]) if not math.isnan(fl(r["t_stat"])) else -1e9)
            lab = short(best)
            b = f"{lab}: {best['trades']} trades, {nf(best['exp_net_1x'], 3, True)} R, p {sci(best['p_value'])}"
            mg = st.median(fl(r["exp_gross"]) for r in T30)
            mc = st.median(fl(r["cost_1x"]) for r in T30)
        else:
            lab, b, mg, mc = "", "no trial with 30 trades", float("nan"), float("nan")
        reason = "S1: no trial passed FDR"
        for e in entries:
            if e["trial"]["setup"] == s:
                v = e["validation"]
                reason += f"; exploratory rule {e['trial']['label'].split(' ', 1)[1]} made {nf(v['exp_net_1x'], 3, True)} R on {v['trades']} validation trades"
        if not T30:
            reason += " (too rare: fewer than 30 trades in every trial)"
        rows.append([s, len(T), len(T30), pct(pos / len(T30)) if T30 else "-", nf(mg, 3, True), nf(mc, 3), b, reason])
    md.append(table(["setup", "trials", "with >= 30 trades", "positive at 1x", "median gross R", "median cost R", "best discovery trial (by t)", "reason"], rows))
    md.append("\n")

    # ---- equity curves
    md.append("## 3. Equity curves and drawdowns (in R)\n")
    md.append("There are no survivors or finalists, so these charts show the 10 **exploratory** rules (the best discovery t among rules with at least 100 trades and positive net expectancy; they failed FDR and could never survive). Months 1-12 are the discovery data they were picked on; the shaded months 13-18 are validation. The holdout was not opened, so it is not drawn.\n\n")
    items = []
    for e in entries:
        p = os.path.join(RD, "equity", f"{e['role']}-{e['trial']['trial_id']}.csv")
        if not os.path.exists(p):
            continue
        rows_e = read_equity(p)
        if not rows_e:
            continue
        lab = e["trial"]["label"]
        fn = f"equity-{e['role']}-{e['trial']['trial_id']}.svg"
        equity_svg(os.path.join(CH, fn), f"{lab}: cumulative net R", rows_e, DISC_START, VAL_END, VAL_END)
        items.append((lab, rows_e))
    if items:
        overview_svg(os.path.join(CH, "equity-overview.svg"), items, DISC_START, VAL_END)
        md.append("![Cumulative net R of the exploratory rules](charts/equity-overview.svg)\n\n")
        lines = ["Each rule with its 1x and 2x curve and its drawdown chart:\n"]
        for e in entries:
            fn = f"equity-{e['role']}-{e['trial']['trial_id']}.svg"
            if os.path.exists(os.path.join(CH, fn)):
                v, d = e["validation"], e["discovery_rerun"]
                lines.append(f"- [{e['trial']['label']}](charts/{fn}): discovery {nf(d['sum_r_1x'], 1, True)} R over {d['trades']} trades, validation {nf(v['sum_r_1x'], 1, True)} R over {v['trades']} trades; max drawdown months 1-18 {nf(e['months_1_18']['max_dd_r'], 1)} R")
        md.append("\n".join(lines) + "\n\n")
        drows = []
        for e in entries:
            m = e["months_1_18"]
            drows.append([e["trial"]["label"], nf(e["discovery_rerun"]["max_dd_r"], 1), nf(e["validation"]["max_dd_r"], 1), nf(m["max_dd_r"], 1), m["longest_losing_streak"], nf(m["worst_trade_r"], 2)])
        md.append("Drawdown table (net R at 1x costs):\n\n")
        md.append(table(["rule", "max DD discovery", "max DD validation", "max DD months 1-18", "longest losing streak", "largest single loss"], drows))
        md.append("\n")

    # ---- warnings
    md.append(text("gate-warnings.md", "## 4. Warnings\n\n(missing)\n"))
    md.append("\n")

    # ---- appendix A
    md.append("## Appendix A. What was run\n\n")
    fam = disc["families"]
    md.append(table(["item", "value"], [
        ["data", "Binance USDS-M BTCUSDT (search) and ETHUSDT (check), 1m candles 2024-11-01 .. 2026-11-25, strict 5m/15m/1h, historical funding"],
        ["discovery", "2024-11-01 .. 2025-11-01 (12 months)"],
        ["validation", "2025-11-01 .. 2026-05-27 (207 days)"],
        ["holdout", f"{seal['holdout_utc'][0]} .. {seal['holdout_utc'][1]} (183 days), sealed {seal['created_utc']} UTC before any result; " + ("**opened once** (`reports/holdout.lock` present)" if lock else "**not opened**: no finalist passed validation; no lock file; still sealed for a later phase")],
        ["trials (FDR pool m)", f"{disc['m']:,} = main {fam['main']:,} + healthy {fam['healthy']} + frontier {fam['frontier']:,} + confluence {fam['confluence']}"],
        ["trials with any trade / with >= 30 trades", f"{disc['trials_with_trades']:,} / {disc['trials_ge30']:,}"],
        ["costs", "taker 5 bps per fill, spread 1 bp, slippage 1 bp + 0.02 ATR per market fill, historical funding; 2x doubles fee, spread and slippage"],
        ["engine", "cs-backtest unchanged (Phase 3); prefix lookahead audit passed on every market built (BTC discovery and months 1-18, ETH months 1-18; 5m/15m/1h; detector and random-entry sources)"],
        ["runtime", f"discovery {disc['seconds']:.1f} s, validation {val['seconds']:.1f} s (release build, 6 threads)" if val else "-"],
        ["ledger", f"`reports/trial-ledger.csv`, {len(ledger):,} rows ({len(D):,} discovery trials + baselines + validation-stage evaluations); discovery part sha256 `{disc['ledger_sha256'][:16]}...`"],
    ]))
    md.append("\n")

    # ---- appendix B
    md.append("## Appendix B. Discovery results\n\n")
    md.append(f"Benjamini-Hochberg at q = 0.10: **{disc['bh_rejections']} rejections**. Lowest p-value {disc['min_p']:.2e}; the best trial would have needed p <= {0.1 / disc['m']:.1e}. Trials with p < 0.05: {disc['trials_p_lt_0_05']} of {disc['trials_ge30']:,} testable; with p < 0.01: {disc['trials_p_lt_0_01']}. Plateau (S2) held for {disc['plateau_pass']} trials, none of which passed S1.\n\n")
    md.append("Distribution over all trials with at least 30 trades. `z gross` = gross mean / (sd / sqrt(n)): if the setups had no pre-cost information at all, about 5 % of trials would show |z| > 1.96 (2.5 % on each side).\n\n")
    rows = []
    for tf in ["5m", "15m", "1h", "all"]:
        T = [r for r in D30 if tf == "all" or r["tf"] == tf]
        z = [fl(r["exp_gross"]) / (fl(r["sd_1x"]) / math.sqrt(int(r["trades"]))) for r in T if fl(r["sd_1x"]) > 0]
        rows.append([tf, f"{len(T):,}", nf(st.median(fl(r['exp_net_1x']) for r in T), 3, True), pct(sum(fl(r['exp_net_1x']) > 0 for r in T) / len(T)),
                     pct(sum(fl(r['exp_net_2x']) > 0 for r in T) / len(T)), nf(st.median(fl(r['exp_gross']) for r in T), 3, True),
                     pct(sum(fl(r['exp_gross']) > 0 for r in T) / len(T)), nf(st.median(fl(r['cost_1x']) for r in T), 3),
                     pct(sum(x > 1.96 for x in z) / len(z)), pct(sum(x < -1.96 for x in z) / len(z))])
    md.append(table(["tf", "trials", "median net 1x R", "net 1x > 0", "net 2x > 0", "median gross R", "gross > 0", "median cost R/trade", "z gross > 1.96", "z gross < -1.96"], rows))
    md.append("\nTop 20 discovery trials by t (net R at 1x; none passes FDR):\n\n")
    top = sorted(D30, key=lambda r: -fl(r["t_stat"]))[:20]
    base = {(b["tf"], b["period"]): b["stats"] for b in disc["baselines"]}
    rows = [[short(r), r["trades"], pct(fl(r["win_rate"])), nf(r["exp_net_1x"], 3, True), nf(r["exp_net_2x"], 3, True), nf(r["exp_gross"], 3, True), nf(r["cost_1x"], 3),
             nf(fl(r["exp_net_1x"]) - base[(r["tf"], "discovery")]["exp_net_1x"], 3, True), nf(r["t_stat"], 2), sci(r["p_value"]), sci(r["q_value"]), "yes" if r["plateau"] == "1" else "no"] for r in top]
    md.append(table(["trial", "trades", "win rate", "net 1x", "net 2x", "gross", "cost/trade", "vs random 1x", "t", "p", "q", "plateau"], rows))
    md.append("\n")

    # ---- appendix C
    md.append("## Appendix C. Random-entry baseline (control)\n\nRandom direction at random candles (p = 0.5, seed 1), stop 1 ATR, target 1 R, same costs. Net = minus the cost, gross = zero within noise, as it should be.\n\n")
    bl = (val["baselines"] if val else []) + (hold["baselines"] if hold else [])
    rows = [[b["symbol"], b["tf"], b["period"], b["stats"]["trades"], nf(b["stats"]["exp_net_1x"], 4, True), nf(b["stats"]["exp_net_2x"], 4, True), nf(b["stats"]["exp_gross"], 4, True), nf(b["stats"]["cost_1x"], 4)] for b in bl]
    md.append(table(["symbol", "tf", "period", "trades", "net 1x R", "net 2x R", "gross R", "cost R/trade"], rows))
    md.append("\n")

    # ---- appendix D
    md.append("## Appendix D. Validation of the exploratory rules (information only, cannot survive)\n\n")
    md.append("Frozen before validation in `reports/research/finalists.json`. Pre-registered criteria: S3 >= 100 validation trades, S4 net at 2x > 0, S5 80 % interval lower bound > 0, S6 ETH months 1-18 not contradicted. S1/S2 are failed by construction for exploratory rules.\n\n")
    rows = []
    for e in entries:
        d, v, c = e["discovery_rerun"], e["validation"], e["criteria"]
        fails = [k.split("_")[0] for k, ok in c.items() if not ok]
        rows.append([e["trial"]["label"], f"{d['trades']} / {nf(d['exp_net_1x'], 3, True)}", v["trades"], pct(v["win_rate"]), nf(v["exp_net_1x"], 3, True),
                     f"{nf(e['validation_ci80'][0], 3, True)} .. {nf(e['validation_ci80'][1], 3, True)}", nf(v["exp_net_2x"], 3, True), nf(v["exp_gross"], 3, True), nf(v["cost_1x"], 3),
                     nf(v["max_dd_r"], 1), nf(e["matched_random"]["validation"]["exp_net_1x"], 3, True), nf(e["validation_minus_matched_random_1x"], 3, True),
                     f"{e['eth']['status']} ({e['eth']['months_1_18']['trades']}, {nf(e['eth']['months_1_18']['exp_net_1x'], 3, True)})", ", ".join(fails)])
    md.append(table(["rule", "discovery trades / net 1x", "val trades", "val win rate", "val net 1x", "80 % interval", "val net 2x", "val gross", "cost/trade", "val max DD", "matched random 1x", "vs matched random", "ETH months 1-18", "failed"], rows))
    md.append(f"\nOn average the 10 rules made {nf(st.mean(e['discovery_rerun']['exp_net_1x'] for e in entries), 3, True)} R per trade in discovery and {nf(st.mean(e['validation']['exp_net_1x'] for e in entries), 3, True)} R in validation: the discovery edge was selection noise.\n\n" if entries else "\n")

    # ---- appendix E
    md.append("## Appendix E. Walk-forward (3 months train, 1 month test, months 1-18)\n\nEach month the variant with the best train-window net expectancy (>= 30 train trades) among the rule and its plateau neighbours is traded in the next month. Test months 2025-02..2025-10 lie inside the discovery data the rule was picked from (not independent); 2025-11..2026-05 are validation months.\n\n")
    rows = []
    for e in entries:
        w = e["wfo"]
        rows.append([e["trial"]["label"], len(w["variants"]), w["oos_trades"], nf(w["oos_exp_1x"], 3, True), nf(w["oos_exp_2x"], 3, True),
                     f"{w['oos_discovery_months_trades']} / {nf(w['oos_discovery_months_exp_1x'], 3, True)}", f"{w['oos_validation_months_trades']} / {nf(w['oos_validation_months_exp_1x'], 3, True)}",
                     f"{w['test_months_positive']} of {w['test_months_with_trades']}"])
    md.append(table(["rule", "variants", "OOS trades", "OOS net 1x", "OOS net 2x", "discovery months: trades / net", "validation months: trades / net", "positive test months"], rows))
    md.append("\n")

    # ---- appendix F
    md.append("## Appendix F. Regimes, volatility, sessions and UTC hours (validation trades, net R at 1x)\n\n")
    md.append("`top-3 hours share` = net R of the best three signal hours divided by the total net R (only defined when the total is positive; above 1 means the other 21 hours lost money in sum).\n\n")
    rows = []
    for e in entries:
        rg = e["regimes"]["validation"]
        cell = lambda key: "; ".join(f"{b['key']} {b['trades']}/{nf(b['exp_net_1x'], 2, True)}" for b in rg[key])
        rows.append([e["trial"]["label"], cell("regime"), cell("volatility"), cell("session"), nf(rg["top3_hour_share"], 2)])
    md.append(table(["rule", "ADX regime (trades/net)", "volatility tercile", "session", "top-3 hours share"], rows))
    md.append("\n")

    # ---- appendix G
    md.append("## Appendix G. Confluence (pre-registered, limited)\n\n")
    C = [r for r in D if r["family"] == "confluence"]
    C30 = [r for r in C if int(r["trades"]) >= 30]
    md.append(f"Confluence family: {len(C)} trials (count >= 1 / 2 / 3, rule b025-t2), {len(C30)} with >= 30 trades; positive at 1x: {sum(fl(r['exp_net_1x']) > 0 for r in C30)}; lowest p {sci(min((fl(r['p_value']) for r in C), default=float('nan')))}.\n\n")
    rows = []
    for k in ["1", "2", "3"]:
        T = [r for r in C30 if r["conf_min"] == k]
        if T:
            rows.append([f">= {k}", len(T), nf(st.median(fl(r['exp_net_1x']) for r in T), 3, True), nf(st.median(fl(r['exp_gross']) for r in T), 3, True), pct(sum(fl(r['exp_net_1x']) > 0 for r in T) / len(T)), nf(st.median(int(r['trades']) for r in T), 0)])
    md.append(table(["threshold", "trials (>= 30 trades)", "median net 1x", "median gross", "positive at 1x", "median trades"], rows))
    if val and val.get("confluence_buckets"):
        md.append("\nDescriptive (not trials): unfiltered b025-t2 trades of each confluence setup split by the exact confluence count of the signal, discovery period, pooled over both directions, per time frame (trades / net R at 1x / net win rate):\n\n")
        agg = {}
        for b in val["confluence_buckets"]:
            if b["period"] != "discovery":
                continue
            for x in b["buckets"]:
                a = agg.setdefault(((b["tf"], b["setup"]), x["count"]), [0, 0.0, 0.0])
                if x["trades"]:
                    a[0] += x["trades"]
                    a[1] += x["trades"] * x["exp_net_1x"]
                    a[2] += x["trades"] * x["win_rate"]
        rows = []
        tfo = {"5m": 0, "15m": 1, "1h": 2}
        for key in sorted({k[0] for k in agg}, key=lambda k: (tfo[k[0]], k[1])):
            cells = []
            for c in range(5):
                n, sr, sw = agg.get((key, c), [0, 0.0, 0.0])
                cells.append(f"{n} / {nf(sr / n, 2, True)} / {pct(sw / n, 0)}" if n else "0")
            rows.append([key[0], key[1]] + cells)
        md.append(table(["tf", "setup", "count 0", "count 1", "count 2", "count 3", "count 4"], rows))
        md.append("\nSee the gate report's confluence note in the warnings for how to read this table.\n")
    md.append("\n")

    # ---- appendix H: win-rate frontier
    md.append("## Appendix H. Win-rate frontier\n\nWin rate against net expectancy at 1x costs across the target ladder, no filter, discovery data. Averages over all setups, both directions and all time frames (trials with >= 30 trades):\n\n")
    rows = []
    for b in ["b025", "b100"]:
        for t in LADDER:
            T = [r for r in D30 if r["filter"] == "F00" and r["rule"] == f"{b}-t{t}" and r["family"] in ("main", "frontier")]
            if T:
                hits = [fl(WR[r["trial_id"]]["target_hit_rate"]) for r in T if r["trial_id"] in WR]
                rows.append([b.replace("b025", "0.25 ATR").replace("b100", "1.0 ATR"), f"{t} R", len(T), pct(st.mean(hits)) if hits else "-", pct(st.mean(fl(r['win_rate']) for r in T)), nf(st.mean(fl(r['exp_net_1x']) for r in T), 3, True),
                             nf(st.mean(fl(r['exp_gross']) for r in T), 3, True), nf(st.mean(fl(r['cost_1x']) for r in T), 3), pct(sum(fl(r['exp_net_1x']) > 0 for r in T) / len(T)), nf(st.mean(fl(r['worst_trade_r']) for r in T), 2)])
    md.append(table(["stop buffer", "target", "trials", "mean target-hit rate", "mean net win rate", "mean net 1x", "mean gross", "mean cost", "positive", "mean largest loss"], rows))
    md.append("\nTwo win rates: the **target-hit rate** (what a chart reader counts as a win) and the **net win rate** (net R > 0 after costs, the protocol's definition). A small target raises the hit rate, but fees are paid on every trade, so many hits are net losses and expectancy does not improve; each full stop-out costs 1 R plus fees. With the 0.25-ATR buffer the structural stop is often only a few basis points away, so fees alone are several R on those trades (hence the large 'largest loss').\n")
    md.append("\nPer setup (native direction, 15m, no filter): target-hit rate / net win rate / net R at 1x for each target. Same table for every direction and time frame: `reports/trial-ledger.csv`.\n\n")
    for b in ["b025", "b100"]:
        md.append(f"Stop buffer {'0.25' if b == 'b025' else '1.0'} ATR:\n\n")
        rows = []
        for s in order:
            cells = []
            for t in LADDER:
                r = next((x for x in D if x["setup"] == s and x["direction"] == "native" and x["tf"] == "15m" and x["filter"] == "F00" and x["rule"] == f"{b}-t{t}" and x["family"] in ("main", "frontier")), None)
                if r is None or int(r["trades"]) == 0:
                    cells.append("-")
                else:
                    h = WR.get(r["trial_id"], {}).get("target_hit_rate")
                    cells.append(f"{pct(fl(h), 0) if h else '-'} / {pct(fl(r['win_rate']), 0)} / {nf(r['exp_net_1x'], 2, True)}" + ("*" if int(r["trades"]) < 30 else ""))
            rows.append([s] + cells)
        md.append(table(["setup"] + [f"{t} R" for t in LADDER], rows))
        md.append("\n`*` fewer than 30 trades.\n\n")
    W = sorted([r for r in D if int(r["trades"]) >= 30 and fl(r["win_rate"]) >= 0.8], key=lambda r: -fl(r["exp_net_1x"]))
    wv = {w["label"]: w for w in (val["wr80"] if val else [])}
    md.append(f"**Every variant with win rate >= 80 % and at least 30 trades ({len(W)}):** ({disc['wr80_small_sample_count']} more reach 80 % with fewer than 30 trades; all are listed in `reports/win-rate-80.md`.)\n\n")
    rows = []
    for r in W:
        lab = short(r)
        v = wv.get(lab, {}).get("validation")
        rows.append([lab, r["trades"], pct(fl(r["win_rate"])), nf(r["exp_net_1x"], 3, True), nf(r["exp_net_2x"], 3, True), r["longest_losing_streak"], nf(r["worst_trade_r"], 2),
                     f"{v['trades']} trades, {pct(v['win_rate'])}, {nf(v['exp_net_1x'], 3, True)} R" if v else "-"])
    md.append(table(["variant", "trades", "win rate", "net 1x R", "net 2x R", "worst losing streak", "largest single loss R", "validation (frozen)"], rows))
    H = sorted([w for w in WR.values() if int(w["trades"]) >= 30 and fl(w["target_hit_rate"]) >= 0.8], key=lambda w: -fl(w["exp_net_1x"]))
    if H:
        Lk = {r["trial_id"]: r for r in D}
        md.append(f"\n**Variants whose target is hit in >= 80 % of trades (at least 30 trades): {len(H)}.** Positive after 1x costs: {sum(fl(w['exp_net_1x']) > 0 for w in H)}; after 2x costs: {sum(fl(Lk[w['trial_id']]['exp_net_2x']) > 0 for w in H)}. Median net expectancy {nf(st.median(fl(w['exp_net_1x']) for w in H), 3, True)} R, median net win rate {pct(st.median(fl(w['net_win_rate']) for w in H))}, median largest single loss {nf(st.median(fl(w['worst_trade_r']) for w in H), 2)} R. The 15 best by expectancy (all {len(H)} in `reports/win-rate-80.md`):\n\n")
        rows = [[w["label"], w["trades"], pct(fl(w["target_hit_rate"])), pct(fl(w["net_win_rate"])), nf(w["exp_net_1x"], 3, True), nf(Lk[w["trial_id"]]["exp_net_2x"], 3, True), w["longest_losing_streak"], nf(w["worst_trade_r"], 2)] for w in H[:15]]
        md.append(table(["variant", "trades", "target-hit rate", "net win rate", "net 1x R", "net 2x R", "worst losing streak", "largest single loss R"], rows))
    md.append("\n")

    # ---- appendix I
    md.append("## Appendix I. Files\n\n")
    md.append(table(["file", "content"], [
        ["`reports/protocol.md`", "pre-registered protocol (written before any run)"],
        ["`reports/search-space.md`", "pre-registered search space, m = 34,584"],
        ["`reports/holdout-seal.json`", "seal: boundary, holdout data hashes, protocol hashes, time"],
        ["`reports/trial-ledger.csv`", "every trial and every later evaluation (one row each)"],
        ["`reports/win-rate-80.md`", "every variant with net win rate >= 80 %, and every variant whose target is hit in >= 80 % of trades"],
        ["`reports/research/winrates-discovery.csv`", "target-hit, gross and net win rates of the fixed-target discovery trials (descriptive)"],
        ["`reports/research/discovery.json`, `finalists.json`, `validation.json`", "stage outputs (frozen lists, all numbers above)"],
        ["`reports/charts/*.svg`", "equity and drawdown charts"],
        ["`scripts/gate_report.py`", "builds this report from the files above"],
    ]))

    with open(os.path.join(R, "gate-report.md"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(md).replace("\n\n\n", "\n\n"))

    # ---- win-rate-80.md
    allw = sorted([r for r in D if int(r["trades"]) > 0 and fl(r["win_rate"]) >= 0.8], key=lambda r: (-int(r["trades"]), -fl(r["exp_net_1x"])))
    out = ["# Variants with win rate >= 80 % (discovery, BTCUSDT, net of 1x costs)\n", "## Net win rate >= 80 % (protocol definition: net R > 0 after costs)\n",
           f"{len(allw)} variants reach a win rate of at least 80 %; {len(W)} of them have at least 30 trades. A win is a trade with net R > 0 after costs. Sorted by number of trades. Validation results (frozen) exist for the variants with at least 30 trades.\n"]
    rows = []
    for r in allw:
        lab = short(r)
        v = wv.get(lab, {}).get("validation")
        rows.append([lab, r["family"], r["trades"], pct(fl(r["win_rate"])), nf(r["exp_net_1x"], 3, True), nf(r["exp_net_2x"], 3, True), r["longest_losing_streak"], nf(r["worst_trade_r"], 2),
                     f"{v['trades']} / {pct(v['win_rate'])} / {nf(v['exp_net_1x'], 3, True)}" if v else "-"])
    out.append(table(["variant", "family", "trades", "win rate", "net 1x R", "net 2x R", "worst losing streak", "largest single loss R", "validation trades / win rate / net"], rows))
    if WR:
        Lk = {r["trial_id"]: r for r in D}
        HH = sorted([w for w in WR.values() if int(w["trades"]) > 0 and fl(w["target_hit_rate"]) >= 0.8], key=lambda w: (-int(w["trades"]), -fl(w["exp_net_1x"])))
        out.append(f"\n## Target hit in >= 80 % of trades\n\n{len(HH)} fixed-target variants hit their target in at least 80 % of trades ({sum(int(w['trades']) >= 30 for w in HH)} with at least 30 trades). The target-hit rate is what a chart reader counts as a win; the net win rate counts only trades that are positive after costs. Computed by `candlescope research --stage winrates` (descriptive, added after the results were seen; same trials, no selection).\n")
        rows = [[w["label"], w["trades"], pct(fl(w["target_hit_rate"])), pct(fl(w["net_win_rate"])), nf(w["exp_net_1x"], 3, True), nf(Lk[w["trial_id"]]["exp_net_2x"], 3, True), w["longest_losing_streak"], nf(w["worst_trade_r"], 2)] for w in HH]
        out.append(table(["variant", "trades", "target-hit rate", "net win rate", "net 1x R", "net 2x R", "worst losing streak", "largest single loss R"], rows))
    with open(os.path.join(R, "win-rate-80.md"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(out))
    print(f"wrote reports/gate-report.md, reports/win-rate-80.md ({len(allw)} variants), {len(items) + (1 if items else 0)} charts")


if __name__ == "__main__":
    main()
