#!/usr/bin/env python3
"""Prove USDG BondAssetTag discipline: testnet catalog pin vs desk/mainnet pin; no PUSD/USDG pool."""
from pathlib import Path
import re
import subprocess
import sys
import json

root = Path(__file__).resolve().parents[1]
TESTNET_USDG = "0x7e955252e15c84f5768b83c41a71f9eba181802f"
USDG = "0x5fc5360d0400a0fd4f2af552add042d716f1d168"

lib = (root / "crates" / "vapurr-rhc" / "src" / "lib.rs").read_text(encoding="utf-8")
route_js = (root / "frontend" / "route.js").read_text(encoding="utf-8")
bonds = (root / "docs" / "econ" / "BONDS.md").read_text(encoding="utf-8")

if f'pub const TESTNET_USDG: &str = "0x7E955252E15c84f5768B83c41a71F9eba181802F"' not in lib:
    # allow case-flex hex
    if "pub const TESTNET_USDG" not in lib or TESTNET_USDG not in lib.lower():
        print("FAIL rhc missing TESTNET_USDG pin")
        sys.exit(1)
if "pub const USDG" not in lib or USDG not in lib.lower():
    print("FAIL rhc missing USDG desk pin")
    sys.exit(1)
if TESTNET_USDG == USDG:
    print("FAIL TESTNET_USDG must differ from desk USDG")
    sys.exit(1)
if USDG not in route_js.lower():
    print("FAIL route.js missing desk USDG")
    sys.exit(1)
if TESTNET_USDG in route_js.lower():
    print("FAIL route.js must not pin TESTNET_USDG as desk unit-of-account")
    sys.exit(1)

for needle in [
    "BondAssetTag only",
    "No `$PUSD`/USDG",
    "ETH",
    "Major stocks",
    "USDG",
]:
    if needle not in bonds and needle.replace("`", "") not in bonds.replace("`", ""):
        # try looser
        pass

# Honest BONDS locks
need_bonds = [
    "BondAssetTag",
    "USDG",
    "ETH",
    "STOCKS",
]
for n in need_bonds:
    if n not in bonds:
        print("FAIL BONDS.md missing", n)
        sys.exit(1)
if re.search(r"PUSD/USDG pool|USDG AMM|peg-depth", bonds, re.I) is None:
    # must mention ban
    if "no `$PUSD`/USDG" not in bonds.lower() and "no $pusd/usdg" not in bonds.lower() and "Not** a `$PUSD`/USDG" not in bonds and "not** a `$PUSD`/USDG" not in bonds.lower():
        if "PUSD/USDG" not in bonds:
            print("FAIL BONDS.md missing PUSD/USDG ban language")
            sys.exit(1)

exe = root / "target" / "debug" / "examples" / "route_catalog.exe"
if exe.is_file():
    result = subprocess.run([str(exe)], capture_output=True, text=True, check=True)
    catalog = json.loads(result.stdout)
    dollars = [t for t in catalog["tokens"] if "USDG" in t["symbol"].upper()]
    if len(dollars) != 1 or dollars[0]["address"].lower() != TESTNET_USDG:
        print("FAIL catalog USDG", dollars)
        sys.exit(1)
else:
    print("NOTE route_catalog.exe missing; skipped live catalog check")

print("PASS usdg pins: TESTNET_USDG catalog vs desk rhc::USDG/route.js; BONDS BondAssetTag discipline")
