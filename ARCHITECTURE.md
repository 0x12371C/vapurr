# vapurr architecture

vapurr is an on-chain browser. Not a Chromium skin. Not Brave with a different orange.

Brave shipped a Chromium fork, an ad-token, and an injected wallet. That is why it still sits at Chrome-class RAM and still feels like a website pretending to be money. vapurr inverts that: the wallet, the mail, and the payment rail are native processes. The web engine is a guest.

**v1 ship bar:** `docs/V1.md`. **Ships vs after-v1:** `docs/STATUS.md`. Do not collapse those columns. Product v1 guests WebView2.

## Brand

Product name: **vapurr** / **VAPURR**. Mark: geometric **cat**. Palette: void + lime. See `BRAND.md` and `DESIGN.md`. On-screen tokens: `frontend/tokens.css`.

## What we are not

- Not the destination engine. WebView2 / wry / tauri / CEF are **guests or scaffolding**. Product engine is Servo.
- Not a dapp browser whose "wallet" is `window.ethereum` in a renderer as the source of truth.
- Not a custodian. Keys stay on device. Avalanche Card is a passthrough, not a deposit we hold.
- Not a KYC database. zer0ID proves claims. vapurr stores an attestation id and a handle.

## Process model (why this is lighter)

Chrome: browser + GPU + network + one renderer per tab (often per subframe) + one process per extension + utility sandboxes. A quiet window is 20–40 processes and multiple GB.

vapurr **target**:

| Process | Count | Owns |
|---|---|---|
| Shell | 1 | native window, wallet, zzzmail, zer0ID, pay router, profile |
| Site | 1 per eTLD+1 | web content (Servo) |
| GPU | 0 or 1 | compositor, shared across sites |

Rules:

- Tabs that share a site share a process.
- Background sites freeze: drop raster, keep a compact DOM/session blob, hard RSS cap.
- `vapurr://` surfaces (wallet, zmail, paywall, identity, **Live Trenches**, scan, …) **never** spawn a site process. They are chrome.
- Extensions, when they exist, are wasm in the shell, not extra renderers.

vapurr **today:** one `tao` window, four wry WebView2s (sidebar, toolbar, page, radio), shared Edge profile under `%LOCALAPPDATA%\vapurr\edge`. Policy types live in `vapurr-core`. The Edge guest is how https works until Servo is pinned.

That is the order-of-magnitude lever. Memory is dominated by renderer count and retained GPU tiles, not by "Rust vs C++". We refuse to pay the Chromium tax on chrome UI.

## Engine

Product engine: **Servo** (`libservo`). Written in Rust, designed to embed.

`FetcherEngine` is a real rustls HTTP + html5ever/scraper reader plus a native 402 paywall. It is not JavaScript and it is not what the window uses for https.

**Product v1** guests Edge via wry/WebView2 so the app actually browses. JS-capable **product** browsing after v1 is Servo, gated on feature `servo`. Enabling that feature without pinning libservo is a compile error on purpose.

Treat Edge as a guest, not the architecture. Do not block v1 on the Servo embed.

## Home chain

Robinhood Chain (`eip155:4663`). Constants: `crates/vapurr-rhc/src/lib.rs`.

- RPC `https://rpc.mainnet.chain.robinhood.com`
- Explorer `https://robinhoodchain.blockscout.com`
- Unit of account: **USDG** `0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168` (6 decimals)
- Gas: ETH
- Account abstraction: ERC-4337 EntryPoint v0.7 / v0.8, **EIP-7702** so an existing address gains batching/session keys/paymaster without migrating

Econ deploys on **testnet 46630** until mainnet has gas (`TESTNET_RPC_HTTP`, faucet `https://faucet.testnet.chain.robinhood.com`). Official Paxos testnet USDG is not mintable; the desk deploys `contracts/MockUsdg.sol` instead. Product v1 is still Robinhood Chain in the chrome — testnet is the bootstrap rail, not a second home chain.

Public RPC only. No Alchemy key required for read. Bundlers/paymasters are optional and behind config.

## Live Trenches

`vapurr://fomo` (rail / home tile labeled **Live Trenches**) opens **https://fomo.family** in this window. Shield does not block fomo.family / fomoapi.io. Ketflix stays a vapurr chrome surface (`vapurr://ketflix`).

## Wallet abstraction

Three account grades:

1. **Guest** — browse. Cannot pay. Cannot send zmail.
2. **LocalUnverified** — device key exists. Can view balances. Cannot hit the card rail. Cannot send zmail.
3. **Verified** — zer0ID attestation. Handle, not hex. Session keys. Card link. Pay and zmail.

Verified UX never shows chain ids, entry points, or raw hashes unless the user opens Advanced.

Session keys: origin-scoped, USDG spend capped, expiring. Dapps get `wallet_sendCalls` / ERC-5792-shaped intents through the shell, not through a renderer-injected script as the source of truth. Injection is compatibility, not authority.

## Identity (zer0ID)

KYC is a prover/verifier protocol. vapurr never stores passport, SSN, selfie, or address.

Attestation claims are boolean/enum only: over-18, unique human, sanctions-clear, jurisdiction code. The shell keeps `attestation_id` + `handle`. Re-prove on demand. Issuer URL is `VAPURR_ZEROID_URL` when live; tests use a local simulator.

v1 chrome: `vapurr://id` is zer0ID (`frontend/id.html`). Start KYC opens thesecretlab.app/kyc. It does not call `vapurr-id` and does not fake Proven. Ad block is `vapurr://shield` (`frontend/shield.html`). Live issuer stays after v1 except earn payout (`docs/zeroid/RHC.md`).

## Payments

HTTP **404** is load-fail (`frontend/404.png`). Money is HTTP **402** / x402. The pay sheet is **KetPay** (`vapurr://pay` / `ketpay`). 404 is not payments.

Flow:

1. Navigation hits a resource.
2. If `402` + `PAYMENT-REQUIRED`, the shell shows a pay sheet: **Pay $X to continue**.
3. `PayRouter` picks a plan:
   - **X402** on Robinhood Chain: **$PUSD** first when the accept list has it on testnet `eip155:46630`. v1.2 does not settle on mainnet `4663`. USDG if that is all they list on 46630. The product dollar is $PUSD; USDG is the chain’s dollar.
   - **Avalanche Card passthrough** for Visa-shaped / fiat merchants: a scoped Rain authorization (merchant, max cents, expiry, single-use). vapurr never sees PAN/CVV. Collateral stays in the user's C-Chain card contract. Native USDC `0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E`
   - **NeedKyc** / **NeedCardLink** if the account is not ready

Passthrough means vapurr is the frontend and policy layer. Settlement is the card or the chain. We do not take custody.

## zzzmail

Mail is a protocol inside the browser, not Gmail in a tab.

- Address: `@name.hood`, `@handle`, or `@0x` (40-hex)
- **PNS** (`vapurr://pns`): TLD `.hood` on testnet 46630. Registry `0x13C9fCaB70e8f7eED688A5548B0E3849B1ae0fC4`. ENS-shaped (namehash, `addr` / `setAddr`, reverse). Type `name.hood` in the bar.
- Seal: X25519 + ChaCha20-Poly1305
- Body: IPFS-shaped CID of ciphertext (veildb-style: encrypt, then pin). Never on-chain.
- Postage: **0.25¢** in `$PUSD` or `$VAPURR` as a **gasless voucher** (0 ETH from the sender). Hard cap **1¢** all-in if a relayer later posts a pointer.
- Pinset: `%LOCALAPPDATA%\vapurr\zzzmail`. Kubo `ipfs add --cid-version=1 --raw-leaves` when present. Optional `VAPURR_MAIL_RELAY`.
- Mailcard: public `.hood` / handle / 0x → X25519.

Device identity is enough to send. zer0ID is not the delivery gate.

## Crate map

```
vapurr-rhc      chain constants, JSON-RPC, Scan APIs, liquidity graph (`liq`), swap/bridge router (score + sim + $VAPURR refund + buy-and-burn remainder)
vapurr-core     tabs, site keys, process policy, profile
vapurr-id       zer0ID
vapurr-wallet   7702 / 4337 / session keys / USDG
vapurr-pay      x402 + card passthrough
vapurr-zmail    encrypted mail + CID pinset + gasless postage
vapurr-blob     local encrypted content-addressed store
vapurr-net      HTTP, 402 hook
vapurr-engine   Engine trait, FetcherEngine, Servo slot
vapurr-ui       leftover egui brand tokens — not linked by the shell
vapurr-fomo     fomo.family feed crate (v1 chrome opens the live site)
vapurr-econ     $VAPURR / $PUSD seigniorage + vapurrbid + PNS on Robinhood Chain
vapurr-shield   adblock-rust (EasyList / EasyPrivacy / uBO)
vapurr-shell    the binary (tao + wry + frontend host)
```

On-chain: `contracts/PusdMarket.sol` — USDG-backed $PUSD at bootstrap, seigniorage after the book is deep. `contracts/Outbid.sol` — vapurrbid ($PUSD pay-to-rank). `contracts/PnsRegistry.sol` — PNS, TLD `.hood`. `contracts/MockUsdg.sol` — mintable 6-dec stand-in on 46630. Device key deploys and signs. CA in `%LOCALAPPDATA%\vapurr\market.json`. Bytecode: `crates/vapurr-econ/src/market.hex` + `outbid.hex` + `mock_usdg.hex`. PNS bytecode: `crates/vapurr-zmail/src/pns.hex`. Compile helpers: `contracts/compile-market.mjs`, `contracts/compile-outbid.mjs`, `contracts/compile-pns.mjs`, `contracts/compile-mock.mjs`. Monte Carlo numbers live in `docs/STATUS.md` (the `sim` module is gone).

Chrome files: `frontend/`. Route table: `docs/SURFACES.md`.

## Security

- Secrets zeroized. Debug/Display must not print keys.
- No key material in the renderer as authority.
- No live Rain/zer0ID credentials in the repo.
- Do not treat this tree as a signer for Relic or crew wallets.
