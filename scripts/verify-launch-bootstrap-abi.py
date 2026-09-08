#!/usr/bin/env python3
"""Prove LaunchBootstrap ABI stub is on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "launch_bootstrap.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL launch_bootstrap.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "amdSeed",
    "browserStream",
    "claimHouseSeed",
    "devFund",
    "ethSeed",
    "fundAndStart",
    "funded",
    "gV",
    "houseSeedHeld",
    "initiator",
    "nvdaSeed",
    "oliver",
    "pullAmount",
    "registry",
    "treasury",
    "treasuryRemainder",
    "vapurr",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL launch_bootstrap.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("launch_bootstrap.abi.json")',
    "LAUNCH_BOOTSTRAP_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn launch_bootstrap_", "fn fund_and_start", "fn claim_house_seed", "fn pull_amount")
for b in banned:
    if b in lib:
        print("FAIL unexpected LaunchBootstrap IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "GENESIS_ALLOCATION.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "launch_bootstrap.abi.json" not in text:
    print("FAIL GENESIS_ALLOCATION.md missing launch_bootstrap.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "LaunchBootstrap.sol").read_text(encoding="utf-8")
if "contract LaunchBootstrap" not in sol:
    print("FAIL LaunchBootstrap.sol missing contract LaunchBootstrap")
    sys.exit(1)
for short in ("fundAndStart", "claimHouseSeed", "pullAmount"):
    if short not in sol:
        print("FAIL LaunchBootstrap.sol missing", short)
        sys.exit(1)

print("PASS launch_bootstrap ABI stub present; docs needled; no user IPC encoder")
