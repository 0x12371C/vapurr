#!/usr/bin/env python3
"""Prove PROXY_DEPLOY_GATE live gen-4 board honesty (UUPS vs bare).

Offline (default): docs + vapurr-rhc constants list the UUPS proxies and
the known bare Market/V/PUSD book. --live: runs scripts/verify-proxy.ps1 per
CA (PowerShell RestMethod; urllib often 403s on the RHC RPC) and asserts
UUPS four PASS / bare three FAIL (expected honesty, not a ship gate).
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RHC = ROOT / "crates" / "vapurr-rhc" / "src" / "lib.rs"
GATE = ROOT / "docs" / "econ" / "PROXY_DEPLOY_GATE.md"
PS1 = ROOT / "scripts" / "verify-proxy.ps1"

UUPS = {
    "TESTNET_LOOP": "0x07d1085b545d5e1f55668a6a2EA9332233AaeC69",
    "TESTNET_HOUSE": "0x603AaDFCD483aC196E2bcB158989dD5d38B24336",
    "TESTNET_SWAP": "0x4a00651238EAf8F849b8d8cbb7FD051a4D0f5383",
    "TESTNET_PNS": "0xC0E6f3217525afc80FE89f077504D9E5377a4bB5",
}
BARE = {
    "TESTNET_MARKET": "0x47Aca5292423e2133A3eE983aB38291de3983617",
    "TESTNET_VAPURR": "0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585",
    "TESTNET_PUSD": "0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e",
}


def const_addr(src: str, name: str) -> str:
    m = re.search(rf'pub const {name}: &str = "(0x[0-9A-Fa-f]{{40}})";', src)
    if not m:
        raise SystemExit(f"FAIL rhc missing {name}")
    return m.group(1)


def proxy_ps1(addr: str) -> tuple[int, str]:
    if not PS1.is_file():
        raise SystemExit("FAIL missing scripts/verify-proxy.ps1")
    r = subprocess.run(
        ["powershell", "-NoProfile", "-File", str(PS1), "-Address", addr],
        capture_output=True,
        text=True,
        cwd=str(ROOT),
    )
    out = (r.stdout or "") + (r.stderr or "")
    return r.returncode, out.strip()


def offline() -> None:
    rhc = RHC.read_text(encoding="utf-8")
    gate = GATE.read_text(encoding="utf-8")
    for name, want in {**UUPS, **BARE}.items():
        got = const_addr(rhc, name)
        if got.lower() != want.lower():
            raise SystemExit(f"FAIL {name} rhc={got} board={want}")
        if want not in gate and want.lower() not in gate.lower():
            raise SystemExit(f"FAIL PROXY_DEPLOY_GATE.md missing {name} {want}")
    for needle in [
        "Live gen-4 proxy board",
        "UUPS (impl slot non-zero)",
        "Bare (impl slot zero",
        "TESTNET_LOOP",
        "TESTNET_HOUSE",
        "TESTNET_SWAP",
        "TESTNET_PNS",
        "TESTNET_MARKET",
        "scripts/verify-proxy-book.py",
    ]:
        if needle not in gate:
            raise SystemExit(f"FAIL PROXY_DEPLOY_GATE.md missing needle: {needle}")
    print("PASS offline proxy-book: rhc constants + PROXY_DEPLOY_GATE live board")


def live() -> None:
    offline()
    for name, addr in UUPS.items():
        code, out = proxy_ps1(addr)
        if code != 0 or not out.startswith("PASS:"):
            raise SystemExit(f"FAIL live {name} {addr} expected UUPS PASS\n{out}")
        print(f"PASS live {name}: {out.splitlines()[0]}")
    for name, addr in BARE.items():
        code, out = proxy_ps1(addr)
        if code == 0:
            raise SystemExit(f"FAIL live {name} {addr} expected bare FAIL, got PASS\n{out}")
        if "impl slot is zero" not in out and "bare implementation" not in out.lower():
            raise SystemExit(f"FAIL live {name} {addr} unexpected error\n{out}")
        print(f"PASS live {name} bare (impl slot zero) - known gate debt")
    print("PASS live proxy-book: UUPS four + bare Market/V/PUSD honesty")


def main() -> int:
    if "--live" in sys.argv:
        live()
    else:
        offline()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
