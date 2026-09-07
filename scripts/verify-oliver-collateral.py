#!/usr/bin/env python3
"""Prove Oliver credit stays $VAPURR-collateral-only until Relic opens gV/wgV types."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
html = (root / "frontend" / "pusd.html").read_text(encoding="utf-8")
notes = (root / "docs" / "econ" / "WGV_HOUSE.md").read_text(encoding="utf-8")
snap = (root / "docs" / "SNAPSHOT.md").read_text(encoding="utf-8")

need_html = [
    'id="e-collat-note"',
    "Live collateral is $VAPURR only",
    "gV/wgV Oliver collateral types stay closed",
    "House still wraps gV to wgV",
]
missing = [n for n in need_html if n not in html]
if missing:
    print("FAIL missing in pusd.html:", missing)
    sys.exit(1)

# Must not advertise a live gV/wgV Oliver collateral picker.
banned = [
    'id="e-collat-gv"',
    'id="e-collat-wgv"',
    "Supply gV as Oliver collateral",
    "Supply wgV as Oliver collateral",
    'data-collat="wgV"',
    'data-collat="gV"',
]
hit = [b for b in banned if b in html]
if hit:
    print("FAIL pusd.html advertises closed Oliver collateral types:", hit)
    sys.exit(1)

for needle in [
    "Oliver credit collateral today = **$VAPURR only**",
    "separate Relic go",
    "#e-collat-note",
]:
    if needle not in notes:
        print("FAIL WGV_HOUSE.md missing", needle)
        sys.exit(1)

if "Oliver gV/wgV collateral" not in snap:
    print("FAIL SNAPSHOT.md missing Oliver gV/wgV collateral gated note")
    sys.exit(1)

print("PASS Oliver collateral boundary ($VAPURR only; gV/wgV closed)")
