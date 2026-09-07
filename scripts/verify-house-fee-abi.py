#!/usr/bin/env python3
"""Prove House fee remittance IPC ABI stubs are on disk and still NeedRemittance-gated."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / "crates" / "vapurr-econ" / "src"
lib = (econ / "lib.rs").read_text(encoding="utf-8")

files = {
    "house_fee_remit.abi.json": ["creditFees", "remitSurplus", "setRemittance", "feeReserve"],
    "house_uni_skim.abi.json": ["skimToCredit", "setHook", "feeRemit"],
    "fee_attribution.abi.json": ["credit", "receiveRemittance", "sourceOf", "breakdown"],
    "remittance_sink.abi.json": ["receiveRemittance", "surplus", "accountedRfv", "forwardSurplus"],
}

for name, need in files.items():
    path = econ / name
    if not path.is_file():
        print(f"FAIL missing {path}")
        sys.exit(1)
    abi = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(abi, list):
        print(f"FAIL {name}: expected ABI array")
        sys.exit(1)
    names = {x.get("name") for x in abi if isinstance(x, dict) and x.get("type") == "function"}
    miss = [n for n in need if n not in names]
    if miss:
        print(f"FAIL {name} missing funcs: {miss}")
        sys.exit(1)

need_lib = [
    'include_str!("house_fee_remit.abi.json")',
    'include_str!("house_uni_skim.abi.json")',
    'include_str!("fee_attribution.abi.json")',
    'include_str!("remittance_sink.abi.json")',
    "HOUSE_FEE_REMIT_ABI",
    "HOUSE_UNI_SKIM_ABI",
    "FEE_ATTRIBUTION_ABI",
    "REMITTANCE_SINK_ABI",
    "NeedRemittance",
    "ABI stub on disk",
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print("FAIL lib.rs missing:", miss_lib)
    sys.exit(1)

def between(src: str, start: str, end: str) -> str:
    if start not in src or end not in src:
        raise SystemExit(f"FAIL lib.rs missing region {start} .. {end}")
    return src.split(start, 1)[1].split(end, 1)[0]

fee_body = between(lib, "fn house_fee_remit", "fn remittance_book_snap")
if "encode_fn" in fee_body:
    print("FAIL house_fee_remit appears to encode calldata — keep NeedRemittance until Relic wire")
    sys.exit(1)
if "NeedRemittance" not in fee_body or "Err(EconError::NeedRemittance)" not in fee_body:
    print("FAIL house_fee_remit must stay NeedRemittance-gated")
    sys.exit(1)

wgv = (root / "docs" / "econ" / "WGV_HOUSE.md").read_text(encoding="utf-8")
if "house_fee_remit.abi.json" not in wgv:
    print("FAIL WGV_HOUSE.md missing house_fee_remit.abi.json needle")
    sys.exit(1)

print("PASS house-fee remittance ABI stubs present; remit path still NeedRemittance-gated")
