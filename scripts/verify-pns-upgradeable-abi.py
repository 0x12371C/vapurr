#!/usr/bin/env python3
"""Prove PnsRegistryUpgradeable ABI stub is wired (ops/zmail; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

fname = "pns_upgradeable_impl.abi.json"
const = "PNS_UPGRADEABLE_IMPL_ABI"
path = econ / fname
if not path.is_file():
    print("FAIL missing", path)
    sys.exit(1)
if ('include_str!("%s")' % fname) not in lib or const not in lib:
    print("FAIL lib.rs missing include/const")
    sys.exit(1)

abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL expected ABI array")
    sys.exit(1)

fns = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function" and x.get("name")}
need = {
    "initialize",
    "proxiableUUID",
    "upgradeToAndCall",
    "pnsVersion",
    "owner",
    "register",
}
missing = sorted(need - fns)
if missing:
    print("FAIL missing fns:", ", ".join(missing))
    sys.exit(1)

doc_paths = [
    root / "docs" / "econ" / "PROXY_DEPLOY_GATE.md",
    root / "docs" / "STATUS.md",
]
if not any(p.is_file() and fname in p.read_text(encoding="utf-8") for p in doc_paths):
    print("FAIL docs needle missing")
    sys.exit(1)

print("PASS %s entries=%d fns=%d const=%s" % (fname, len(abi), len(fns), const))
