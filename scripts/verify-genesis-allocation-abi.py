#!/usr/bin/env python3
"""Prove GenesisAllocation ABI stub matches forge + GENESIS hard lock (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

fname = "genesis_allocation.abi.json"
const = "GENESIS_ALLOCATION_ABI"
path = econ / fname
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL expected ABI array")
    sys.exit(1)

fns = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = {
    "GENESIS_MINT",
    "LAUNCH_V",
    "DEV_FUND_AMOUNT",
    "BROWSERSTREAM_V",
    "HOUSE_SEED_V",
    "POL_ETH_V",
    "POL_NVDA_V",
    "POL_AMD_V",
    "TREASURY_GROSS",
}
missing = sorted(need - fns)
if missing:
    print("FAIL missing fns", missing)
    sys.exit(1)

for n in (f'include_str!("{fname}")', const):
    if n not in lib:
        print("FAIL lib.rs missing:", n)
        sys.exit(1)

banned = ("fn genesis_mint", "fn allocate_genesis", "fn fund_and_start_alloc")
for b in banned:
    if b in lib:
        print("FAIL unexpected genesis IPC fn - keep stub-only")
        sys.exit(1)

doc = root / "docs" / "econ" / "GENESIS_ALLOCATION.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "genesis_allocation.abi.json" not in text or "GENESIS_ALLOCATION_ABI" not in text:
    print("FAIL GENESIS_ALLOCATION.md missing abi needles")
    sys.exit(1)

art = root / "contracts" / "out-forge" / "GenesisAllocation.sol" / "GenesisAllocation.json"
if not art.is_file():
    print("FAIL missing forge artifact", art)
    sys.exit(1)
art_abi = json.loads(art.read_text(encoding="utf-8"))["abi"]
if art_abi != abi:
    print("FAIL stub ABI != forge GenesisAllocation artifact")
    sys.exit(1)

# Soft check sol constants if present
sol = (root / "contracts" / "GenesisAllocation.sol").read_text(encoding="utf-8")
for needle_s in ("1_200_000", "1_000_000", "200_000", "50_000"):
    if needle_s not in sol:
        print("WARN sol missing", needle_s)

print("PASS genesis_allocation ABI stub present; docs needled; matches forge; no user IPC encoder")
