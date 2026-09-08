#!/usr/bin/env python3
"""Prove BrowserStream ABI stub is on disk and documented (ops/earn drip; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "browser_stream.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL browser_stream.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "CAP",
    "DURATION",
    "vapurr",
    "owner",
    "distributor",
    "start",
    "released",
    "started",
    "setOwner",
    "setDistributor",
    "fund",
    "startStream",
    "vested",
    "releasable",
    "drip",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL browser_stream.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("browser_stream.abi.json")',
    "BROWSER_STREAM_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

if "fn browser_stream_" in lib or "fn drip_browser" in lib:
    print("FAIL unexpected BrowserStream IPC fn - keep stub-only until Relic wire")
    sys.exit(1)

doc = (root / "docs" / "econ" / "GENESIS_ALLOCATION.md").read_text(encoding="utf-8")
if "browser_stream.abi.json" not in doc:
    print("FAIL GENESIS_ALLOCATION.md missing browser_stream.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "GvFed.sol").read_text(encoding="utf-8")
if "contract BrowserStream" not in sol:
    print("FAIL GvFed.sol missing BrowserStream")
    sys.exit(1)
for fn in ("function fund", "function startStream", "function drip", "function releasable"):
    if fn not in sol:
        print("FAIL BrowserStream missing", fn)
        sys.exit(1)

print("PASS browser_stream ABI stub present; docs needled; no user IPC encoder")
