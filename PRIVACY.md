# Privacy Policy (vapurr)

**Last updated:** 2026-09-04  
**Product:** vapurr (Windows on-chain browser for Robinhood Chain)  
**Publisher:** The Secret Lab / 0x12371C  
**Contact:** GitHub [@0x12371C](https://github.com/0x12371C) · https://thesecretlab.app/vapurr

This policy describes what vapurr and related Secret Lab services may process when you install or use the software.

## Summary

- vapurr is a local Windows app (WebView2) that browses the web and talks to Robinhood Chain.
- We do **not** sell personal data.
- Some features (browse-earn, device binding, KYC) need identifiers or identity proofs; those are optional unless you use those features.

## Data we may process

### On your device (local)

- Browser profile data normal to WebView2 (cookies, cache, local storage for sites you visit).
- App settings and wallet-related state you create in the app.
- An **install / device binding id** used to bind a machine install for sybil resistance on earn features (not an advertising ID).

### Over the network

- **Public blockchain data** you choose to submit (transactions, addresses). On-chain activity is public by design.
- **Download / update checks** against our distribution endpoints (e.g. thesecretlab.app channel manifests).
- **Optional KYC / identity** for earn claim: if you complete KYC at https://www.thesecretlab.app/kyc (zer0ID), that process is handled by that service and its own notices; vapurr may receive a verification status needed to unlock claim.
- **Crash / load-failure chrome** is local UI (including branded 404 load-fail pages). It is not a payments surface.

### What we do not do

- We do not sell your personal information.
- We do not use your wallet keys for third-party advertising.
- Unsigned builds may be scanned by Windows Defender; that is Microsoft’s local security product, not our telemetry.

## Code signing

Release Windows binaries are intended to be Authenticode-signed. While on the SignPath Foundation open-source program, Windows may show **SignPath Foundation** as the publisher. Signing policy: https://github.com/0x12371C/vapurr/blob/master/docs/SIGNING.md

## Third parties

- **Microsoft WebView2 / Windows** — local OS and browser runtime.
- **Robinhood Chain / RPC providers** — public chain reads and writes you initiate.
- **SignPath Foundation** — code signing of release artifacts (build provenance), not end-user browsing content.
- **thesecretlab.app** — downloads, optional KYC / lab services linked from the product.

## Retention

Local app data stays on your machine until you delete the app or clear profile data. Server-side earn / KYC records follow the retention of those services. On-chain data is permanent.

## Your choices

- You can use vapurr without enabling earn / KYC.
- You can uninstall the app and delete its local profile folders.
- For KYC account questions, use the Secret Lab KYC flow contacts on that site.

## Changes

We may update this file in the public repository. The “Last updated” date above will change when we do.

## Contact

Open an issue on https://github.com/0x12371C/vapurr or contact the maintainer via GitHub @0x12371C.
