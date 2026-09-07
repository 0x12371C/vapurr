#!/usr/bin/env python3
"""Prove sPUSD CD sketch stays live-CTA + NeedSavings honest."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
html = (root / "frontend" / "bonds.html").read_text(encoding="utf-8")
econ = (root / "crates" / "vapurr-econ" / "src" / "lib.rs").read_text(encoding="utf-8")
spusd = (root / "docs" / "econ" / "SPUSD.md").read_text(encoding="utf-8")

need_html = [
    'id="spusd-cd"',
    'id="cd-book-note"',
    'id="cd-cta"',
    'id="cd-amt"',
    "Open CD",
    "NeedSavings",
    "econ-cd-open",
    "Sav.configured",
]
missing_html = [n for n in need_html if n not in html]
if missing_html:
    print("FAIL missing in bonds.html:", missing_html)
    sys.exit(1)

# Live CTA: Open CD must not be gray-gated / disabled by default.
cta_idx = html.find('id="cd-cta"')
if cta_idx < 0:
    print("FAIL cd-cta missing")
    sys.exit(1)
cta_snip = html[max(0, cta_idx - 120) : cta_idx + 160]
if "disabled" in cta_snip.lower():
    print("FAIL Open CD CTA appears disabled in markup")
    sys.exit(1)
if "pointer-events:none" in cta_snip.replace(" ", ""):
    print("FAIL Open CD CTA pointer-events none")
    sys.exit(1)

for k in ["spusd", "spusd_cd", "savings_router", "configured"]:
    if f'"{k}"' not in econ or "fn savings_book_snap" not in econ:
        print("FAIL savings_book_snap missing", k)
        sys.exit(1)
if "NeedSavings" not in econ:
    print("FAIL econ missing NeedSavings")
    sys.exit(1)

for needle in [
    "SavingsRouter",
    "SpusdCd",
    "NeedSavings",
    "live CTA stub",
    "econ-cd-open",
    "Open CD",
    "cdBps",
    "availableSurplus",
]:
    if needle not in spusd:
        print("FAIL SPUSD.md missing", needle)
        sys.exit(1)

print("PASS sPUSD CD live CTA + NeedSavings + SPUSD.md sketch")
