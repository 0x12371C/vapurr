#!/usr/bin/env python3
"""Prove CanonicalLitheFactory ABI stub is on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "canonical_lithe_factory.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL canonical_lithe_factory.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "canonicalV",
    "policy",
    "gV",
    "market",
    "loop",
    "converter",
    "migrator",
    "legacyMarket",
    "legacyV",
    "legacyVSupply",
    "bootstrapV",
    "devFundAllocation",
    "treasuryNet",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL canonical_lithe_factory.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("canonical_lithe_factory.abi.json")',
    "CANONICAL_LITHE_FACTORY_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn canonical_lithe_", "fn deploy_canonical", "fn factory_deploy")
for b in banned:
    if b in lib:
        print("FAIL unexpected CanonicalLitheFactory IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "MINT_AUTHORITY.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "canonical_lithe_factory.abi.json" not in text:
    print("FAIL MINT_AUTHORITY.md missing canonical_lithe_factory.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "CanonicalLitheFactory.sol").read_text(encoding="utf-8")
if "contract CanonicalLitheFactory" not in sol:
    print("FAIL CanonicalLitheFactory.sol missing contract CanonicalLitheFactory")
    sys.exit(1)
for short in ("canonicalV", "migrator", "converter", "legacyMarket", "GENESIS_MINT"):
    if short not in sol:
        print("FAIL CanonicalLitheFactory.sol missing", short)
        sys.exit(1)

print("PASS canonical_lithe_factory ABI stub present; docs needled; no user IPC encoder")
