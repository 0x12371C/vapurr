#!/usr/bin/env python3
"""Prove DevFundStream ABI stub is on disk and documented (ops/treasury; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "dev_fund_stream.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL dev_fund_stream.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "startStream",
    "settle",
    "drawPusd",
    "fund",
    "vested",
    "unsettleable",
    "lockedInOliver",
    "recipient",
    "owner",
    "setOwner",
    "setRecipient",
    "vapurr",
    "oliver",
    "pusd",
    "started",
    "repayPusd",
    "oliverCollateral",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL dev_fund_stream.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("dev_fund_stream.abi.json")',
    "DEV_FUND_STREAM_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

if "fn dev_fund_" in lib or "fn draw_pusd_dev" in lib:
    print("FAIL unexpected DevFund IPC fn - keep stub-only until Relic wire")
    sys.exit(1)

dev = (root / "docs" / "econ" / "DEV_FUND.md").read_text(encoding="utf-8")
if "dev_fund_stream.abi.json" not in dev:
    print("FAIL DEV_FUND.md missing dev_fund_stream.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "DevFundStream.sol").read_text(encoding="utf-8")
if "contract DevFundStream" not in sol:
    print("FAIL DevFundStream.sol missing contract")
    sys.exit(1)
for fn in ("function startStream", "function settle", "function drawPusd"):
    if fn not in sol:
        print("FAIL DevFundStream missing", fn)
        sys.exit(1)

print("PASS dev_fund_stream ABI stub present; docs needled; no user IPC encoder")
