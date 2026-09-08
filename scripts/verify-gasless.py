"""Static honesty checks for vapurr://gasless preview surface.

Proves nav/SURFACES wiring and that FALLBACK + /relay/quote solo math
share fee::EVM_BASE_TX_GAS (25_732), not textbook 21_000.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FRONT = ROOT / "frontend" / "gasless.html"
NAV = ROOT / "crates" / "vapurr-shell" / "src" / "nav.rs"
API = ROOT / "crates" / "vapurr-relay" / "src" / "api.rs"
FEE = ROOT / "crates" / "vapurr-relay" / "src" / "fee.rs"
SURFACES = ROOT / "docs" / "SURFACES.md"


def must(cond: bool, msg: str) -> None:
    if not cond:
        raise AssertionError(msg)


def main() -> None:
    html = FRONT.read_text(encoding="utf-8")
    nav = NAV.read_text(encoding="utf-8")
    api = API.read_text(encoding="utf-8")
    fee = FEE.read_text(encoding="utf-8")
    surfaces = SURFACES.read_text(encoding="utf-8")

    must(FRONT.is_file(), "frontend/gasless.html missing")
    must(re.search(r'"gasless"\s*\|\s*"savings"\s*=>\s*"gasless\.html"', nav), "nav.rs missing gasless|savings route")
    must("`gasless`" in surfaces, "SURFACES.md missing gasless row")
    must("PREVIEW" in html, "gasless.html must stay labeled PREVIEW")
    must("/relay/quote" in html, "gasless.html must fetch /relay/quote")
    must("exampleFirstRequest" in html, "gasless.html must read exampleFirstRequest")
    must("not yet the default" in html.lower(), "must stay honest: not default path")

    m = re.search(r"pub const EVM_BASE_TX_GAS:\s*u64\s*=\s*([0-9_]+)", fee)
    must(m is not None, "EVM_BASE_TX_GAS missing in fee.rs")
    base = int(m.group(1).replace("_", ""))
    must(base == 25_732, f"unexpected EVM_BASE_TX_GAS={base}")

    must("fee::EVM_BASE_TX_GAS" in api, "api.rs quote must use fee::EVM_BASE_TX_GAS")
    must("21_000 + values[0]" not in api, "api.rs still hardcodes textbook 21_000 solo base")

    m2 = re.search(r"soloCostGas:\s*([0-9_]+)\s*\+\s*([0-9_]+)", html)
    must(m2 is not None, "gasless.html FALLBACK.soloCostGas pattern missing")
    a = int(m2.group(1).replace("_", ""))
    b = int(m2.group(2).replace("_", ""))
    must(a == base, f"FALLBACK base {a} != EVM_BASE_TX_GAS {base}")
    must(b == 60_000, f"FALLBACK callGas example expected 60000, got {b}")
    must("* 0.95" in html or "*0.95" in html, "FALLBACK sponsoredFeeGas should mirror default 9500 bps")

    print(f"PASS: gasless wired; quote/FALLBACK solo base={base} (+{b} callGas example)")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAIL:", e)
        sys.exit(1)
