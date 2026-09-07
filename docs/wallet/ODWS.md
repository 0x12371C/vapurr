# Open Digital Wallet Standard (ODWS) v0.1

**Scope:** vapurr device wallet + zer0ID human attestation  
**Issuer:** The Secret Lab  
**Status:** living standard — match code, not marketing

## What this is

ODWS defines how a vapurr wallet is created, stored, locked, and how it ties to **zer0ID** for human-level KYC (browse-earn payout). It is a product standard for this stack — not an industry consortium paper.

## Builds on (accurate attribution)

| Layer | What we use | What we do **not** claim |
|---|---|---|
| **Worldcoin [open-iris](https://github.com/worldcoin/open-iris)** (MIT) | Server-side ocular template / uniqueness for **full KYC** (`trustLevel` 4) | Not OpenAI. Not Orb hardware. Not a medical retina device. Not an official Worldcoin partnership — we consume the open-source pipeline. |
| **OpenAI technology** | Product/agent tooling and related OpenAI tech where the Secret Lab stack integrates it | Not an official OpenAI partnership or joint venture. "Builds on" / uses — not co-branded collab theater. |

OpenAI ≠ open-iris. Name each correctly in UI and press.

## Wallet (custody & safety)

1. **Local-first / non-custodial keys** — secp256k1 device key derived from BIP-39 (12-word) or imported hex; ETH path `m/44'/60'/0'/0/0`.
2. **Encrypted vault** — Windows DPAPI (`wallet.vault`); raw key/seed not left as plaintext on disk after migration.
3. **Passcode** — 4-digit PIN, salted iterated HMAC on disk; never plaintext in web storage. Idle auto-lock default 15 minutes; manual Lock returns to glass lock UI.
4. **Recovery** — user must back up seed when shown at create; screenshot is not a backup.
5. **Honesty** — UI shows only real on-chain / real session state; empty when unpiped.

## zer0ID (human layer)

| Level | Anchor | Notes |
|---|---|---|
| 0–2 | Wallet (+ optional age/jurisdiction claims) | Wallet uniqueness only |
| 3 | Phone (Twilio Verify) | Phone nullifier |
| **4 / Level 4 full KYC** | Ocular template via **open-iris** | UI: https://thesecretlab.app/kyc/scan (iPhone-first). Product copy says **Level 4**. Claims: UniqueHuman, OcularBiometric. |

Browse-earn **claim** requires proven human attestation (**Level 4** / trustLevel 4). install_id binds the machine install separately.

## First create (product gate)

Before `login-create` mints a new wallet, vapurr shows an ODWS acknowledgment:

- This wallet follows ODWS v0.1
- Keys stay on this device (encrypted)
- Full KYC for earn uses ocular scan built on open-iris; tooling may build on OpenAI tech
- User must continue explicitly

## References

- `docs/PASSCODE_LOCK.md`
- `docs/ketpay/EARN_KYC.md`, `docs/zeroid/RHC.md`
- Secret Lab `ZEROID_ISSUER.md` / `docs/OCULAR_L4.md`
