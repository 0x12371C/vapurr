#!/usr/bin/env python3
"""Prove HousePairConfig ABI stub is on disk and documented (ops/deploy; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "house_pair_config.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL house_pair_config.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "gV",
    "isHousePair",
    "pusd",
    "requireHouseEquity",
    "requireHousePair",
    "wgV",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL house_pair_config.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("house_pair_config.abi.json")',
    "HOUSE_PAIR_CONFIG_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn house_pair_config_", "fn require_house_pair", "fn require_house_equity")
for b in banned:
    if b in lib:
        print("FAIL unexpected HousePairConfig IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "WGV_HOUSE.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "house_pair_config.abi.json" not in text:
    print("FAIL WGV_HOUSE.md missing house_pair_config.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "HousePairConfig.sol").read_text(encoding="utf-8")
if "contract HousePairConfig" not in sol:
    print("FAIL HousePairConfig.sol missing contract HousePairConfig")
    sys.exit(1)
for fn in (
    "function requireHouseEquity",
    "function requireHousePair",
    "function isHousePair",
    "function wgV",
):
    # immutables use `address public immutable override wgV` — accept either shape
    short = fn.split()[-1]
    if short not in sol and fn not in sol:
        print("FAIL HousePairConfig missing", short)
        sys.exit(1)

print("PASS house_pair_config ABI stub present; docs needled; no user IPC encoder")
