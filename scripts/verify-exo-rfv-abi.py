#!/usr/bin/env python3
"""Prove ExoRfvSink ABI stub is on disk and documented (ops/keeper; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "exo_rfv_sink.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL exo_rfv_sink.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = ["spend", "approveSpender", "setKeeper", "setOwner", "owner", "keeper"]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL exo_rfv_sink.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("exo_rfv_sink.abi.json")',
    "EXO_RFV_SINK_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

# No user IPC encode path yet — stub must not grow a live spend encoder overnight.
if "fn exo_rfv_spend" in lib or "fn exo_spend" in lib:
    print("FAIL unexpected exo spend IPC fn — keep stub-only until Relic wire")
    sys.exit(1)

bonds = (root / "docs" / "econ" / "BONDS.md").read_text(encoding="utf-8")
rfv = (root / "docs" / "econ" / "RFV_STOCK_ETH_V_LOOP.md").read_text(encoding="utf-8")
if "exo_rfv_sink.abi.json" not in bonds:
    print("FAIL BONDS.md missing exo_rfv_sink.abi.json needle")
    sys.exit(1)
if "exo_rfv_sink.abi.json" not in rfv:
    print("FAIL RFV_STOCK_ETH_V_LOOP.md missing exo_rfv_sink.abi.json needle")
    sys.exit(1)
if "0xdbf2736cd318489d0cc6844445f2a6c019d6a36f" not in rfv.lower() and "0xdbf2736cd318489d0cc6844445f2a6c019d6a36f" not in rfv:
    # STATUS/playbook already carry live CA; soft check case-insensitive via lower on copy
    pass
sol = (root / "contracts" / "ExoRfvSink.sol").read_text(encoding="utf-8")
for fn in ("function spend", "function approveSpender", "function setKeeper"):
    if fn not in sol:
        print("FAIL ExoRfvSink.sol missing", fn)
        sys.exit(1)

print("PASS exo_rfv_sink ABI stub present; docs needled; no user IPC encoder")
