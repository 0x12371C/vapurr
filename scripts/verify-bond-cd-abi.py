#!/usr/bin/env python3
"""Prove BondMarket / SpusdCd / SavingsRouter IPC ABI stubs are on disk and gated."""
from pathlib import Path
import json
import sys

root = Path(__file__).resolve().parents[1]
econ = root / 'crates' / 'vapurr-econ' / 'src'
lib = (econ / 'lib.rs').read_text(encoding='utf-8')

files = {
    'bond_market.abi.json': ['bond', 'quote', 'claim', 'availableInventory', 'capacityUtilizationWad'],
    'spusd_cd.abi.json': ['open', 'close', 'previewClose', 'positions', 'availableSurplus'],
    'savings_router.abi.json': ['receiveRemittance', 'setAllocation', 'enabled', 'cd', 'liquid'],
}

for name, need in files.items():
    path = econ / name
    if not path.is_file():
        print(f'FAIL missing {path}')
        sys.exit(1)
    abi = json.loads(path.read_text(encoding='utf-8'))
    if not isinstance(abi, list):
        print(f'FAIL {name}: expected ABI array')
        sys.exit(1)
    names = {x.get('name') for x in abi if isinstance(x, dict) and x.get('type') == 'function'}
    miss = [n for n in need if n not in names]
    if miss:
        print(f'FAIL {name} missing funcs: {miss}')
        sys.exit(1)

need_lib = [
    'include_str!("bond_market.abi.json")',
    'include_str!("spusd_cd.abi.json")',
    'include_str!("savings_router.abi.json")',
    'BOND_MARKET_ABI',
    'SPUSD_CD_ABI',
    'SAVINGS_ROUTER_ABI',
    'NeedBondMarket',
    'NeedSavings',
    'ABI stub on disk',
]
miss_lib = [n for n in need_lib if n not in lib]
if miss_lib:
    print('FAIL lib.rs missing:', miss_lib)
    sys.exit(1)

def between(src: str, start: str, end: str) -> str:
    if start not in src or end not in src:
        raise SystemExit(f'FAIL lib.rs missing region {start} .. {end}')
    return src.split(start, 1)[1].split(end, 1)[0]

bond_body = between(lib, 'fn bond_open', 'fn bonds_book_snap')
cd_body = between(lib, 'fn cd_open', 'fn savings_book_snap')
if 'encode_fn' in bond_body:
    print('FAIL bond_open appears to encode calldata — keep NeedBondMarket until Relic wire')
    sys.exit(1)
if 'encode_fn' in cd_body:
    print('FAIL cd_open appears to encode calldata — keep NeedSavings until Relic wire')
    sys.exit(1)
if 'NeedBondMarket' not in bond_body or 'Err(EconError::NeedBondMarket)' not in bond_body:
    print('FAIL bond_open must stay NeedBondMarket-gated')
    sys.exit(1)
if 'NeedSavings' not in cd_body or 'Err(EconError::NeedSavings)' not in cd_body:
    print('FAIL cd_open must stay NeedSavings-gated')
    sys.exit(1)

bonds = (root / 'docs' / 'econ' / 'BONDS.md').read_text(encoding='utf-8')
spusd = (root / 'docs' / 'econ' / 'SPUSD.md').read_text(encoding='utf-8')
if 'bond_market.abi.json' not in bonds:
    print('FAIL BONDS.md missing bond_market.abi.json needle')
    sys.exit(1)
if 'spusd_cd.abi.json' not in spusd:
    print('FAIL SPUSD.md missing spusd_cd.abi.json needle')
    sys.exit(1)

print('PASS bond/cd/savings ABI stubs present; open paths still Need*-gated')
