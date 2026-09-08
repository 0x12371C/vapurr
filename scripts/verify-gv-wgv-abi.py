#!/usr/bin/env python3
"""Prove gVAPURR + wgVAPURR ABI stubs are on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

checks = [
    (
        "gv_apurr.abi.json",
        "GV_APURR_ABI",
        ["accrue", "rebase", "stake", "unstake", "index", "shares", "policy", "vapurr"],
    ),
    (
        "wgv_apurr.abi.json",
        "WGV_APURR_ABI",
        ["wrap", "unwrap", "gV", "convertToAssets", "convertToShares", "gvPerShare"],
    ),
]
for fname, const, need in checks:
    path = econ / fname
    if not path.is_file():
        print(f"FAIL missing {path}")
        sys.exit(1)
    abi = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(abi, list):
        print(f"FAIL {fname}: expected ABI array")
        sys.exit(1)
    names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
    miss = [n for n in need if n not in names]
    if miss:
        print(f"FAIL {fname} missing funcs:", miss)
        sys.exit(1)
    for n in (f'include_str!("{fname}")', const):
        if n not in lib:
            print("FAIL lib.rs missing:", n)
            sys.exit(1)

banned = ("fn gv_stake", "fn gv_rebase", "fn wgv_wrap", "fn wgv_unwrap", "fn stake_gv")
for b in banned:
    if b in lib:
        print("FAIL unexpected gV/wgV IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "WGV_HOUSE.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "gv_apurr.abi.json" not in text or "wgv_apurr.abi.json" not in text:
    print("FAIL WGV_HOUSE.md missing gv/wgv abi needles")
    sys.exit(1)

sol = (root / "contracts" / "GvFed.sol").read_text(encoding="utf-8")
for needle in ("contract gVAPURR", "contract wgVAPURR", "function rebase", "function wrap"):
    if needle not in sol:
        print("FAIL GvFed.sol missing", needle)
        sys.exit(1)

print("PASS gv_apurr + wgv_apurr ABI stubs present; docs needled; no user IPC encoder")
