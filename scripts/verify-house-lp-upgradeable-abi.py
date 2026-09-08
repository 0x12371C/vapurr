#!/usr/bin/env python3
"""Prove HouseLpUpgradeable ABI stub is wired (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

fname = "house_lp_upgradeable_impl.abi.json"
const = "HOUSE_LP_UPGRADEABLE_IMPL_ABI"
path = econ / fname
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
if f'include_str!("{fname}")' not in lib or const not in lib:
    print("FAIL lib.rs missing include/const")
    sys.exit(1)

abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL expected ABI array")
    sys.exit(1)

fns = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = {
    "adopt",
    "initialize",
    "liquidity",
    "poolId",
    "tokenId",
    "upgradeToAndCall",
    "houseLpVersion",
    "snapshot",
    "seed",
    "setOwner",
    "vapurr",
    "pusd",
}
missing = sorted(need - fns)
if missing:
    print("FAIL missing fns:", ", ".join(missing))
    sys.exit(1)

doc_paths = [
    root / "docs" / "econ" / "HOUSE_PAIR.md",
    root / "docs" / "econ" / "WGV_HOUSE.md",
]
if not any(p.is_file() and fname in p.read_text(encoding="utf-8") for p in doc_paths):
    print("FAIL docs needle missing")
    sys.exit(1)

print(f"PASS {fname} entries={len(abi)} fns={len(fns)} const={const}")
