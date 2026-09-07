#!/usr/bin/env python3
"""Prove local ship-version board honesty: Programs == AppData channel == DisplayVersion.

Also reports Cargo.toml + optional public TSL channel skew (warn-only; promote is Relic-gated
until signed pack). Use --live on Windows with an install present (strict: Programs==channel==DisplayVersion).
Use --watch for hourly overnight honesty (reports skew, exits 0 unless install missing).
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TSL_URLS = [
    "https://thesecretlab.app/vapurr/channel/manifest.json",
    "https://www.thesecretlab.app/vapurr/channel/manifest.json",
]


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


def read_manifest_version(path: Path):
    if not path.is_file():
        return None, None
    data = json.loads(path.read_text(encoding="utf-8"))
    return data.get("version"), data.get("sha256")


def cargo_version():
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
    return m.group(1) if m else None


def prove_source() -> None:
    pack = (ROOT / "pack.ps1").read_text(encoding="utf-8", errors="replace")
    need = ["Invoke-VapurrSign", "DisplayVersion", "UNSIGNED"]
    missing = [n for n in need if n not in pack]
    if missing:
        print("FAIL missing pack.ps1 needles:", missing)
        sys.exit(1)
    print("PASS pack.ps1 still owns sign + DisplayVersion path")


def fetch_tsl():
    notes = []
    for url in TSL_URLS:
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "vapurr-verify-version-board"})
            with urllib.request.urlopen(req, timeout=20) as resp:
                body = resp.read().decode("utf-8")
                data = json.loads(body)
                notes.append((url, "ok", data.get("version"), data.get("sha256")))
        except Exception as e:
            notes.append((url, f"FAIL:{type(e).__name__}:{e}", None, None))
    return notes


def prove_live(*, strict: bool = True) -> None:
    """strict=True (--live): FAIL on Programs/channel/DisplayVersion skew.
    strict=False (--watch): report SKEW lines and exit 0 (hourly overnight).
    """
    local = Path(os.environ.get("LOCALAPPDATA", ""))
    prog = local / "Programs" / "vapurr"
    if not prog.is_dir():
        print("SKIP live: Programs install missing")
        return

    prog_ver = None
    txt = prog / "VERSION.txt"
    if txt.is_file():
        prog_ver = parse_version_stamp(txt.read_text(encoding="utf-8", errors="replace"))
    man_ver, prog_sha = read_manifest_version(prog / "manifest.json")
    if not prog_ver:
        prog_ver = man_ver
    if not prog_ver:
        print("FAIL live: Programs has no VERSION/manifest")
        sys.exit(1)

    chan_path = local / "vapurr" / "channel" / "manifest.json"
    chan_ver, chan_sha = read_manifest_version(chan_path)
    if not chan_ver:
        print("FAIL live: AppData channel/manifest.json missing")
        sys.exit(1)

    skews = []
    if chan_ver != prog_ver:
        skews.append(f"Programs {prog_ver} != AppData channel {chan_ver}")
    if prog_sha and chan_sha and prog_sha.lower() != chan_sha.lower():
        skews.append(
            f"Programs sha != channel sha ({prog_sha[:12]} vs {chan_sha[:12]})"
        )

    disp = None
    try:
        import winreg
    except ImportError:
        print("SKIP live DisplayVersion: winreg unavailable")
    else:
        key_path = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\vapurr"
        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path) as k:
                disp, _ = winreg.QueryValueEx(k, "DisplayVersion")
        except FileNotFoundError:
            msg = "Uninstall key missing while Programs present"
            if strict:
                print(f"FAIL live: {msg}")
                sys.exit(1)
            skews.append(msg)
        else:
            if str(disp) != str(prog_ver):
                skews.append(f"DisplayVersion={disp!r} != Programs {prog_ver!r}")

    sha12 = (prog_sha or "")[:12]
    chan12 = (chan_sha or "")[:12]
    if skews:
        for s in skews:
            tag = "FAIL" if strict else "SKEW"
            print(f"{tag} live: {s}")
        print(
            f"NOTE board Programs={prog_ver} sha={sha12} | channel={chan_ver} sha={chan12}"
            + (f" | DisplayVersion={disp}" if disp is not None else "")
        )
        if strict:
            sys.exit(1)
    else:
        print(f"PASS live Programs==channel==DisplayVersion={prog_ver} sha={sha12}")

    cargo = cargo_version()
    print(f"NOTE Cargo.toml version={cargo} (local Programs={prog_ver}; bump is pack/House)")

    profile = local / "vapurr" / "VERSION.txt"
    if profile.is_file():
        prof_ver = parse_version_stamp(profile.read_text(encoding="utf-8", errors="replace"))
        if prof_ver and prof_ver != chan_ver:
            print(f"NOTE AppData root VERSION.txt={prof_ver} (channel={chan_ver})")
        elif prof_ver:
            print(f"NOTE AppData root VERSION.txt={prof_ver} matches channel")

    for url, status, ver, sha in fetch_tsl():
        host = url.split("/")[2]
        if status == "ok":
            skew = "MATCH" if ver == prog_ver else f"SKEW local={prog_ver} tsl={ver}"
            sha12t = (sha or "")[:12]
            print(f"NOTE TSL {host}: {ver} ({skew}) sha={sha12t}")
        else:
            print(f"NOTE TSL {host}: {status}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--live",
        action="store_true",
        help="Strict: Programs/channel/DisplayVersion must match; report TSL/Cargo",
    )
    ap.add_argument(
        "--watch",
        action="store_true",
        help="Hourly honesty: report skew without failing (still FAIL if Programs missing VERSION)",
    )
    args = ap.parse_args()
    prove_source()
    if args.live and args.watch:
        print("FAIL pass only one of --live / --watch")
        sys.exit(1)
    if args.live:
        prove_live(strict=True)
    elif args.watch:
        prove_live(strict=False)


if __name__ == "__main__":
    main()
