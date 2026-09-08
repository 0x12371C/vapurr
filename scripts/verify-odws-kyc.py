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
    'id="kyc-ladder"',
    'id="kyc-phone-open"',
    'id="kyc-iris-open"',
    'id="zkkyc-explain"',
    'zer0ID',
    'open-iris',
    'ZKKYC',
    'nullifier-only',
    'Start open-iris L4 scan',
    'L4 open-iris unlocks browse-earn claims',
    'on-screen mock code',
    'needKyc',
]
missing = [n for n in need_login if n not in login]
if missing:
    print('FAIL login.html missing:', missing)
    sys.exit(1)

# Country / Phone must be live handlers (not polish-only chrome).
need_handlers = [
    'kyc-attest-jurisdiction',
    'id="kyc-geo-go"',
    'kyc-geo-go").onclick',
    'kyc-phone-open").onclick',
    'thesecretlab.app/kyc/phone',
    'cmd: "newtab"',
]
missing = [n for n in need_handlers if n not in login]
if missing:
    print('FAIL login.html KYC handlers missing:', missing)
    sys.exit(1)
if 'vapurr.send({ cmd: "open", url: url })' in login and 'cmd: "newtab", url:' not in login:
    print('FAIL openKycTab still races newtab then delayed open (dead Phone click)')
    sys.exit(1)
if 'function openKycTab' in login and 'cmd: "newtab", url:' not in login:
    print('FAIL openKycTab must send newtab with url in one IPC')
    sys.exit(1)

need_earn = [
    'id="go-kyc"',
    'Complete Level 4 KYC',
    'Level 4 required to claim browse tokens',
    'zer0ID',
    'open-iris',
    'ZKKYC',
    'nullifier-only',
]
missing = [n for n in need_earn if n not in earn]
if missing:
    print('FAIL earn.html missing:', missing)
    sys.exit(1)

need_id = [
    'ODWS',
    'zer0ID',
    'ZKKYC',
    'nullifier',
    'Level 4 is required to claim browse-earn',
    'open-iris',
    '/kyc/phone',
    'id="zkkyc-explain"',
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
    'zer0ID',
    'open-iris',
    'ZKKYC',
    'nullifier',
]
missing = [n for n in need_doc if n not in odws]
if missing:
    print('FAIL docs/wallet/ODWS.md missing:', missing)
    sys.exit(1)

if 'kyc_proven' in earn and 'Level 4 proven' not in earn:
    print('FAIL earn.html kyc_proven branch missing Level 4 proven copy')
    sys.exit(1)

print('PASS ODWS v0.1 + zer0ID/open-iris/ZKKYC + Level 4 KYC steer (login/earn/id + ODWS.md) + Country/Phone handlers')
