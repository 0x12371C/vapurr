#!/usr/bin/env python3
"""Prove LegacyVConverter ABI stub is on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "legacy_v_converter.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL legacy_v_converter.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = ["available", "canonicalV", "convert", "converted", "fund", "legacyLocked", "legacyV"]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL legacy_v_converter.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("legacy_v_converter.abi.json")',
    "LEGACY_V_CONVERTER_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn legacy_v_convert", "fn convert_legacy", "fn fund_converter")
for b in banned:
    if b in lib:
        print("FAIL unexpected LegacyVConverter IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "MINT_AUTHORITY.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "legacy_v_converter.abi.json" not in text:
    print("FAIL MINT_AUTHORITY.md missing legacy_v_converter.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "LegacyVConverter.sol").read_text(encoding="utf-8")
if "contract LegacyVConverter" not in sol:
    print("FAIL LegacyVConverter.sol missing contract LegacyVConverter")
    sys.exit(1)
for short in ("convert", "fund", "canonicalV", "legacyV", "available"):
    if short not in sol:
        print("FAIL LegacyVConverter.sol missing", short)
        sys.exit(1)

print("PASS legacy_v_converter ABI stub present; docs needled; no user IPC encoder")
