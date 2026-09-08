#!/usr/bin/env python3
"""Prove ExogenousPairRegistry ABI stub is on disk and documented (POL/bootstrap; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "exogenous_pair_registry.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL exogenous_pair_registry.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "registerPair",
    "bindPool",
    "setEnabled",
    "pairOf",
    "requireExogenousPair",
    "isExogenousPair",
    "validateAndMark",
    "vapurr",
    "usdgBanned",
    "pusdBanned",
    "owner",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL exogenous_pair_registry.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("exogenous_pair_registry.abi.json")',
    "EXOGENOUS_PAIR_REGISTRY_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

# No user IPC encode path yet - stub only until Relic wire.
if "fn exo_pair_" in lib or "fn register_exo_pair" in lib:
    print("FAIL unexpected exo pair IPC fn - keep stub-only until Relic wire")
    sys.exit(1)

bonds = (root / "docs" / "econ" / "BONDS.md").read_text(encoding="utf-8")
if "exogenous_pair_registry.abi.json" not in bonds:
    print("FAIL BONDS.md missing exogenous_pair_registry.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "ExogenousPairRegistry.sol").read_text(encoding="utf-8")
for fn in ("function registerPair", "function bindPool", "function validateAndMark"):
    if fn not in sol:
        print("FAIL ExogenousPairRegistry.sol missing", fn)
        sys.exit(1)
if "contract ExogenousSeedMarket" not in sol:
    print("FAIL ExogenousSeedMarket missing from same sol file")
    sys.exit(1)

print("PASS exogenous_pair_registry ABI stub present; docs needled; no user IPC encoder")
