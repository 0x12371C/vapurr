#!/usr/bin/env python3
"""Prove swap/bridge routing visuals stay honest (network strip, idle sim board, desk cross-links)."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
swap = (root / "frontend" / "swap.html").read_text(encoding="utf-8")
bridge = (root / "frontend" / "bridge.html").read_text(encoding="utf-8")
route_js = (root / "frontend" / "route.js").read_text(encoding="utf-8")
route_css = (root / "frontend" / "route.css").read_text(encoding="utf-8")
flow_css = (root / "frontend" / "defi-flow.css").read_text(encoding="utf-8")

for name, html in (("swap.html", swap), ("bridge.html", bridge)):
    need = [
        'class="route-network"',
        'id="route-network"',
        'id="refresh-tokens"',
        'class="route-cross"',
        'id="chip-sim"',
        'id="chip-hops"',
        'id="chip-refund"',
        'id="route"',
        'href="/route.css"',
        'href="/defi-flow.css"',
        'data-go="vapurr://swap"',
        'data-go="vapurr://bridge"',
        'data-go="vapurr://defi"',
    ]
    missing = [n for n in need if n not in html]
    if missing:
        print(f"FAIL {name} missing:", missing)
        sys.exit(1)

if 'data-desk="swap"' not in swap:
    print("FAIL swap.html missing data-desk=swap")
    sys.exit(1)
if 'data-desk="bridge"' not in bridge:
    print("FAIL bridge.html missing data-desk=bridge")
    sys.exit(1)

for needle in [
    "function idleTrace(",
    "function paintIdle(",
    'simChip.textContent = "idle"',
    "sim-board",
    'byId("route-network")',
    'byId("refresh-tokens")',
    'var USDG = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168"',
]:
    if needle not in route_js:
        print("FAIL route.js missing", needle)
        sys.exit(1)

for needle in [".route-network", ".sim-board", ".trace"]:
    if needle not in route_css:
        print("FAIL route.css missing", needle)
        sys.exit(1)

if ".route-cross" not in flow_css:
    print("FAIL defi-flow.css missing .route-cross")
    sys.exit(1)

print("PASS routing visuals: swap+bridge network strip, idle sim board, desk cross-links")
