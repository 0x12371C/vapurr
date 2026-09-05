# Release review — 2026-09-04

**Recommendation: hold the public release** until (1) code signing lands and (2) Relic re-reviews the security bar on a signed build. Wallet IPC trust hardening is **in-tree** and package tests for `vapurr-shell` + `vapurr-wallet` are green; that does not replace an independent hostile-page retest or a signed installer.

## Timeline

1. **Source review (Codex):** Working tree around `5bb5616` / workspace `1.1.9`. Found P0/P1 wallet IPC and chrome-trust bugs. Original write-up below under **Original findings (pre-fix)**.
2. **Hardening (Codex):** Applied native trust boundary fixes (`security.rs` / `security.js`, `ipc.rs`, `main.rs`, `keystore.rs`, `session.rs`, `transactions.rs`, host routes). Said fixes compile; usage limit hit mid regression + release-note update.
3. **Finish pass (Relic / Grok Bot, same day):** Module wiring verified; `cargo +stable-x86_64-pc-windows-gnu test -p vapurr-shell -p vapurr-wallet -- --test-threads=1` with isolated `%TEMP%\vapurr-hardening-finish-*` LOCALAPPDATA/APPDATA and MinGW on PATH — **98 passed, 0 failed, 1 ignored** (`desk::tests::live_testnet_snap`). Docs updated; graphify refreshed. No real wallet profile used.

## What landed (in-tree)

| Area | Fix |
|------|-----|
| Chrome URL trust | Exact parsed origin (`http` + host `vapurr.localhost` + port 80); substring/`contains` checks retired for trust |
| IPC / replies | Document binding + guarded `evaluate_script`; native `MessageBox` confirm for signing/export (not HTML sheet alone) |
| Chrome HTTP API | `x-vapurr-client` capability token + origin / `sec-fetch-site` checks (`api_authorized`) |
| Key material | DPAPI `wallet.vault` (`keystore.rs`); migrate off plaintext `device.sk` / `seed.phrase` |
| Payment UX | Receipt-gated confirmation (`transactions.rs` — only successful receipts confirm payment) |
| Stale test | `ketbook_is_chrome` now passes (Oliver naming) |

Modules wired: `vapurr-wallet` `lib.rs` has `mod keystore` + `pub mod transactions`; `vapurr-shell` `main.rs` has `mod security`.

## Still hold — remaining bar

- **Code signing** remains P0 ship blocker (Defender / unsigned `vapurr-setup.exe`). See `docs/TRACKS.md` + `docs/SIGNING.md`.
- **Relic security retest** required: hostile page vs IPC, chrome impersonation URLs, private API cross-origin, unlock/export confirm, vault migration on a real profile copy (not only temp).
- Async protocol / quote UI-thread blocking (original P1 #4) — not claimed fixed here; verify before calling Swap “ready.”
- No clean-machine install, signed pack, or full workspace `--locked` gate claimed in this finish pass (shell+wallet packages only).
- Ignored live tx tests stayed ignored.

## Validation (finish pass)

```
cargo +stable-x86_64-pc-windows-gnu test -p vapurr-shell -p vapurr-wallet -- --test-threads=1
```

Isolated profile under `%TEMP%\vapurr-hardening-finish-*`. PATH included `C:\Users\jfren\winlibs\mingw64\bin`. Result: **shell lib 22**, **shell bin 50**, **wallet 26 + 1 ignored** — all green.

Security unit coverage that ran: `security::tests::chrome_origin_is_exact`, `security::tests::private_api_requires_capability_and_rejects_external_origin`, keystore migration/corrupt-vault tests, `transactions::tests::only_successful_receipts_confirm_payment`.

## Original findings (pre-fix)

Codex’s initial review (application source not yet changed at that moment). Kept for audit trail; treat as **addressed or partially addressed** per table above, not as current open bugs without retest.

### 1. P0 — Ordinary websites can invoke wallet commands and receive private keys

**Locations (then):** `crates/vapurr-shell/src/main.rs`; wallet desk/session export paths.

Same IPC handler on browsing + chrome webviews; sender URI discarded; export via `window.__setWallet` without origin binding.

**Required fix (then):** exact trusted origins + command permissions at native ingress; native authorization for signing/export; bind replies to requesting trusted document.

**Status:** Hardening in-tree; Relic hostile-page retest still required before ship.

### 2. P0 — External URLs can impersonate chrome and receive private browsing data

**Then:** `url.contains("vapurr.localhost")` accepted query-string / subdomain tricks; cookie/mail injection followed.

**Status:** Exact `is_chrome_url` in `security.rs` with rejection tests; retest injection paths on navigation.

### 3. P1 — Chrome HTTP API had no caller authorization

**Then:** Wildcard CORS; mail/PNS without Origin checks.

**Status:** `api_authorized` + worker rejection tests in-tree; Relic retest still required.

### 4. P1 — Swap/bridge quotes block the native browser event loop

Synchronous custom-protocol `quote_json`. **Not claimed fixed** in the Codex hardening set.

### 5. P1 — Unconfirmed transaction displayed as “Paid”

**Status:** Receipt-gated path added (`transactions` tests green); end-to-end KetPay sheet retest still needed.

### 6. P1 — Wallet keys / recovery stored as plaintext

**Status:** DPAPI vault + migration tests green on isolated profile.

### 7. P2 — Workspace test gate / `ketbook_is_chrome`

**Status:** Package tests for shell+wallet pass; Oliver expectation aligned.

## Bottom line

Do **not** ship a public unsigned build. Hardening is committed in-tree pending Relic retest + signed pack. 404 remains load-fail chrome only; KetPay remains 402/x402/$PUSD — never fuse.
