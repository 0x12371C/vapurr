#!/usr/bin/env python3
"""Prove uninstall DisplayVersion prefers VERSION.txt over Cargo package version.

With --live (Windows): also assert HKCU Uninstall DisplayVersion matches
Programs VERSION.txt / manifest when an install is present.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]


def parse_version_stamp(text: str):
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        rest = line[7:] if line.lower().startswith("vapurr ") else line
        parts = rest.split()
        ver = parts[0].strip() if parts else ""
        if ver and ver[0].isdigit():
            return ver
    return None


def prove_source() -> None:
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

    if 'reg_set_sz(hkey, w!("DisplayVersion"), SETUP_VER)' in setup:
        print("FAIL still hardcodes SETUP_VER for DisplayVersion")
        sys.exit(1)

    if "setup::refresh_uninstall_key(&dest)" not in patch:
        print("FAIL patch swap does not refresh uninstall key")
        sys.exit(1)

    print("PASS uninstall DisplayVersion honesty (VERSION.txt + patch refresh)")


def prove_live() -> None:
    local = Path(os.environ.get("LOCALAPPDATA", ""))
    prog = local / "Programs" / "vapurr"
    if not prog.is_dir():
        print("SKIP live: Programs install missing")
        return

    ver = None
    txt = prog / "VERSION.txt"
    if txt.is_file():
        ver = parse_version_stamp(txt.read_text(encoding="utf-8", errors="replace"))
    if not ver:
        man = prog / "manifest.json"
        if man.is_file():
            try:
                ver = json.loads(man.read_text(encoding="utf-8")).get("version") or None
            except Exception as e:
                print("FAIL live: bad Programs manifest.json:", e)
                sys.exit(1)
    if not ver:
        print("FAIL live: Programs has no VERSION.txt/manifest version")
        sys.exit(1)

    try:
        import winreg
    except ImportError:
        print("SKIP live: winreg unavailable")
        return

    key_path = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\vapurr"
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path) as k:
            disp, _ = winreg.QueryValueEx(k, "DisplayVersion")
    except FileNotFoundError:
        print("FAIL live: Uninstall key missing while Programs present")
        sys.exit(1)

    if str(disp) != str(ver):
        print(f"FAIL live: Uninstall DisplayVersion={disp!r} != Programs {ver!r}")
        sys.exit(1)

    print(f"PASS live DisplayVersion={disp} matches Programs")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--live",
        action="store_true",
        help="Also check HKCU Uninstall DisplayVersion vs Programs VERSION",
    )
    args = ap.parse_args()
    prove_source()
    if args.live:
        prove_live()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
