#!/usr/bin/env python3
"""Prove House remittance destination address-book stub is wired."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
html = (root / "frontend" / "pusd.html").read_text(encoding="utf-8")
econ = (root / "crates" / "vapurr-econ" / "src" / "lib.rs").read_text(encoding="utf-8")

need = [
    'id="h-remit-book"',
    'id="h-remit-fee"',
    'id="h-remit-skim"',
    'id="h-remit-attr"',
    'id="h-remit-sink"',
    "R.house_fee_remit",
    "R.house_uni_skim",
    "R.fee_attribution",
    "R.remittance_sink",
    "NeedRemittance",
    "remitCa(",
]
missing = [n for n in need if n not in html]
if missing:
    print("FAIL missing in pusd.html:", missing)
    sys.exit(1)

for k in ["house_fee_remit", "house_uni_skim", "fee_attribution", "remittance_sink", "configured"]:
    if f'"{k}"' not in econ:
        print("FAIL remittance_book_snap missing", k)
        sys.exit(1)

print("PASS remittance destination address-book stub")
