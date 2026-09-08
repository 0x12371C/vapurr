#!/usr/bin/env python3
"""Prove LitheCutoverMigrator ABI stub is on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "lithe_cutover_migrator.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL lithe_cutover_migrator.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "canonicalMarket",
    "canonicalPusd",
    "canonicalV",
    "converter",
    "legacyMarket",
    "legacyPusd",
    "legacyV",
    "migrate",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL lithe_cutover_migrator.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("lithe_cutover_migrator.abi.json")',
    "LITHE_CUTOVER_MIGRATOR_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn lithe_cutover_", "fn migrate_legacy", "fn cutover_migrate")
for b in banned:
    if b in lib:
        print("FAIL unexpected LitheCutoverMigrator IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "MINT_AUTHORITY.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "lithe_cutover_migrator.abi.json" not in text:
    print("FAIL MINT_AUTHORITY.md missing lithe_cutover_migrator.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "LitheCutoverMigrator.sol").read_text(encoding="utf-8")
if "contract LitheCutoverMigrator" not in sol:
    print("FAIL LitheCutoverMigrator.sol missing contract LitheCutoverMigrator")
    sys.exit(1)
for short in ("migrate", "legacyMarket", "canonicalMarket", "converter"):
    if short not in sol:
        print("FAIL LitheCutoverMigrator.sol missing", short)
        sys.exit(1)

print("PASS lithe_cutover_migrator ABI stub present; docs needled; no user IPC encoder")
