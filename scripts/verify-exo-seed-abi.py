#!/usr/bin/env python3
"""Prove ExogenousSeedMarket ABI stub is on disk and documented (POL seed; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "exogenous_seed_market.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL exogenous_seed_market.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "fundV",
    "seed",
    "reserves",
    "reserve0",
    "reserve1",
    "registry",
    "tag",
    "token0",
    "token1",
    "owner",
    "setOwner",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL exogenous_seed_market.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("exogenous_seed_market.abi.json")',
    "EXOGENOUS_SEED_MARKET_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

# No user IPC encode path yet - stub only until Relic wire.
if "fn exo_seed_" in lib or "fn seed_exo_" in lib:
    print("FAIL unexpected exo seed IPC fn - keep stub-only until Relic wire")
    sys.exit(1)

bonds = (root / "docs" / "econ" / "BONDS.md").read_text(encoding="utf-8")
if "exogenous_seed_market.abi.json" not in bonds:
    print("FAIL BONDS.md missing exogenous_seed_market.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "ExogenousPairRegistry.sol").read_text(encoding="utf-8")
if "contract ExogenousSeedMarket" not in sol:
    print("FAIL ExogenousSeedMarket missing from ExogenousPairRegistry.sol")
    sys.exit(1)
for fn in ("function seed", "function fundV", "function reserves"):
    if fn not in sol:
        print("FAIL ExogenousSeedMarket missing", fn)
        sys.exit(1)

print("PASS exogenous_seed_market ABI stub present; docs needled; no user IPC encoder")
