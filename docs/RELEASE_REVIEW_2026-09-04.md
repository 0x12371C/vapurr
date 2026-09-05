# Release review — 2026-09-04

**Recommendation: hold the public release.** The current browser exposes native wallet authority to untrusted web content. This also affects browsing privacy, independently of whether product payments are limited to testnet.

Reviewed the working tree at `5bb5616`, workspace version `1.1.9`, including the existing uncommitted changes. This was a source review with targeted offline reproductions and Rust tests. Application source was not changed.

## Findings

### 1. P0 — Ordinary websites can invoke wallet commands and receive private keys

**Locations:** `crates/vapurr-shell/src/main.rs:244`, `:326`, `:949`, `:973`, `:1011`; `crates/vapurr-wallet/src/desk.rs:198`; `crates/vapurr-wallet/src/session.rs:104`.

The same IPC handler is installed on the browsing webview and chrome webviews. It parses the message body without checking the sender URI. The locally installed wry 0.47.2 implementation injects `window.ipc` into documents and supplies WebView2's message-source URL in the request URI; that provenance is discarded here.

A website can request key/seed export or submit a wallet transaction without going through the confirmation sheet. Wallet dispatch does not require an authenticated session. Export results are evaluated through `window.__setWallet` in the currently open page, with no origin or request binding. The frontend confirmation sheet is therefore bypassable. Testnet restrictions on selected assets do not protect the private key or ETH on supported networks.

**Verified:** an isolated harness used the actual IPC parser and compiled wallet crate with a synthetic key in a temporary profile. Key export succeeded while logged out. No real key was read and no transaction was sent. The browser-to-handler connection was verified against local wry source, not by running an attack in the user's browser.

**Required fix:** enforce exact trusted origins and command permissions at native ingress; require native authorization for signing/export; bind replies to the requesting trusted document and discard them after navigation. Checking the sender alone leaves the asynchronous reply/navigation leak open.

### 2. P0 — External URLs can impersonate chrome and receive private browsing data

**Locations:** `crates/vapurr-shell/src/host/wv.rs:14`; `crates/vapurr-shell/src/main.rs:1178`, `:1205`; `crates/vapurr-shell/src/cookies.rs:9`, `:35`, `:158`.

Trust checks use `url.contains("vapurr.localhost")`. An external URL such as `https://example.invalid/cookies.html?vapurr.localhost` passes. On navigation, the shell injects desk data and, for a URL containing `cookies.html`, the cookie snapshot into the external document. Cookie enumeration covers the shared profile, including HttpOnly cookies; values are included, truncated to 72 characters. The same navigation branch can supply mail data when its URL contains the mail filename.

**Verified:** the exact `is_chrome_url` function accepted both an external query-string example and `https://vapurr.localhost.example.invalid/` in an offline test. The injection and cookie-enumeration paths were traced in application and wry source. This remains a separate leak after fixing IPC ingress.

**Required fix:** use one parsed, exact origin check throughout navigation, injection, tabs, and Shield exemptions. Select chrome surfaces from parsed paths only after establishing trust; bind private-data injection to the intended document.

### 3. P1 — The chrome HTTP API has no caller authorization

**Locations:** `crates/vapurr-shell/src/host/routes.rs:14`; `crates/vapurr-shell/src/host/zzzmail_api.rs:21`, `:78`; `crates/vapurr-shell/src/host/pns.rs:220`.

The host dispatches mail/PNS requests without checking Origin or other caller authorization. Responses explicitly allow every origin and GET/POST/OPTIONS. Sensitive operations include reading the inbox, signing postage, changing PNS addresses, and deploying the registry. PNS mutations do not require POST; the OPTIONS special case is the only method guard.

This creates an independent path around an IPC fix for untrusted content that can reach the chrome host. Browser network restrictions can affect a particular cross-origin reproduction; no end-to-end cross-origin browser test was performed, but the native API itself has no protection.

**Required fix:** authorize callers before dispatch, restrict allowed methods, remove wildcard CORS from private APIs, and require explicit authorization for transactions. Add rejection tests for external origins and GET mutations.

### 4. P1 — Swap/bridge quotes block the native browser event loop

**Locations:** `crates/vapurr-shell/src/main.rs:324`; `crates/vapurr-shell/src/host/routes.rs:125`; `crates/vapurr-rhc/src/route.rs:46`, `:350`.

The shell uses a synchronous custom-protocol callback. A quote request runs `quote_json` directly in that callback. It waits for remote quote requests and RPC simulations; its scoped worker threads are joined before returning. Consequently, slow routers or RPC timeouts block the native UI thread, affecting the window, tabs, and toolbar. Aborting the frontend fetch does not cancel these synchronous Rust operations.

**Required fix:** use asynchronous protocol responses backed by bounded workers, deadlines, and stale-request cancellation. Verify native UI responsiveness with delayed/unavailable RPC responses.

### 5. P1 — An unconfirmed transaction can be displayed as “Paid”

**Locations:** `crates/vapurr-wallet/src/desk.rs:641`, `:660`, `:700`, `:711`; `frontend/pay.html:146`.

After 80 unsuccessful receipt polls, `broadcast` returns `Ok(hash)` anyway. `exec_route` similarly falls through and sets `ok: true`. KetPay treats a new transaction hash as success and completes the sheet titled “Paid.” A transaction still pending, dropped, or later reverted can therefore be presented as settled. Receipt RPC errors take this path too.

**Required fix:** represent submitted, pending, confirmed, and reverted separately. Complete the payment sheet only after a successful receipt; preserve pending hashes for later reconciliation. Test timeout/error cases using a mock RPC.

### 6. P1 — Wallet keys and recovery phrases are stored as plaintext

**Locations:** `crates/vapurr-wallet/src/lib.rs:88`; `crates/vapurr-wallet/src/session.rs:66`, `:70`, `:117`.

`device.sk` contains raw secret bytes and `seed.phrase` contains the recovery words, written with ordinary filesystem writes. Logout only changes session JSON. A readable profile copy or backup exposes the wallet without unlocking it. The existing zeroization of in-memory key types does not protect these files.

**Required fix:** use an encrypted keystore with an explicit unlock policy and a migration that preserves existing keys. Protect recovery material as well as the signing key; handle persistence failures explicitly.

### 7. P2 — The documented workspace test gate currently fails

**Location:** `crates/vapurr-shell/src/main.rs:1315`.

`tests::ketbook_is_chrome` expects `pusd.html?tab=euler` for the Euler/loop aliases, while `nav.rs` returns `pusd.html?tab=oliver`. The frontend accepts both spellings, so this appears to be a stale test expectation rather than a broken page. Nevertheless, `cargo test --workspace` exits with code 101 and stops before later packages run.

**Required fix:** reconcile the test and route documentation with the intended Oliver naming, then rerun the complete workspace gate.

## Validation and limits

- Ran `cargo +stable-x86_64-pc-windows-gnu test --workspace --locked` with temporary LOCALAPPDATA/APPDATA. It compiled and stopped at the shell test failure above.
- Separately ran the remaining Shield, UI, wallet, and zmail package tests with `--no-fail-fast`. Across both runs: **265 passed, 1 failed, 4 ignored**. The ignored tests include live transaction operations and were not enabled. The suite also contains optional live read tests; their passing status is not proof of deployment health.
- The temporary boundary harness passed its two reproduction checks and six existing IPC tests. Here “passed” means the vulnerable behavior was reproduced, not that the boundary is safe.
- Recompiled `PusdLoop.sol` in memory with the installed solc 0.8.24 and the compile helper's settings. Compilation succeeded and the result exactly matches the current `loop.hex` (14,021 bytes). This does not validate contract economics or prove the deployed vault matches; STATUS explicitly records a required redeployment for the cash-cap change.
- No installer was launched, no existing application process was stopped, and no real wallet was used. No clean-machine install, full browser UI exercise, deployed-contract audit, or security dependency audit was performed. Remaining workspace doctests after Cargo's early failure were not all run.

The immediate release gate is fixing findings 1–3 and testing hostile-page isolation. Payment confirmation and UI responsiveness need verification before presenting the wallet/payment flows as ready to ship.
