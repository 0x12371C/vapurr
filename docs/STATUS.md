# vapurr status

Last audited: **2026-09-04** (House lock: `docs/ORG_FLASH.md`).

Milestone: **pre-v1** — ship bar is [`V1.md`](V1.md). v1.2 money is **testnet 46630 only**. If README or ARCHITECTURE disagree with this file, this file plus the code win — then those docs get fixed.

## v1.2 (testnet money)

| Gate | State |
|---|---|
| Canonical 46630 market | **Gen-4 live.** Market `0x47Aca529…3617` · V `0xD4b36DDe…7585` · P `0xBe71EF3e…E42e`. Retired `0x447F…` do not count. |
| KetPay settle | `pay.html` signs `wallet-send` `$PUSD` on 46630. Wallet refuses `$PUSD`/`$VAPURR` on 4663. `PayRouter` ignores `eip155:4663`. |
| Postage | `mail_postage` extra.token = canonical testnet `$PUSD` / `$VAPURR`. Still a gasless voucher (no relayer). |
| vapurrbid | Live `$PUSD` pay-to-rank on the testnet book. |
| Ketcharts listing | `$PUSD` pay-to-list. `TESTNET_KETLIST` empty until this device deploys `KetList.sol`. |
| Not v1.2 | Servo, Rain, live zer0ID issuer, mainnet `$PUSD`. |

## v1 progress

| v1 item | State |
|---|---|
| Native Windows window (tao + 4 WebView2s) | Ships |
| Chrome HTML in `frontend/` at `vapurr.localhost` | Ships |
| Packed `dist\vapurr\vapurr.exe` via `pack.ps1` | Ships `dist\vapurr-1.1.0-windows-x64.zip` (**50.6 MB** at 10:28 — Ketflix trailers rust-embedded). **Install vapurr.exe** is the branded first-run (no admin, Start Menu). Logs in `%LOCALAPPDATA%\vapurr`. Next pack: Relic says if mp4 ships. House only. |
| Per-machine `install_id` | Ships — UUID at `%LOCALAPPDATA%\vapurr\install_id`, minted on successful Install (idempotent). Desk/earn payload includes it. See `docs/ketpay/INSTALL_ID.md`. |
| Brand tokens on screen (`frontend/tokens.css`) | Ships (`#c0f800` lime) |
| Tabs, home, settings â†’ `desk.json` | Ships |
| Shield hooked into WebResourceRequested | Ships |
| Scan live (`/scan/api/*`, `vapurr-rhc`) | Ships |
| RHC liquidity graph (`/scan/api/liq`, Scan Liquidity tab) | Ships. Live Robinhood RPC (`vapurr-rhc::liq`). View is capped (â‰¤48 nodes / 72 edges). Factory-log archive crawl was removed â€” it froze the chrome. Lookups use the full RPC book. |
| Live Trenches | Ships â€” rail/home open https://fomo.family in this window |
| PUSD/VAPURR on-chain market (`vapurr-econ` + `PusdMarket.sol`) | Burn `$VAPURR` ↔ mint `$PUSD` at the oracle. Virtual CP spread, min 2%. **Lithe** is 9% on `$PUSD`. No USDG in the mint/burn loop. |
| PUSD vault (`PusdLoop.sol`, Euler-shaped) | Isolated `$PUSD` credit + `$VAPURR` collateral. Boot kink **150%** → **6%** as 100k real `$PUSD` cash lands. Looping does not fade the boot. **Live** `0x89E17eef…4521`. Old `0xC4d4…` retired. |
| House Uni v4 CL (`HouseLp.sol`) | **Live on 46630.** `0x667bFcAF…1bf7`. NFT #2273. `$VAPURR`/`$PUSD` 0.30% ±20%. Swapper `0x6304419b…4dD2` (PUSD settle-safe). Dead `0xb699…` / `0xb10d…` / `0xbD6b…` do not count. |
| Euler vault (`PusdLoop.sol`) | **Live.** `0x89E17eef…4521`. Boot 150% kink, fades with exogenous cash. |
| vapurrbid (`vapurr://vapurrbid`, $PUSD pay-to-rank) | Ships. `Outbid.sol` + `vapurr-econ::outbid`. Rank is $PUSD paid. Aliases: `outbid`, `bid`, `board`. |
| Ketcharts listing (`vapurr://ketcharts` Listed) | Ships. `KetList.sol` + `vapurr-econ::ketlist`. Pay `$PUSD` to list a token (50 min, +25 to take #1). Profile (web/X/tg/discord/bio/logo) is on-chain with the payment. Snap paints Listed + pair card. Inbox copy `%LOCALAPPDATA%\vapurr\ketlist.json`. Never refunded. Organic tape stays. `TESTNET_KETLIST` empty until this device deploys it. |
| PNS (`vapurr://pns`, `.hood` names) | Live on testnet 46630. Registry `0x13C9fCaB70e8f7eED688A5548B0E3849B1ae0fC4` (owns namehash `hood`). ENS-shaped (namehash, addr, reverse, setAddr). Type `alice.hood` in the bar. |
| Swap / Bridge (`vapurr://swap`, `vapurr://bridge`) | Simulate on this device, then you sign and it broadcasts (`wallet-exec`). MAX + balances + impact. `$VAPURR` refund. 4663/46630. |
| Light theme (`data-theme="light"`) | Ships â€” rail + settings. Sage set in `frontend/tokens.css` |
| Bookmarks, history, cookies, Boost, radio | Ships |
| `cargo test --workspace` | Protocol crates have unit tests; keep green |
| 404 / zzzmail / card / wallet / swap / bridge / id as **honest skins** | Product dollar is **$PUSD**. **KetPay** (`vapurr://pay` / `ketpay`) settles `$PUSD` on **testnet 46630 only** — not mainnet 4663. 404 is load-fail. Postage voucher is bound to canonical testnet `$PUSD`. vapurrbid is `$PUSD`. Swap/bridge: simulate, then sign and broadcast. Earn-submit refuses payout without a VerifiedAccount (visits stay queued). `vapurr://id` opens thesecretlab.app/kyc; no fake Proven. See docs/zeroid/RHC.md. |
| `vapurr://id` vs Shield | Split. `id.html` is zer0ID (KYC CTA to Secret Lab). `shield.html` is adblock. Rail is `data-id="shield"` |
| Ketbook (`vapurr://ketbook`) | Ships â€” public product docs (what vapurr is, how it works). Source `ketbook/`. Internal specs stay in `docs/`. |
| WebView2 guest | **Allowed for v1.** Not the product engine. |

## After v1 (do not gold-plate)

- **Servo** as the page engine. `vapurr-engine` feature `servo` `compile_error!`s until libservo is pinned.
- FetcherEngine as the user's browser.
- egui chrome (`vapurr-ui`) â€” unused by the binary.
- Live Rain card. zer0ID issuer (Secret Lab) is required for browse-earn payout; chrome does not fake Proven. No secrets in tree.
- KetPay / `$PUSD` spend on **mainnet 4663**. Testnet 46630 settlement is v1.2. zzzmail postage still a voucher until a relayer posts it.
- One site-process per eTLD+1 with freeze (`vapurr-core` types only).

## Brand tokens that are on screen

Source of truth: `frontend/tokens.css`

| Token | Hex |
|---|---|
| lime | `#c0f800` |
| forest | `#2a3800` |
| void | `#0e0e0e` |
| steel | `#1f2327` |
| snow | `#f2f3f4` |
| muted | `#8aa090` |

`DESIGN.md` / `BRAND.md` must match that table. `vapurr-ui` still has the older `#00F05A` / `#0A2E1B` pair â€” leave it unless you are deleting or rewiring egui (after v1).

Optional light theme (`html[data-theme="light"]`): lime `#4d8a00`, forest `#c8d6b0`, void `#f3f5f0`, steel `#e6ebe0`, snow `#161816`, muted `#3f5340`.

Radio chrome is allowed a private palette (`frontend/radio.css`).

## Contracts

| Item | Bytecode |
|---|---|
| `contracts/PusdMarket.sol` | `crates/vapurr-econ/src/market.hex` — `contracts/compile-market.mjs` |
| `contracts/HouseLp.sol` | `crates/vapurr-econ/src/house.hex` — `contracts/compile-house.mjs` |
| `contracts/PusdLoop.sol` | `crates/vapurr-econ/src/loop.hex` — `contracts/compile-loop.mjs` |
| `contracts/Outbid.sol` | `crates/vapurr-econ/src/outbid.hex` — `contracts/compile-outbid.mjs` |
| `contracts/KetList.sol` | `crates/vapurr-econ/src/ketlist.hex` — `contracts/compile-ketlist.mjs` |
| `contracts/PnsRegistry.sol` | `crates/vapurr-zmail/src/pns.hex` â€” `contracts/compile-pns.mjs` |
| `contracts/MockUsdg.sol` | `crates/vapurr-econ/src/mock_usdg.hex` â€” `contracts/compile-mock.mjs` |

`vapurr-ui` is a workspace member, not linked by the shell. `dist/`, `target/`, crash dumps, and `%LOCALAPPDATA%\vapurr` are not in git.

## RPC / chain (code)

From `crates/vapurr-rhc/src/lib.rs`:

- Chain id `4663` / `eip155:4663`
- RPC `https://rpc.mainnet.chain.robinhood.com`
- Explorer `https://robinhoodchain.blockscout.com`
- USDG `0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168` (6 decimals)
- Native gas: ETH
- Mainnet PusdMarket vanity (not live): `MAINNET_MARKET_VANITY` `0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2`. Deployer `0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5` nonce 0. `VAPURR_MARKET` stays empty until the tx lands.
- Testnet `46630` — econ and KetPay spend here until mainnet has gas. RPC `https://rpc.testnet.chain.robinhood.com`. Canonical gen-4 book: market `0x47Aca5292423e2133A3eE983aB38291de3983617`, `$PUSD` `0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e`, `$VAPURR` `0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585`, vault `0x89E17eefa58B99d025145970c0FBAe7768a14521`, house `0x667bFcAF9D3Ee809336788Bf52511D35AE9C1bf7`, swapper `0x6304419b838Efb12D0Cdf931dd9579c5b4084dD2`. Retired `0x447F…` / `0x435C…` / `0x59bB…` do not count. `TESTNET_OUTBID` / `TESTNET_KETLIST` / mock USDG empty.

## How to run

```
.\run.ps1
```

Or `cargo +stable-x86_64-pc-windows-gnu run -p vapurr-shell --release` (`target\x86_64-pc-windows-gnu\release\vapurr.exe`). `run.ps1` launches via WMI so the process outlives the packing shell.
