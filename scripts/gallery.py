"""Detector gallery: draws the random events picked by `candlescope detect` (gallery.json) as
SVG charts, one sheet of up to 10 panels per detector, plus an index.html. Standard library only.

Usage: python scripts/gallery.py data/events/um/BTCUSDT/15m/gallery.json reports/detector-gallery

Legend per panel: candles (green up, red down); orange dashed line = formed_at (pattern
completes); blue solid line = known_at (first candle at whose close the event is known; the
event is usable from the next candle); shaded band = lo..hi (zone / pattern extent); grey dashed
line = price; purple line = anchors A-B (legs, trendlines, structure). Candles to the right of
the blue line are shown only for checking by eye; detectors never saw them.
"""

import html
import json
import os
import sys

PW, PH = 620, 270  # panel size
COLS = 2
PAD_L, PAD_R, PAD_T, PAD_B = 8, 62, 34, 14


def esc(s):
    return html.escape(str(s), quote=True)


def panel(ev, ox, oy):
    cs = ev["candles"]
    n = len(cs)
    w0 = ev["w0"]
    vals = [x for c in cs for x in (c[1], c[2])]
    for k in ("price", "lo", "hi", "ay", "by"):
        v = ev.get(k)
        if v is not None:
            vals.append(v)
    lo, hi = min(vals), max(vals)
    if hi <= lo:
        hi = lo + 1.0
    span = hi - lo
    lo -= span * 0.04
    hi += span * 0.04
    pw = PW - PAD_L - PAD_R
    ph = PH - PAD_T - PAD_B
    cw = pw / max(n, 1)

    def X(idx):
        return ox + PAD_L + (idx - w0 + 0.5) * cw

    def Y(p):
        return oy + PAD_T + (hi - p) / (hi - lo) * ph

    out = []
    out.append(
        f'<rect x="{ox}" y="{oy}" width="{PW}" height="{PH}" fill="#fff" stroke="#ccc"/>'
    )
    dirs = {1: "bull", -1: "bear", 0: "neutral"}
    title = (
        f'{ev["kind"]} ({dirs.get(ev["dir"], ev["dir"])}) formed {ev["formed_utc"]} '
        f'known {ev["known_utc"]} UTC'
    )
    sub = f'idx {ev["formed_idx"]}/{ev["known_idx"]}  tags {ev["tags"]}'
    if ev.get("value") is not None:
        sub += f'  value {ev["value"]:.3g}'
    out.append(
        f'<text x="{ox + 8}" y="{oy + 14}" font-size="12" font-family="sans-serif" '
        f'fill="#222">{esc(title)}</text>'
    )
    out.append(
        f'<text x="{ox + 8}" y="{oy + 28}" font-size="10" font-family="sans-serif" '
        f'fill="#666">{esc(sub)}</text>'
    )
    # zone band
    if ev.get("lo") is not None and ev.get("hi") is not None:
        x0 = X(max(ev["start_idx"], w0)) - cw / 2
        x1 = ox + PAD_L + pw
        y0, y1 = Y(ev["hi"]), Y(ev["lo"])
        out.append(
            f'<rect x="{x0:.1f}" y="{y0:.1f}" width="{max(x1 - x0, 1):.1f}" '
            f'height="{max(y1 - y0, 1):.1f}" fill="#f2c14e" fill-opacity="0.25" '
            f'stroke="#d9a400" stroke-opacity="0.6"/>'
        )
    # candles
    for k, (o, h, l, c, _v) in enumerate(cs):
        x = X(w0 + k)
        col = "#1a9850" if c >= o else "#d73027"
        out.append(
            f'<line x1="{x:.1f}" y1="{Y(h):.1f}" x2="{x:.1f}" y2="{Y(l):.1f}" '
            f'stroke="{col}" stroke-width="1"/>'
        )
        top, bot = Y(max(o, c)), Y(min(o, c))
        bw = max(cw * 0.7, 1)
        out.append(
            f'<rect x="{x - bw / 2:.1f}" y="{top:.1f}" width="{bw:.1f}" '
            f'height="{max(bot - top, 0.8):.1f}" fill="{col}"/>'
        )
    # price line
    if ev.get("price") is not None:
        y = Y(ev["price"])
        out.append(
            f'<line x1="{ox + PAD_L}" y1="{y:.1f}" x2="{ox + PAD_L + pw}" y2="{y:.1f}" '
            f'stroke="#555" stroke-dasharray="4 3" stroke-width="1"/>'
        )
        out.append(
            f'<text x="{ox + PAD_L + pw + 3}" y="{y + 3:.1f}" font-size="9" '
            f'font-family="sans-serif" fill="#555">{ev["price"]:.1f}</text>'
        )
    # anchors
    if ev.get("ax") is not None and ev.get("ay") is not None:
        ax, ay = ev["ax"], ev["ay"]
        if ev.get("bx") is not None and ev.get("by") is not None:
            bx, by = ev["bx"], ev["by"]
            if ev["det"] == "trendline" and bx != ax:
                # extend the line to the event candle
                ex = ev["known_idx"]
                ey = ay + (by - ay) * (ex - ax) / (bx - ax)
            else:
                ex, ey = bx, by
            if ax >= w0:
                out.append(
                    f'<line x1="{X(ax):.1f}" y1="{Y(ay):.1f}" x2="{X(ex):.1f}" '
                    f'y2="{Y(ey):.1f}" stroke="#7b3294" stroke-width="1.5"/>'
                )
            for px, py in ((ax, ay), (bx, by)):
                if px >= w0:
                    out.append(
                        f'<circle cx="{X(px):.1f}" cy="{Y(py):.1f}" r="3" fill="#7b3294"/>'
                    )
        elif ax >= w0:
            out.append(
                f'<circle cx="{X(ax):.1f}" cy="{Y(ay):.1f}" r="3" fill="#7b3294"/>'
            )
    # formed / known markers
    yt, yb = oy + PAD_T, oy + PAD_T + ph
    fx, kx = X(ev["formed_idx"]), X(ev["known_idx"])
    out.append(
        f'<line x1="{fx:.1f}" y1="{yt}" x2="{fx:.1f}" y2="{yb}" stroke="#ff7f00" '
        f'stroke-dasharray="3 2" stroke-width="1.2"/>'
    )
    out.append(
        f'<line x1="{kx:.1f}" y1="{yt}" x2="{kx:.1f}" y2="{yb}" stroke="#2166ac" '
        f'stroke-width="1.2"/>'
    )
    return out


def sheet(det, total, events, meta):
    rows = max(1, (len(events) + COLS - 1) // COLS)
    W, H = COLS * PW + (COLS + 1) * 10, rows * PH + (rows + 1) * 10 + 30
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
        f'viewBox="0 0 {W} {H}">',
        f'<rect width="{W}" height="{H}" fill="#f7f7f7"/>',
        f'<text x="10" y="22" font-size="16" font-family="sans-serif" fill="#111">'
        f'{esc(det)}: {len(events)} random of {total} events '
        f'({esc(meta)})</text>',
    ]
    if not events:
        parts.append(
            '<text x="10" y="60" font-size="14" font-family="sans-serif" fill="#b00">'
            "no events</text>"
        )
    for k, ev in enumerate(events):
        r, c = divmod(k, COLS)
        ev = dict(ev)
        ev["det"] = det
        parts += panel(ev, 10 + c * (PW + 10), 40 + r * (PH + 10))
    parts.append("</svg>")
    return "\n".join(parts), W, H


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    with open(src, encoding="utf-8") as f:
        g = json.load(f)
    os.makedirs(dst, exist_ok=True)
    meta = f'{g["market"]} {g["symbol"]} {g["tf"]}'
    items = []
    for d in g["detectors"]:
        svg, _, _ = sheet(d["det"], d["total"], d["events"], meta)
        name = f'{d["det"]}.svg'
        with open(os.path.join(dst, name), "w", encoding="utf-8") as f:
            f.write(svg)
        items.append((d["det"], d["total"], len(d["events"]), name))
    legend = esc(__doc__.split("Legend per panel: ")[1].strip())
    rows = "\n".join(
        f'<h2 id="{esc(n)}">{esc(n)} <small>({k} shown of {t})</small></h2>'
        f'<img src="{esc(f)}" alt="{esc(n)} gallery" loading="lazy">'
        for n, t, k, f in items
    )
    toc = " | ".join(f'<a href="#{esc(n)}">{esc(n)}</a>' for n, _, _, _ in items)
    page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>Detector gallery</title>
<style>body{{font-family:sans-serif;margin:16px;background:#fafafa;color:#111}}
img{{max-width:100%;height:auto;border:1px solid #ddd;background:#fff}}
h2{{margin-top:28px}} small{{color:#666;font-weight:normal}}</style></head><body>
<h1>Detector gallery: {esc(meta)}</h1>
<p>{legend}</p><p>{toc}</p>
{rows}
</body></html>
"""
    with open(os.path.join(dst, "index.html"), "w", encoding="utf-8") as f:
        f.write(page)
    print(f"wrote {len(items)} sheets and index.html to {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
