#!/usr/bin/env python3
"""Prove House remittance destination address-book + live Remit fees CTA."""
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
    "Remit fees",
    "R.house_fee_remit",
    "R.house_uni_skim",
    "R.fee_attribution",
    "R.remittance_sink",
    "NeedRemittance",
    "remitCa(",
    "econ-house-fee-remit",
]
missing = [n for n in need if n not in html]
if missing:
    print("FAIL missing in pusd.html:", missing)
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

if "econ-house-fee-remit" not in house and "NeedRemittance" not in house:
    print("FAIL HOUSE_PAIR.md missing remittance IPC honesty")
    sys.exit(1)
if "Remit fees" not in house and "live CTA" not in house.lower():
    # soft: require a CTA mention after this slice
    if "h-remit-cta" not in house and "Remit fees stays live" not in house:
        print("FAIL HOUSE_PAIR.md missing Remit fees live-CTA note")
        sys.exit(1)

print("PASS remittance destination address-book + live Remit fees CTA")
