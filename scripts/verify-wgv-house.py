#!/usr/bin/env python3
"""Prove wgV House UI + operator notes stay honest (wrap-first, no raw gV)."""
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
html = (root / "frontend" / "pusd.html").read_text(encoding="utf-8")
econ = (root / "crates" / "vapurr-econ" / "src" / "lib.rs").read_text(encoding="utf-8")
notes = (root / "docs" / "econ" / "WGV_HOUSE.md").read_text(encoding="utf-8")
pair = (root / "docs" / "econ" / "HOUSE_PAIR.md").read_text(encoding="utf-8")

need_html = [
    'id="h-gate"',
    'id="h-fee-note"',
    'id="h-attrib"',
    "Wrap gV to wgV before supplying House",
    "wgV / $PUSD",
    "NeedRemittance",
    "No USDG",
]
missing_html = [n for n in need_html if n not in html]
if missing_html:
    print("FAIL missing in pusd.html:", missing_html)
    sys.exit(1)

if "raw gV / $PUSD" in html:
    print("FAIL pusd.html teaches raw gV / $PUSD as House pair")
    sys.exit(1)
# Ban bare "gV / $PUSD book" but allow "wgV / $PUSD book"
if re.search(r"(?<![wW])gV / \$PUSD book", html):
    print("FAIL pusd.html teaches bare gV / $PUSD book")
    sys.exit(1)
if "wgV / $PUSD book" not in html:
    print("FAIL pusd.html missing wgV / $PUSD book copy")
    sys.exit(1)

if "NeedHouse" not in econ:
    print("FAIL econ missing NeedHouse")
    sys.exit(1)

for needle in [
    "wgVAPURR / $PUSD",
    "Raw rebasing",
    "requireHousePair",
    "Oliver credit collateral",
    "Live Uni v4",
]:
    if needle not in notes:
        print("FAIL WGV_HOUSE.md missing", needle)
        sys.exit(1)

for needle in ["requireHousePair", "RawGvNotHouseEquity", "wgVAPURR", "$PUSD"]:
    if needle not in pair:
        print("FAIL HOUSE_PAIR.md missing", needle)
        sys.exit(1)

print("PASS wgV House UI + WGV_HOUSE / HOUSE_PAIR honesty")
