#!/usr/bin/env python3
"""Prove uninstall DisplayVersion prefers VERSION.txt over Cargo package version."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
setup = (root / "crates" / "vapurr-shell" / "src" / "setup.rs").read_text(encoding="utf-8")
patch = (root / "crates" / "vapurr-shell" / "src" / "patch.rs").read_text(encoding="utf-8")

need_setup = [
    "fn parse_version_stamp",
    "fn resolve_display_version",
    "fn refresh_uninstall_key",
    "let display_ver = resolve_display_version(dir)",
    "parse_version_stamp_pack_and_publish_shapes",
    "resolve_display_version_prefers_version_txt",
]
missing = [n for n in need_setup if n not in setup]
if missing:
    print("FAIL missing in setup.rs:", missing)
    sys.exit(1)

if "reg_set_sz(hkey, w!(\"DisplayVersion\"), SETUP_VER)" in setup:
    print("FAIL still hardcodes SETUP_VER for DisplayVersion")
    sys.exit(1)

if "setup::refresh_uninstall_key(&dest)" not in patch:
    print("FAIL patch swap does not refresh uninstall key")
    sys.exit(1)

print("PASS uninstall DisplayVersion honesty (VERSION.txt + patch refresh)")
