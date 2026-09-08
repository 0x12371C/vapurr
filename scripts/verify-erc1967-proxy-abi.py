#!/usr/bin/env python3
"""Prove ERC1967Proxy ABI stub is on disk and documented (ops; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

fname = "erc1967_proxy.abi.json"
const = "ERC1967_PROXY_ABI"
path = econ / fname
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL expected ABI array")
    sys.exit(1)

types = {x.get("type") for x in abi if isinstance(x, dict)}
for need in ("constructor", "fallback", "receive", "error", "event"):
    if need not in types:
        print("FAIL abi missing type:", need)
        sys.exit(1)

errs = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "error"}
evts = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "event"}
if "ZeroAddr" not in errs:
    print("FAIL missing ZeroAddr error")
    sys.exit(1)
for e in ("Upgraded", "AdminChanged"):
    if e not in evts:
        print("FAIL missing event", e)
        sys.exit(1)

for n in (f'include_str!("{fname}")', const):
    if n not in lib:
        print("FAIL lib.rs missing:", n)
        sys.exit(1)

banned = ("fn deploy_proxy", "fn upgrade_proxy", "fn proxy_admin", "fn cutover_proxy")
for b in banned:
    if b in lib:
        print("FAIL unexpected proxy IPC fn - keep stub-only until Relic cutover")
        sys.exit(1)

doc = root / "docs" / "econ" / "PROXY_DEPLOY_GATE.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "erc1967_proxy.abi.json" not in text or "ERC1967_PROXY_ABI" not in text:
    print("FAIL PROXY_DEPLOY_GATE.md missing abi needles")
    sys.exit(1)

sol = (root / "contracts" / "proxy" / "ERC1967Proxy.sol").read_text(encoding="utf-8")
for needle in ("contract ERC1967Proxy", "IMPLEMENTATION_SLOT", "fallback()", "receive()"):
    if needle not in sol:
        print("FAIL ERC1967Proxy.sol missing", needle)
        sys.exit(1)

print("PASS erc1967_proxy ABI stub present; docs needled; no user IPC encoder")
