#!/usr/bin/env python3
"""Prove PusdMarketFedUpgradeable ABI stub is on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

fname = "pusd_market_fed_upgradeable.abi.json"
const = "PUSD_MARKET_FED_UPGRADEABLE_ABI"
path = econ / fname
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL expected ABI array")
    sys.exit(1)

types = {x.get("type") for x in abi if isinstance(x, dict)}
for need in ("constructor", "error", "event", "function"):
    if need not in types:
        print("FAIL abi missing type:", need)
        sys.exit(1)

fns = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
for need in ("initialize", "upgradeToAndCall", "proxiableUUID", "swapPusdToV", "swapVToPusd", "accrue"):
    if need not in fns:
        print("FAIL missing fn", need)
        sys.exit(1)

for n in (f'include_str!("{fname}")', const):
    if n not in lib:
        print("FAIL lib.rs missing:", n)
        sys.exit(1)

banned = ("fn swap_pusd_to_v", "fn swap_v_to_pusd", "fn market_fed_initialize", "fn upgrade_market_fed")
for b in banned:
    if b in lib:
        print("FAIL unexpected market-fed IPC fn - keep stub-only until Relic cutover")
        sys.exit(1)

doc = root / "docs" / "econ" / "PROXY_DEPLOY_GATE.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "pusd_market_fed_upgradeable.abi.json" not in text or "PUSD_MARKET_FED_UPGRADEABLE_ABI" not in text:
    print("FAIL PROXY_DEPLOY_GATE.md missing abi needles")
    sys.exit(1)

art = root / "contracts" / "out-forge" / "PusdMarketFedUpgradeable.sol" / "PusdMarketFedUpgradeable.json"
if not art.is_file():
    print("FAIL missing forge artifact", art)
    sys.exit(1)

print("PASS pusd_market_fed_upgradeable ABI stub present; docs needled; no user IPC encoder")
