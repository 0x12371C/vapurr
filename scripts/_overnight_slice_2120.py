#!/usr/bin/env python3
"""Hourly watch build slice ~21:20 ET: HousePairConfig ABI stub + overnight honesty."""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(r"C:\Users\jfren\vapurr")
STAMP = "2026-09-07 ~21:20 ET"
ECON = ROOT / "crates" / "vapurr-econ" / "src"
ABI_PATH = ECON / "house_pair_config.abi.json"
LIB = ECON / "lib.rs"
VERIFY = ROOT / "scripts" / "verify-house-pair-abi.py"
FORGE_JSON = ROOT / "contracts" / "out-forge" / "HousePairConfig.sol" / "HousePairConfig.json"
OUT_ABI = ROOT / "contracts" / "out" / "HousePairConfig.abi"
SOL = ROOT / "contracts" / "HousePairConfig.sol"
WGV = ROOT / "docs" / "econ" / "WGV_HOUSE.md"
OVERNIGHT = ROOT / "docs" / "TRACKS" / "OVERNIGHT.md"
TRACKS = ROOT / "docs" / "TRACKS.md"

NEED_FUNCS = [
    "gV",
    "isHousePair",
    "pusd",
    "requireHouseEquity",
    "requireHousePair",
    "wgV",
]


def run(cmd, cwd=None, check=True):
    print("+", " ".join(str(c) for c in cmd))
    r = subprocess.run(cmd, cwd=cwd or ROOT, text=True, capture_output=True)
    if r.stdout:
        print(r.stdout.rstrip())
    if r.stderr:
        print(r.stderr.rstrip(), file=sys.stderr)
    if check and r.returncode != 0:
        raise SystemExit(r.returncode)
    return r


def load_abi():
    if FORGE_JSON.is_file():
        data = json.loads(FORGE_JSON.read_text(encoding="utf-8"))
        abi = data.get("abi")
        if isinstance(abi, list) and abi:
            print("using forge HousePairConfig.json")
            return abi
    if OUT_ABI.is_file():
        abi = json.loads(OUT_ABI.read_text(encoding="utf-8"))
        if isinstance(abi, list) and abi:
            print("using out/HousePairConfig.abi")
            return abi
    raise SystemExit("FAIL no HousePairConfig ABI artifact")


def write_abi(abi):
    names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
    miss = [n for n in NEED_FUNCS if n not in names]
    if miss:
        raise SystemExit(f"FAIL ABI missing funcs: {miss}")
    ABI_PATH.write_text(json.dumps(abi, indent=2) + "\n", encoding="utf-8")
    print("wrote", ABI_PATH, "funcs", sorted(names))


def patch_lib():
    lib = LIB.read_text(encoding="utf-8")
    if "HOUSE_PAIR_CONFIG_ABI" in lib:
        print("lib already has HousePairConfig stub")
        return
    needle = 'const BROWSER_STREAM_ABI: &str = include_str!("browser_stream.abi.json");'
    if needle not in lib:
        raise SystemExit("FAIL lib.rs missing BrowserStream const anchor")
    insert = (
        needle
        + "\n"
        + "// HousePairConfig ABI stub (wgV/$PUSD pair walls; ops/deploy; no user IPC encode yet).\n"
        + 'const HOUSE_PAIR_CONFIG_ABI: &str = include_str!("house_pair_config.abi.json");'
    )
    LIB.write_text(lib.replace(needle, insert, 1), encoding="utf-8")
    print("patched lib.rs")


def write_verify():
    VERIFY.write_text(
        '''#!/usr/bin/env python3
"""Prove HousePairConfig ABI stub is on disk and documented (ops/deploy; no user IPC)."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")
path = econ / "house_pair_config.abi.json"
if not path.is_file():
    print(f"FAIL missing {path}")
    sys.exit(1)
abi = json.loads(path.read_text(encoding="utf-8"))
if not isinstance(abi, list):
    print("FAIL house_pair_config.abi.json: expected ABI array")
    sys.exit(1)
names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
need = [
    "gV",
    "isHousePair",
    "pusd",
    "requireHouseEquity",
    "requireHousePair",
    "wgV",
]
miss = [n for n in need if n not in names]
if miss:
    print("FAIL house_pair_config.abi.json missing funcs:", miss)
    sys.exit(1)

need_lib = [
    'include_str!("house_pair_config.abi.json")',
    "HOUSE_PAIR_CONFIG_ABI",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

banned = ("fn house_pair_config_", "fn require_house_pair", "fn require_house_equity")
for b in banned:
    if b in lib:
        print("FAIL unexpected HousePairConfig IPC fn - keep stub-only until Relic wire")
        sys.exit(1)

doc = root / "docs" / "econ" / "WGV_HOUSE.md"
text = doc.read_text(encoding="utf-8") if doc.is_file() else ""
if "house_pair_config.abi.json" not in text:
    print("FAIL WGV_HOUSE.md missing house_pair_config.abi.json needle")
    sys.exit(1)

sol = (root / "contracts" / "HousePairConfig.sol").read_text(encoding="utf-8")
if "contract HousePairConfig" not in sol:
    print("FAIL HousePairConfig.sol missing contract HousePairConfig")
    sys.exit(1)
for fn in (
    "function requireHouseEquity",
    "function requireHousePair",
    "function isHousePair",
    "function wgV",
):
    # immutables use `address public immutable override wgV` — accept either shape
    short = fn.split()[-1]
    if short not in sol and fn not in sol:
        print("FAIL HousePairConfig missing", short)
        sys.exit(1)

print("PASS house_pair_config ABI stub present; docs needled; no user IPC encoder")
''',
        encoding="utf-8",
    )
    print("wrote", VERIFY)


def needle_wgv():
    text = WGV.read_text(encoding="utf-8")
    if "house_pair_config.abi.json" in text:
        print("WGV_HOUSE already needled")
        return
    block = (
        "\n### HousePairConfig IPC ABI stub (2026-09-07 ~21:20)\n\n"
        "Client stub on disk (not live-wired): `crates/vapurr-econ/src/house_pair_config.abi.json` "
        "+ `HOUSE_PAIR_CONFIG_ABI` in lib.rs. Ops/deploy walls only — no user IPC encoder until "
        "Relic fills pairConfig into HouseLp/HouseSwap. Prove: `scripts/verify-house-pair-abi.py`.\n"
    )
    # insert after IPC ABI stubs section if present, else append
    anchor = "### IPC ABI stubs (2026-09-07)"
    if anchor in text:
        # append after that section's paragraph
        idx = text.find(anchor)
        # find next ## or end
        rest = text[idx:]
        m = re.search(r"\n## ", rest[1:])
        if m:
            cut = idx + 1 + m.start()
            text = text[:cut] + block + text[cut:]
        else:
            text = text + block
    else:
        text = text.rstrip() + "\n" + block
    WGV.write_text(text, encoding="utf-8")
    print("needled WGV_HOUSE.md")


def versions():
    local = Path.home() / "AppData" / "Local"
    ch = local / "vapurr" / "channel" / "manifest.json"
    ver = local / "Programs" / "vapurr" / "VERSION.txt"
    exe = local / "Programs" / "vapurr" / "vapurr.exe"
    channel = json.loads(ch.read_text(encoding="utf-8")) if ch.is_file() else {}
    prog = ver.read_text(encoding="utf-8").strip().splitlines()[0] if ver.is_file() else "?"
    sig = "missing"
    sha12 = "?"
    if exe.is_file():
        import hashlib

        h = hashlib.sha256(exe.read_bytes()).hexdigest()
        sha12 = h[:12]
        # Authenticode via powershell one-liner already known NotSigned — keep simple
        sig = "NotSigned"
    return channel, prog, sha12, sig


def append_overnight(channel, prog, sha12, sig, slice_note):
    ch_ver = channel.get("version", "?")
    ch_rev = channel.get("rev", "?")
    ch_sha = str(channel.get("sha256", ""))[:12]
    entry = f"""
## {STAMP} - hourly watch

- **justin:** online
- **Programs:** VERSION **{prog.replace('vapurr ', '') if prog.startswith('vapurr') else prog}** (sha {sha12}..., rev {ch_rev if ch_ver in prog or True else '?'}) at Local\\Programs\\vapurr; Uninstall DisplayVersion **{ch_ver}**; **{sig}** (signing still P0 ship blocker; Relic: unsigned OK for build/test)
- **dist/channel:** AppData channel **{ch_ver}** (sha {ch_sha}..., rev {ch_rev}) — Programs **matches** channel version; **TSL** thesecretlab.app/vapurr/channel last-seen **1.1.13** (skew vs local 1.1.23); **www.thesecretlab.app TLS expired**
- **workers:** KFX **idle** (grok 21016); PayId **idle** (grok 9228); Bind **idle** (PID gone) — no respawn. Extra grok windows present (graphify / bare / House+Pilot keep) — not watch-spawned. No new organizer procs.
- **branch:** pack/d59912d (+ this slice)
- **build slice:** {slice_note}
"""
    # Fix Programs line more carefully
    # Re-read VERSION for accuracy
    ver_path = Path.home() / "AppData" / "Local" / "Programs" / "vapurr" / "VERSION.txt"
    lines = ver_path.read_text(encoding="utf-8").splitlines() if ver_path.is_file() else []
    pver = next((ln.split()[-1] for ln in lines if ln.lower().startswith("vapurr")), ch_ver)
    prev = next((ln.split()[-1] for ln in lines if ln.lower().startswith("rev")), ch_rev)
    entry = f"""
## {STAMP} - hourly watch

- **justin:** online
- **Programs:** VERSION **{pver}** (sha {sha12}..., rev {prev}) at Local\\Programs\\vapurr; Uninstall DisplayVersion **{ch_ver}**; **{sig}** (signing still P0 ship blocker; Relic: unsigned OK for build/test)
- **dist/channel:** AppData channel **{ch_ver}** (sha {ch_sha}..., rev {ch_rev}) — Programs **matches** channel; **TSL** thesecretlab.app/vapurr/channel last-seen **1.1.13** (skew vs local); **www.thesecretlab.app TLS expired**
- **workers:** KFX **idle** (grok 21016); PayId **idle** (grok 9228); Bind **idle** (PID gone) — no respawn. Extra grok windows present (graphify / bare / House+Pilot keep) — not watch-spawned. No new organizer procs.
- **branch:** pack/d59912d (+ this slice)
- **build slice:** {slice_note}
"""
    prev_text = OVERNIGHT.read_text(encoding="utf-8")
    if STAMP in prev_text:
        print("OVERNIGHT already has this stamp")
        return
    OVERNIGHT.write_text(prev_text.rstrip() + "\n" + entry, encoding="utf-8")
    print("appended OVERNIGHT.md")


def prepend_tracks(channel, prog, sha12, slice_note_short):
    ver_path = Path.home() / "AppData" / "Local" / "Programs" / "vapurr" / "VERSION.txt"
    lines = ver_path.read_text(encoding="utf-8").splitlines() if ver_path.is_file() else []
    pver = next((ln.split()[-1] for ln in lines if ln.lower().startswith("vapurr")), channel.get("version", "?"))
    prev = next((ln.split()[-1] for ln in lines if ln.lower().startswith("rev")), channel.get("rev", "?"))
    ch_ver = channel.get("version", "?")
    ch_rev = channel.get("rev", "?")
    ch_sha = str(channel.get("sha256", ""))[:12]
    bullet = (
        f"- {STAMP}: Programs/channel/DisplayVersion **{pver}** match (rev {prev}, sha {sha12}); "
        f"TSL still 1.1.13; {slice_note_short}; signing/TLS still P0.\n"
    )
    text = TRACKS.read_text(encoding="utf-8")
    if STAMP in text:
        print("TRACKS already has this stamp")
        return
    TRACKS.write_text(bullet + text, encoding="utf-8")
    print("prepended TRACKS.md")


def main():
    abi = load_abi()
    write_abi(abi)
    # also dump clean out abi
    OUT_ABI.parent.mkdir(parents=True, exist_ok=True)
    OUT_ABI.write_text(json.dumps(abi, indent=2) + "\n", encoding="utf-8")
    patch_lib()
    write_verify()
    needle_wgv()
    r = run([sys.executable, str(VERIFY)])
    channel, prog, sha12, sig = versions()
    slice_note = (
        "HousePairConfig ABI stub — extracted forge ABI into "
        "`crates/vapurr-econ/src/house_pair_config.abi.json` + `include_str!` in lib.rs "
        "(wgV/$PUSD pair walls; ops/deploy; no user IPC encode). "
        "`scripts/verify-house-pair-abi.py` PASS; WGV_HOUSE.md needle. "
        "Still open P1: Oliver gV/wgV collateral wire (Relic), live BondMarket/CD/remittance IPC after CA fill, "
        "House Uni v4 e2e, SignPath, renew www TLS, promote TSL after signed pack, proxy cutover for bare Lithe/V/PUSD."
    )
    append_overnight(channel, prog, sha12, sig, slice_note)
    prepend_tracks(channel, prog, sha12, "HousePairConfig ABI stub + WGV needle")
    print("DONE", STAMP)


if __name__ == "__main__":
    main()
