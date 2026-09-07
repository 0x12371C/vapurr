#!/usr/bin/env python3
"""Prove Bonds / sPUSD CD address-book stubs stay honest-empty."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
html = (root / "frontend" / "bonds.html").read_text(encoding="utf-8")
econ = (root / "crates" / "vapurr-econ" / "src" / "lib.rs").read_text(encoding="utf-8")
bonds_md = (root / "docs" / "econ" / "BONDS.md").read_text(encoding="utf-8")

need_html = [
    'id="bond-book-note"',
    'id="cd-book-note"',
    'NeedBondMarket',
    'NeedSavings',
    'US_EQUITY_FED_OPS',
    'CRYPTO_BOND_FED_OPS',
    'equitySessionNow',
    'Bon.configured',
    'Sav.configured',
]
missing_html = [n for n in need_html if n not in html]
if missing_html:
    print('FAIL missing in bonds.html:', missing_html)
    sys.exit(1)

# ETH / USDG / stocks tab chrome - live-by-default, never gray-gated.
need_tabs = [
    'role="tablist"',
    'data-asset="ETH"',
    'data-asset="USDG"',
    'data-asset="AMD"',  # STOCKS-tagged major
    'id="bond-cta"',
    'never gray-gate',
]
missing_tabs = [n for n in need_tabs if n not in html]
if missing_tabs:
    print('FAIL bonds.html missing ETH/USDG/stocks tab posture:', missing_tabs)
    sys.exit(1)
if 'cta.disabled = false' not in html:
    print('FAIL bonds.html must keep bond CTA live (cta.disabled = false)')
    sys.exit(1)

for k in ['bond_market', 'configured']:
    if f'"{k}"' not in econ or 'fn bonds_book_snap' not in econ:
        print('FAIL bonds_book_snap missing', k)
        sys.exit(1)
for k in ['spusd', 'spusd_cd', 'savings_router', 'configured']:
    if f'"{k}"' not in econ or 'fn savings_book_snap' not in econ:
        print('FAIL savings_book_snap missing', k)
        sys.exit(1)
if 'NeedBondMarket' not in econ or 'NeedSavings' not in econ:
    print('FAIL econ missing NeedBondMarket/NeedSavings')
    sys.exit(1)

# Canon: USDG is BondAssetTag only; ETH/USDG/STOCKS tabs live-by-default.
for needle in ['BondAssetTag', 'USDG', 'ETH', 'STOCKS', 'NeedBondMarket', 'Do not gray-gate']:
    if needle not in bonds_md:
        print('FAIL BONDS.md missing', needle)
        sys.exit(1)
if 'PUSD/USDG' in bonds_md and 'Banned as pairs' not in bonds_md:
    print('FAIL BONDS.md lost USDG pool ban posture')
    sys.exit(1)

print('PASS bonds/CD address-book + BONDS.md ETH/USDG/STOCKS tab posture')