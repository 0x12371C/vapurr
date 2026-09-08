#!/usr/bin/env python3
"""Prove House remittance book + Remit fees CTA + FeeAttribution who-paid chips."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
html = (root / "frontend" / "pusd.html").read_text(encoding="utf-8")
econ = (root / "crates" / "vapurr-econ" / "src" / "lib.rs").read_text(encoding="utf-8")
house = (root / "docs" / "econ" / "HOUSE_PAIR.md").read_text(encoding="utf-8")

need = [
    'id="h-remit-book"',
    'id="h-remit-fee"',
    'id="h-remit-skim"',
    'id="h-remit-attr"',
    'id="h-remit-sink"',
    'id="h-remit-note"',
    'id="h-remit-amt"',
    'id="h-remit-cta"',
    'id="h-remit-err"',
    'id="h-attrib"',
    'id="h-attrib-house"',
    'id="h-attrib-lithe"',
    'id="h-attrib-oliver"',
    'id="h-fee-note"',
    "Remit fees",
    "FeeAttribution who paid",
    "R.house_fee_remit",
    "R.house_uni_skim",
    "R.fee_attribution",
    "R.remittance_sink",
    "NeedRemittance",
    "remitCa(",
    "econ-house-fee-remit",
    "FeeAttribution.breakdown()",
]
missing = [n for n in need if n not in html]
if missing:
    print("FAIL missing in pusd.html:", missing)
    sys.exit(1)

# Who-paid chips stay em-dash until live breakdown reads land.
if 'ah.textContent = "\u2014"' not in html and 'ah.textContent = "—"' not in html:
    print("FAIL who-paid chips must reset to em-dash until FeeAttribution.breakdown()")
    sys.exit(1)
if "FeeAttribution.breakdown()" not in html:
    print("FAIL missing FeeAttribution.breakdown() honesty comment/needle")
    sys.exit(1)

cta_idx = html.find('id="h-remit-cta"')
cta_snip = html[max(0, cta_idx - 120) : cta_idx + 160]
if "disabled" in cta_snip.lower():
    print("FAIL Remit fees CTA appears disabled in markup")
    sys.exit(1)
if "pointer-events:none" in cta_snip.replace(" ", ""):
    print("FAIL Remit fees CTA pointer-events none")
    sys.exit(1)

for k in ["house_fee_remit", "house_uni_skim", "fee_attribution", "remittance_sink", "configured"]:
    if f'"{k}"' not in econ:
        print("FAIL remittance_book_snap missing", k)
        sys.exit(1)

if "NeedRemittance" not in house:
    print("FAIL HOUSE_PAIR.md missing remittance IPC honesty")
    sys.exit(1)
if "FeeAttribution who-paid" not in house and "#h-attrib" not in house:
    print("FAIL HOUSE_PAIR.md missing FeeAttribution who-paid UI stub note")
    sys.exit(1)
if "verify-remittance-book.py" not in house:
    print("FAIL HOUSE_PAIR.md missing verify-remittance-book prove pointer")
    sys.exit(1)

print("PASS remittance book + Remit fees CTA + FeeAttribution who-paid chips")
