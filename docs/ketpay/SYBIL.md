# KetPay / Earn sybil

Relic: **per machine install bound** + **zer0ID KYC for payout**.

## Machine
- `install_id` UUID at `%LOCALAPPDATA%\vapurr\install_id` (minted on Install; see INSTALL_ID.md)
- Earn enrollment keyed by `(install_id, device_key)`
- IP optional secondary only

## Human (payout)
- Must complete zer0ID KYC at https://www.thesecretlab.app/kyc
- Without attestation / VerifiedAccount: pending may queue, **claim pays nothing**
- See docs/zeroid/RHC.md — live issuer + RHC integration required

## Not enough alone
IP, cookies, Edge profile, raw HWID in logs
