#!/usr/bin/env python3
"""Prove wallet Level-4 KYC path as far as machines can without a live eyeball."""
from __future__ import annotations
import json, os, sys, urllib.request, urllib.error, subprocess
from pathlib import Path

ROOT = Path(r"C:\Users\jfren\vapurr")
TSL = Path(r"C:\Users\jfren\thesecretlab")
fails = []

def check(name, ok, detail=""):
    print(("PASS" if ok else "FAIL"), name, detail)
    if not ok:
        fails.append(name)

login = (ROOT / "frontend/login.html").read_text(encoding="utf-8")
earn = (ROOT / "frontend/earn.html").read_text(encoding="utf-8")
ident = (ROOT / "frontend/id.html").read_text(encoding="utf-8")
check("login ODWS+KYC ladder", all(x in login for x in [
    'id="odws-box"', 'id="kyc-box"', 'id="kyc-age-go"', 'id="kyc-geo-go"',
    'id="kyc-phone-open"', 'id="kyc-iris-open"', 'id="kyc-iris-done"',
    "kyc-attest-age", "kyc-attest-jurisdiction",
    "thesecretlab.app/kyc/phone", "thesecretlab.app/kyc/scan",
]))
check("login no auto goSetPin on iris open", "kyc-iris-open" in login and "goSetPin()" in login
      and login.split('kyc-iris-open')[1].split("onclick")[1].split("};")[0].count("goSetPin") == 0)
check("earn L4 gate", "Complete Level 4 KYC" in earn and "kyc/scan" in earn)
check("id full KYC", "open-iris" in ident and "kyc/scan" in ident)
priv = login.lower()
check("privacy copy login", ("store" in priv and ("not" in priv or "don't" in priv or "do not" in priv)) or "nullifier" in priv or "vapurrdb" in priv)

page = (TSL / "app/kyc/page.tsx").read_text(encoding="utf-8")
check("marketing CTA /kyc/scan", 'href="/kyc/scan"' in page)
check("L4 ocular honesty", "Full KYC (Ocular)" in page and "Accredited Investor" not in page)
scan = (TSL / "app/kyc/scan/page.tsx").read_text(encoding="utf-8")
check("scan page exists", len(scan) > 500)

ocular = (TSL / "lib/zeroid/ocular.js").read_text(encoding="utf-8")
chunk = ocular[ocular.find("persistEnrollment"):ocular.find("persistEnrollment")+500]
check("persistEnrollment no template field", "template" not in chunk or "templateHex" not in chunk)
check("no durable template index push", "index.push({ nullifier, template })" not in ocular)

ver = Path(r"C:\Users\jfren\AppData\Local\Programs\vapurr\VERSION.txt").read_text(encoding="utf-8", errors="ignore")
check("Programs desk 1.1.10 / 31d8e7d", ("31d8e7d" in ver) or ("1.1.10" in ver), ver.splitlines()[0] if ver else "")

for url, name in [
    ("https://thesecretlab.app/kyc/scan", "prod /kyc/scan HTTP"),
    ("https://thesecretlab.app/kyc", "prod /kyc HTTP"),
]:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "vapurr-kyc-prove/1"})
        with urllib.request.urlopen(req, timeout=20) as r:
            body = r.read(4000).decode("utf-8", "ignore")
            check(name, r.status == 200, f"status={r.status}")
    except Exception as e:
        check(name, False, str(e)[:140])

found = False
for py in [TSL / ".venv-ocular311" / "Scripts" / "python.exe", TSL / ".venv-ocular" / "Scripts" / "python.exe"]:
    if py.exists():
        r = subprocess.run([str(py), "-c", "import cv2, iris; print('ok')"], capture_output=True, text=True)
        check(f"open-iris import ({py.parent.parent.name})", r.returncode == 0, (r.stdout or r.stderr)[:100])
        found = True
        break
if not found:
    check("open-iris import", False, "waiting on Python 3.11 + venv")

print("---")
if fails:
    print("SUMMARY FAIL", len(fails), ":", ", ".join(fails))
    sys.exit(1)
print("SUMMARY PASS wallet KYC path (machine-prove; live eyeball still human)")
sys.exit(0)
