#!/usr/bin/env python3
"""Prove GenesisTreasury ABI stub is on disk and documented (ops/treasury; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "genesis_treasury.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL genesis_treasury.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "collateralizeOliver",
    "fund",
    "gV",
    "lock",
    "oliver",
    "pusd",
    "vapurr",
    "withdrawV",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL genesis_treasury.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("genesis_treasury.abi.json")',
    "GENESIS_TREASURY_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn genesis_treasury_", "fn collateralize_oliver", "fn treasury_fund")
for b in banned:
    if b in lib:
        print("FAIL unexpected GenesisTreasury IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "GENESIS_ALLOCATION.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "genesis_treasury.abi.json" not in text:
    print("FAIL GENESIS_ALLOCATION.md missing genesis_treasury.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "GenesisTreasury.sol").read_text(encoding="utf-8")
if "contract GenesisTreasury" not in sol:
    print("FAIL GenesisTreasury.sol missing contract GenesisTreasury")
    sys.exit(1)
for short in ("collateralizeOliver", "fund", "lock", "withdrawV"):
    if short not in sol:
        print("FAIL GenesisTreasury.sol missing", short)
        sys.exit(1)

print("PASS genesis_treasury ABI stub present; docs needled; no user IPC encoder")
