# zer0ID on Robinhood Chain (Relic overnight)

## Why this moved up
Browse-earn **payout** requires zer0ID KYC at https://www.thesecretlab.app/kyc.
Therefore live zer0ID is **not** parked theater for the earn/KetPay path — it must be built and integrated on Robinhood Chain (and wired into vapurr).

## Today
- `vapurr-id`: hardened. `verify_attestation`/`load_verified` require an EIP-191 signature (`crates/vapurr-id/src/sig.rs` — k256 + Keccak256) recovering to `VAPURR_ZEROID_ISSUER_ADDRESS`, plus expiry. The old "any non-empty claims list passes" check is gone — that was the sybil hole docs/ketpay/SYBIL.md warns about.
- **Issuer is live** (item 1 below, done): `thesecretlab.app/api/kyc/*` (see that repo's `ZEROID_ISSUER.md`) mints attestations signed with the same scheme. Verified cross-repo: an issuer-minted attestation is accepted by both `vapurr-id::verify_attestation` (Rust) and vapurrDB's `cacheAttestation` (TS) without modification.
  - Levels shipped: 0 wallet-signature, 1 self-attested age, 2 IP-jurisdiction (+ country-level embargo refusal), 3 SMS phone possession (Twilio Verify). None of these are the marketing page's document-scan/liveness/ZK circuit — see that repo's doc for the exact honesty boundary per level.
- `vapurr://id`: still the parked honest sheet — does not call the new issuer yet (see Build target #3).
- Claims model unchanged: AgeOver18, UniqueHuman, SanctionsClear, Jurisdiction — attestation_id + handle only (no PII in vapurr).

## Build target
1. ~~**Issuer**: thesecretlab.app/kyc issues attestation vapurr can verify~~ — done, see Today.
2. **On-chain (RHC 4663 / testnet 46630 bootstrap)**: registry or attestation anchor so earn/BrowsePool can check unique-human without trusting only local JSON — align with Secret Lab ZeroIdRegistry patterns where possible (do not invent a second KYC stack)
3. **vapurr-id**: real HTTP client to the now-live issuer + a chrome flow that actually calls it (wallet-sign the challenge, walk the level ladder, handle the phone OTP two-step) — `Zer0IdProvider::from_env()` still only verifies; nothing in the shell calls `thesecretlab.app/api/kyc/*` yet.
4. **Chrome**: `vapurr://id` opens/continues KYC (deep link or embedded flow to thesecretlab.app/kyc), shows handle + proven claims
5. **Earn**: claim/submit requires VerifiedAccount; install_id still binds machine

## Owners
- Pilot: vapurr wiring (id chrome, earn gate, VAPURR_ZEROID_URL)
- House: STATUS/V1 — earn path exception: live zer0ID required for browse-earn payout (other after-v1 stays)
- Secret Lab / ZER0 stack: issuer + registry truth (coordinate; don't fork KYC)

## Honesty
No fake Proven. No PII in vapurr tree. No mainnet secrets in repo.


## Full KYC (ocular)
Wallet Identity pane opens https://thesecretlab.app/kyc/scan. Builds on Worldcoin open-iris (MIT); tooling may build on OpenAI tech. Not partnership claims. ODWS: docs/wallet/ODWS.md.

**Storage hard lock (Relic, 2026-09-07):** we do not store anyone's personal
info. Proof is ZK; durable state lives only in the vapurrDB-secured backend
as opaque attestations / nullifiers — no raw photos, no PII, no long-lived
biometric templates tied to a wallet. The Secret Lab ocular path
(`lib/zeroid/ocular.js`) was found storing the raw template durably and
indexing it for cross-enrollment near-dup checks; both are fixed — durable
storage now keeps only `{nullifier, engine, enrolledAt}`, and dedup is
exact-nullifier only (see that repo's `ZEROID_ISSUER.md`). UI surfaces
(`login.html`, `earn.html`, `id.html`, `/kyc/scan`) now say this plainly:
we don't store your scan, only a ZK-style attestation.

