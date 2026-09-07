#!/usr/bin/env python3
"""Prove ODWS v0.1 + Level 4 KYC steer for browse-earn claims stays honest."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
login = (root / 'frontend' / 'login.html').read_text(encoding='utf-8')
earn = (root / 'frontend' / 'earn.html').read_text(encoding='utf-8')
ident = (root / 'frontend' / 'id.html').read_text(encoding='utf-8')
odws = (root / 'docs' / 'wallet' / 'ODWS.md').read_text(encoding='utf-8')

need_login = [
    'id="odws-box"',
    'id="odws-ack"',
    'id="odws-go"',
    'ODWS v0.1',
    'id="kyc4-go"',
    'Start Level 4 scan',
    'Level 4 unlocks browse-earn claims',
    'needKyc4',
]
missing = [n for n in need_login if n not in login]
if missing:
    print('FAIL login.html missing:', missing)
    sys.exit(1)

need_earn = [
    'id="go-kyc"',
    'Complete Level 4 KYC',
    'Level 4 required to claim browse tokens',
]
missing = [n for n in need_earn if n not in earn]
if missing:
    print('FAIL earn.html missing:', missing)
    sys.exit(1)

need_id = [
    'ODWS',
    'Level 4 required to claim browse-earn',
    'open-iris',
]
missing = [n for n in need_id if n not in ident]
if missing:
    print('FAIL id.html missing:', missing)
    sys.exit(1)

need_doc = [
    'ODWS',
    'v0.1',
    'Level 4',
    'browse-earn',
]
missing = [n for n in need_doc if n not in odws]
if missing:
    print('FAIL docs/wallet/ODWS.md missing:', missing)
    sys.exit(1)

if 'kyc_proven' in earn and 'Level 4 proven' not in earn:
    print('FAIL earn.html kyc_proven branch missing Level 4 proven copy')
    sys.exit(1)

print('PASS ODWS v0.1 + Level 4 KYC steer (login/earn/id + ODWS.md)')
