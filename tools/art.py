#!/usr/bin/env python3
"""Status Trends' title art, drawn as TikZ and compiled, not generated as
pixels (Sam's diagrams-as-code method, as the games' tools/art.py are made).

    python3 tools/art.py            # web/art/title.png and tools/art/title.tex

The look follows Sam_Comm_final.png in the vagrancy repository, as the
games' title art does: a warm yellow sky with outlined clouds, salmon,
coral and pink buildings, sage and teal greens, flat fills, everything
outlined in one plum ink, rooftop clutter (air-conditioning boxes,
antennas, water tanks, a billboard) and a giant green robot cat.

Facts the picture must get right, and where this file enforces each:

- text: no words, letters or numbers anywhere; the page sets the title.
  `document` refuses any TeX that could typeset text (\\node, \\text,
  \\put, \\char ...).
- the subject: the skyline is a bar chart. Four blocks of four buildings,
  one block per year, each block taller than the one before, so the street
  reads as growth across years and quarters (`skyline`). The billboard on
  the tallest roof carries a stacked column chart in the tool's three
  status colours, rising left to right (`billboard`).
- calm left third: the page sets the title and tagline there, so nothing
  but sky, a low pale skyline and the ground may enter x < W/3 (`calm`
  raises if a building, cloud or prop is placed there).
- size: 1920 x 720, opaque; at most 1.5 MB (`check`).
- colour is never the only cue: every filled shape has the plum outline.
- repeatable: every random choice comes from one seeded generator.
"""
import random
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "art"
SRC = ROOT / "tools" / "art"
PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"
PDFTOCAIRO = shutil.which("pdftocairo") or "/opt/homebrew/bin/pdftocairo"

W, H = 32.0, 12.0          # cm, at 60 px per cm
PX = (1920, 720)
CALM = W / 3

# Sam_Comm_final's palette, by eye, as in the games, plus the tool's three
# status colours for the billboard chart.
COLOURS = {
    "ink": "3B2340", "sky": "EFC46A", "skyhi": "F5DA92", "skylow": "E9B060", "cloud": "F2D58A",
    "wallA": "E88A73", "wallB": "EE9F7E", "wallC": "E47A7E", "wallD": "F0B07E", "wallE": "D9765F", "wallF": "E99A8A",
    "far": "F0C98A", "fardark": "E8B97A",
    "greenA": "8DBB7E", "greenB": "6FA77A", "greenC": "A9C97F", "greenD": "5E9C6A",
    "window": "F6E7A0", "windark": "8C5A64", "metal": "8D9A93", "metaldark": "5E6B66",
    "paving": "E2BD8C", "ground": "D3A77D", "road": "C9957A",
    "cardboard": "C9A06A", "tape": "E3CC98", "screen": "2E2638",
    "statreq": "2A78D6", "statpend": "EB6834", "statdone": "1BAF7A",
    "eyewhite": "FBF4E4", "trim": "F26B38", "gold": "FFB000",
}
WALLS = ["wallA", "wallB", "wallC", "wallD", "wallE", "wallF"]


def f(x):
    return f"{x:.3f}".rstrip("0").rstrip(".") if abs(x) > 0.0005 else "0"


class Pic:
    def __init__(self):
        self.lines = []

    def calm(self, x0, what):
        if x0 < CALM:
            raise SystemExit(f"{what} at x={x0:.2f} reaches into the calm left third (x < {CALM:.2f})")

    def fill(self, path, colour, lw=1.1):
        self.lines.append(f"\\filldraw[fill={colour}, draw=ink, line width={f(lw)}pt, line join=round] {path};")

    def stroke(self, path, lw=1.1, colour="ink"):
        self.lines.append(f"\\draw[{colour}, line width={f(lw)}pt, line cap=round, line join=round] {path};")

    def rect(self, x0, y0, x1, y1, colour, lw=1.1):
        self.fill(f"({f(x0)},{f(y0)}) rectangle ({f(x1)},{f(y1)})", colour, lw)

    def poly(self, pts, colour, lw=1.1):
        self.fill(" -- ".join(f"({f(x)},{f(y)})" for x, y in pts) + " -- cycle", colour, lw)

    def circle(self, x, y, r, colour, lw=1.1):
        self.fill(f"({f(x)},{f(y)}) circle ({f(r)})", colour, lw)

    def line(self, pts, lw=1.1, colour="ink"):
        self.stroke(" -- ".join(f"({f(x)},{f(y)})" for x, y in pts), lw, colour)

    def plain(self, x0, y0, x1, y1, colour):
        """A flat band with no outline: the sky only."""
        self.lines.append(f"\\fill[{colour}] ({f(x0)},{f(y0)}) rectangle ({f(x1)},{f(y1)});")


def cloud(p, x, y, s):
    p.calm(x - 1.4 * s, "a cloud")
    for dx, dy, r in [(-0.9, 0, 0.55), (-0.3, 0.35, 0.75), (0.45, 0.2, 0.62), (1.0, -0.05, 0.45)]:
        p.circle(x + dx * s, y + dy * s, r * s, "cloud", lw=1.0)
    p.line([(x - 1.1 * s, y - 0.3 * s), (x + 1.25 * s, y - 0.3 * s)], lw=1.0)


def roof(p, rng, x, y, w):
    """Rooftop clutter: boxes, antennas, a tank."""
    k = rng.random()
    if k < 0.4:
        bx = x + rng.uniform(0.15, max(0.2, w - 0.9))
        p.rect(bx, y, bx + 0.7, y + 0.45, "metal", lw=0.9)
        p.circle(bx + 0.35, y + 0.22, 0.13, "metaldark", lw=0.7)
    if k > 0.3:
        ax = x + rng.uniform(0.3, max(0.35, w - 0.3))
        h = rng.uniform(0.8, 1.6)
        p.line([(ax, y), (ax, y + h)], lw=1.0)
        for j in range(3):
            yy = y + h * (0.45 + 0.18 * j)
            p.line([(ax - 0.25, yy), (ax + 0.25, yy)], lw=0.8)
    if k > 0.75:
        tx = x + rng.uniform(0.2, max(0.25, w - 0.9))
        p.line([(tx + 0.1, y), (tx + 0.1, y + 0.4)], lw=0.9)
        p.line([(tx + 0.6, y), (tx + 0.6, y + 0.4)], lw=0.9)
        p.fill(f"({f(tx)},{f(y + 0.4)}) rectangle ({f(tx + 0.7)},{f(y + 1.1)})", rng.choice(["greenB", "greenD", "wallE"]), lw=0.9)


def building(p, rng, x, y0, w, h, wall, clutter=True):
    p.calm(x, "a building")
    p.rect(x, y0, x + w, y0 + h, wall)
    p.rect(x - 0.08, y0 + h, x + w + 0.08, y0 + h + 0.2, wall)
    cols = max(1, int((w - 0.3) / 0.55))
    rows = max(1, int((h - 0.6) / 0.72))
    gap = (w - cols * 0.3) / (cols + 1)
    for i in range(cols):
        for j in range(rows):
            wx = x + gap + i * (0.3 + gap)
            wy = y0 + 0.45 + j * 0.72
            if wy + 0.4 > y0 + h - 0.12:
                continue
            p.rect(wx, wy, wx + 0.3, wy + 0.4, "window" if rng.random() < 0.45 else "windark", lw=0.55)
    if clutter:
        roof(p, rng, x, y0 + h + 0.2, w)


def skyline(rng, ground):
    """Four years of four quarters: sixteen buildings, each year's block
    taller than the last, with quarter-to-quarter wobble. Returns the
    layout; `draw_skyline` draws it, so the cat can stand behind it."""
    x = CALM + 0.4
    span = W - x - 0.2
    w = span / 16
    out = []
    for year in range(4):
        base = 2.6 + 1.55 * year
        for q in range(4):
            h = base + rng.uniform(-0.25, 0.45) + 0.18 * q
            bw = w - rng.uniform(0.05, 0.22)
            out.append({"x": x + (w - bw) / 2, "w": bw, "h": h, "top": ground + h + 0.2,
                        "wall": WALLS[(year * 2 + q) % len(WALLS)], "clutter": year != 1 and (year, q) != (2, 1)})
            x += w
    return out


def draw_skyline(p, rng, ground, layout):
    for b in layout:
        building(p, rng, b["x"], ground, b["w"], b["h"], b["wall"], b["clutter"])


def far_skyline(p, rng, ground):
    """A low pale skyline behind everything, across the whole width; the
    only thing in the left third besides sky and ground."""
    x = 0.0
    while x < W:
        w = rng.uniform(0.9, 1.8)
        h = rng.uniform(0.8, 2.2) if x < CALM else rng.uniform(1.5, 4.0)
        p.rect(x, ground, x + w, ground + h, rng.choice(["far", "fardark"]), lw=0.7)
        x += w


def billboard(p, x, y, w, h):
    """A frame on legs with a stacked column chart that rises left to right."""
    p.calm(x, "the billboard")
    for lx in (x + 0.25 * w, x + 0.75 * w):
        p.rect(lx - 0.06, y - 0.7, lx + 0.06, y, "metaldark", lw=0.8)
    p.line([(x + 0.25 * w, y - 0.4), (x + 0.75 * w, y - 0.4)], lw=0.9)
    p.rect(x, y, x + w, y + h, "screen", lw=1.4)
    p.rect(x + 0.12, y + 0.12, x + w - 0.12, y + h - 0.12, "eyewhite", lw=0.8)
    n = 6
    cw = (w - 0.6) / n
    base = y + 0.3
    for i in range(n):
        total = (h - 0.75) * (0.35 + 0.65 * i / (n - 1))
        cx = x + 0.3 + i * cw + cw * 0.18
        bw = cw * 0.64
        done, pend = total * 0.62, total * 0.22
        yy = base
        for part, colour in [(done, "statdone"), (pend, "statpend"), (total - done - pend, "statreq")]:
            p.rect(cx, yy, cx + bw, yy + part, colour, lw=0.6)
            yy += part
    # The trend line over the columns, with an arrow head.
    pts = [(x + 0.3 + i * cw + cw * 0.5, base + (h - 0.75) * (0.35 + 0.65 * i / (n - 1)) + 0.12) for i in range(n)]
    p.line(pts, lw=1.6)
    ex, ey = pts[-1]
    p.poly([(ex + 0.05, ey + 0.05), (ex - 0.22, ey + 0.02), (ex - 0.04, ey - 0.2)], "ink", lw=0.6)


def robot_cat(p, cx, base, s, ground):
    """A giant green robot cat standing behind the street and looking over
    the roofs at the billboard, after the cat in Sam_Comm_final. Its chin
    rests on the lowest roof in front of it; its body shows only in the
    gaps between buildings."""
    p.calm(cx - 2.2 * s, "the cat")
    def P(x, y):
        return (cx + x * s, base + y * s)
    p.rect(cx - 1.9 * s, ground, cx + 1.9 * s, base + 0.4 * s, "greenD", lw=1.2)
    # Ears.
    p.poly([P(-1.75, 1.6), P(-1.45, 3.75), P(-0.45, 2.35)], "greenB", lw=1.3)
    p.poly([P(1.75, 1.6), P(1.45, 3.75), P(0.45, 2.35)], "greenB", lw=1.3)
    p.poly([P(-1.5, 1.9), P(-1.35, 3.2), P(-0.75, 2.35)], "greenC", lw=0.8)
    p.poly([P(1.5, 1.9), P(1.35, 3.2), P(0.75, 2.35)], "greenC", lw=0.8)
    # Head: a rounded box.
    x0, y0 = P(-2.1, -0.2)
    x1, y1 = P(2.1, 2.7)
    r = 0.9 * s
    p.fill(f"({f(x0)},{f(y0)}) [rounded corners={f(r)}cm] rectangle ({f(x1)},{f(y1)})", "greenA", lw=1.4)
    # Panel seams and rivets.
    p.line([P(-2.1, 1.75), P(2.1, 1.75)], lw=0.8)
    p.line([P(0, 1.75), P(0, 2.7)], lw=0.8)
    for x in (-1.8, -1.2, 1.2, 1.8):
        p.circle(*P(x, 2.3), 0.06 * s, "greenD", lw=0.5)
    # Goggle eyes, looking right toward the billboard.
    for ex in (-0.95, 0.95):
        p.circle(*P(ex, 1.05), 0.62 * s, "metaldark", lw=1.2)
        p.circle(*P(ex, 1.05), 0.48 * s, "eyewhite", lw=0.9)
        p.circle(*P(ex + 0.2, 1.12), 0.22 * s, "ink", lw=0.5)
        p.circle(*P(ex + 0.12, 1.22), 0.07 * s, "eyewhite", lw=0.3)
    # Snout and the orange mouth plate.
    p.poly([P(-0.18, 0.5), P(0.18, 0.5), P(0, 0.3)], "ink", lw=0.6)
    p.poly([P(-0.7, 0.18), P(0.7, 0.18), P(0.5, -0.12), P(-0.5, -0.12)], "trim", lw=1.0)
    for x in (-0.3, 0, 0.3):
        p.line([P(x, 0.18), P(x, -0.12)], lw=0.7)
    # Whisker struts.
    for side in (-1, 1):
        for dy in (0.42, 0.25):
            p.line([P(side * 0.85, dy), P(side * 1.75, dy)], lw=0.8)


def crates(p, rng, x, ground):
    """A stack of request boxes on the pavement, some ticked done."""
    p.calm(x, "the boxes")
    spots = [(0, 0, 1.0), (1.05, 0, 0.9), (0.45, 0.82, 0.8), (2.15, 0, 0.75)]
    for i, (dx, dy, sz) in enumerate(spots):
        bx, by = x + dx, ground + dy
        p.rect(bx, by, bx + sz, by + sz * 0.82, "cardboard", lw=1.0)
        p.rect(bx + sz * 0.42, by + sz * 0.82 - 0.16, bx + sz * 0.58, by + sz * 0.82, "tape", lw=0.6)
        if i % 2 == 0:
            cx, cy = bx + sz * 0.5, by + sz * 0.38
            p.line([(cx - 0.18 * sz, cy), (cx - 0.04 * sz, cy - 0.14 * sz), (cx + 0.22 * sz, cy + 0.18 * sz)], lw=1.6, colour="statdone")


def title():
    rng = random.Random(20261012)
    p = Pic()
    ground = 1.5
    p.plain(0, 0, W, H, "skylow")
    p.plain(0, 3.0, W, H, "sky")
    p.plain(0, 8.6, W, H, "skyhi")
    for x, y, s in [(14.5, 10.4, 0.9), (22.0, 9.6, 1.15), (29.2, 10.9, 0.8)]:
        cloud(p, x, y, s)
    far_skyline(p, rng, ground)
    layout = skyline(rng, ground)
    # The cat stands behind the second year's block and looks over it.
    second = layout[4:8]
    cx, scale = (second[1]["x"] + second[2]["x"] + second[2]["w"]) / 2, 0.95
    under = [b["top"] for b in layout if b["x"] < cx + 2.1 * scale and b["x"] + b["w"] > cx - 2.1 * scale]
    robot_cat(p, cx, min(under) - 0.05, scale, ground)
    draw_skyline(p, rng, ground, layout)
    # The billboard stands on the roof just right of the cat, in its gaze.
    board = layout[9]
    billboard(p, board["x"] + board["w"] / 2 - 1.6, board["top"] + 0.7, 3.2, 2.0)
    p.rect(0, 0, W, ground, "paving", lw=1.2)
    p.line([(0, ground * 0.45), (W, ground * 0.45)], lw=0.8)
    for i in range(int(W / 1.6)):
        x = i * 1.6 + 0.4
        p.line([(x, ground * 0.45), (x - 0.25, 0)], lw=0.7)
    crates(p, rng, 12.4, ground * 0.45 + 0.02)
    return p


def document(p):
    body = "\n".join(p.lines)
    for banned in ("\\node", "\\text", "\\put", "\\char", "\\pgftext", "label=", "node["):
        if banned in body:
            raise SystemExit(f"the picture would typeset text ({banned})")
    colours = "\n".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}" for k, v in COLOURS.items())
    return (
        "\\documentclass[tikz,border=0pt]{standalone}\n\\usepackage{xcolor}\n"
        "% Palette: Sam_Comm_final.png by eye, and the tool's status colours.\n"
        f"{colours}\n\\begin{{document}}\n\\begin{{tikzpicture}}[x=1cm,y=1cm]\n"
        f"\\useasboundingbox (0,0) rectangle ({f(W)},{f(H)});\n\\clip (0,0) rectangle ({f(W)},{f(H)});\n"
        f"{body}\n\\end{{tikzpicture}}\n\\end{{document}}\n"
    )


def check(png):
    data = png.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{png}: not a PNG")
    w, h = struct.unpack(">II", data[16:24])
    if (w, h) != PX:
        raise SystemExit(f"{png}: {w}x{h}, expected {PX[0]}x{PX[1]}")
    if len(data) > 1_500_000:
        raise SystemExit(f"{png}: {len(data)} bytes, over 1.5 MB")
    print(f"{png.relative_to(ROOT)}: {w}x{h}, {len(data) // 1024} KB")


def main():
    tex = document(title())
    SRC.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    (SRC / "title.tex").write_text(tex)
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        (t / "art.tex").write_text(tex)
        run = subprocess.run([PDFLATEX, "-interaction=nonstopmode", "-halt-on-error", "art.tex"], cwd=t, capture_output=True, text=True)
        if run.returncode:
            sys.stderr.write(run.stdout[-3000:])
            raise SystemExit("pdflatex failed")
        subprocess.run([PDFTOCAIRO, "-png", "-scale-to-x", str(PX[0]), "-scale-to-y", str(PX[1]), "-singlefile", "art.pdf", "art"], cwd=t, check=True)
        shutil.copy(t / "art.png", OUT / "title.png")
    check(OUT / "title.png")


if __name__ == "__main__":
    main()
