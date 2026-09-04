# zer0ID on Robinhood Chain (Relic overnight)

## Why this moved up
Browse-earn **payout** requires zer0ID KYC at https://www.thesecretlab.app/kyc.
Therefore live zer0ID is **not** parked theater for the earn/KetPay path — it must be built and integrated on Robinhood Chain (and wired into vapurr).

## Today
- `vapurr-id`: simulator IdentityProvider; `VAPURR_ZEROID_URL` for live base
- `vapurr://id`: parked honest sheet (no issuer form)
- Secret Lab KYC web: https://www.thesecretlab.app/kyc
- Claims model already: AgeOver18, UniqueHuman, SanctionsClear, Jurisdiction — attestation_id + handle only (no PII in vapurr)

## Build target
1. **Issuer**: thesecretlab.app/kyc issues attestation vapurr can verify
2. **On-chain (RHC 4663 / testnet 46630 bootstrap)**: registry or attestation anchor so earn/BrowsePool can check unique-human without trusting only local JSON — align with Secret Lab ZeroIdRegistry patterns where possible (do not invent a second KYC stack)
3. **vapurr-id**: real HTTP client to issuer + verify path; stop using simulator when URL set
4. **Chrome**: `vapurr://id` opens/continues KYC (deep link or embedded flow to thesecretlab.app/kyc), shows handle + proven claims
5. **Earn**: claim/submit requires VerifiedAccount; install_id still binds machine

## Owners
- Pilot: vapurr wiring (id chrome, earn gate, VAPURR_ZEROID_URL)
- House: STATUS/V1 — earn path exception: live zer0ID required for browse-earn payout (other after-v1 stays)
- Secret Lab / ZER0 stack: issuer + registry truth (coordinate; don't fork KYC)

## Honesty
No fake Proven. No PII in vapurr tree. No mainnet secrets in repo.
