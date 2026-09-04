# vapurr status

Last audited: **2026-09-04**.

Milestone: **pre-v1** — ship bar is [`V1.md`](V1.md). If README or ARCHITECTURE disagree with this file, this file plus the code win — then those docs get fixed.

## v1 progress

| v1 item | State |
|---|---|
| Native Windows window (tao + 4 WebView2s) | Ships |
| Chrome HTML in `frontend/` at `vapurr.localhost` | Ships |
| Packed `dist\vapurr\vapurr.exe` via `pack.ps1` | Ships `dist\vapurr-0.1.0-windows-x64.zip`. **Install vapurr.exe** is the branded first-run (no admin, Start Menu). Logs in `%LOCALAPPDATA%\vapurr`. |
| Brand tokens on screen (`frontend/tokens.css`) | Ships (`#c0f800` lime) |
| Tabs, home, settings → `desk.json` | Ships |
| Shield hooked into WebResourceRequested | Ships |
| Scan live (`/scan/api/*`, `vapurr-rhc`) | Ships |
| RHC liquidity graph (`/scan/api/liq`, Scan Liquidity tab) | Ships. Live Robinhood RPC (`vapurr-rhc::liq`). View is capped (≤48 nodes / 72 edges). Factory-log archive crawl was removed — it froze the chrome. Lookups use the full RPC book. |
| Live Trenches | Ships — rail/home open https://fomo.family in this window |
| PUSD/VAPURR on-chain market (`vapurr-econ` + `PusdMarket.sol`) | Burn `$VAPURR` ↔ mint `$PUSD` at the oracle. Virtual CP spread, min 2%. **Lithe** is 9% on `$PUSD`. No USDG in the mint/burn loop. |
| vapurrbid (`vapurr://vapurrbid`, $PUSD pay-to-rank) | Ships. `Outbid.sol` + `vapurr-econ::outbid`. Rank is $PUSD paid. Aliases: `outbid`, `bid`, `board`. |
| PNS (`vapurr://pns`, `.hood` names) | Live on testnet 46630. Registry `0x7eAc2c587Dbb60B2a7f357cfCB28c37c74A6E7d6`. ENS-shaped (namehash, addr, reverse, setAddr). Type `alice.hood` in the bar. |
| Swap / Bridge (`vapurr://swap`, `vapurr://bridge`) | Ships as LI.FI quote chrome (`/route/api/*`). CTA copies the route JSON and opens Wallet — does not settle |
| Light theme (`data-theme="light"`) | Ships — rail + settings. Sage set in `frontend/tokens.css` |
| Bookmarks, history, cookies, Boost, radio | Ships |
| `cargo test --workspace` | Protocol crates have unit tests; keep green |
| 404 / zzzmail / card / wallet / swap / bridge / id as **honest skins** | Product dollar is **$PUSD**. 404 prefers $PUSD on x402 (USDG if that is all they list). zzzmail postage 0.25¢ $PUSD/$VAPURR voucher. vapurrbid is $PUSD. Swap/bridge 25 bps vapurr scoop — quote only. Pay does not settle. `vapurr://id` is parked zer0ID. |
| `vapurr://id` vs Shield | Split. `id.html` is parked identity. `shield.html` is adblock. Rail is `data-id="shield"` |
| Ketbook (`vapurr://ketbook`) | Ships — public product docs (what vapurr is, how it works). Source `ketbook/`. Internal specs stay in `docs/`. |
| WebView2 guest | **Allowed for v1.** Not the product engine. |

## After v1 (do not gold-plate)

- **Servo** as the page engine. `vapurr-engine` feature `servo` `compile_error!`s until libservo is pinned.
- FetcherEngine as the user's browser.
- egui chrome (`vapurr-ui`) — unused by the binary.
- Live Rain card / zer0ID issuer. No secrets in tree.
- x402 settlement from the 404 sheet. zzzmail postage voucher settlement when $PUSD is live.
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

`DESIGN.md` / `BRAND.md` must match that table. `vapurr-ui` still has the older `#00F05A` / `#0A2E1B` pair — leave it unless you are deleting or rewiring egui (after v1).

Optional light theme (`html[data-theme="light"]`): lime `#4d8a00`, forest `#c8d6b0`, void `#f3f5f0`, steel `#e6ebe0`, snow `#161816`, muted `#3f5340`.

Radio chrome is allowed a private palette (`frontend/radio.css`).

## Contracts

| Item | Bytecode |
|---|---|
| `contracts/PusdMarket.sol` | `crates/vapurr-econ/src/market.hex` — `contracts/compile-market.mjs` |
| `contracts/Outbid.sol` | `crates/vapurr-econ/src/outbid.hex` — `contracts/compile-outbid.mjs` |
| `contracts/PnsRegistry.sol` | `crates/vapurr-zmail/src/pns.hex` — `contracts/compile-pns.mjs` |
| `contracts/MockUsdg.sol` | `crates/vapurr-econ/src/mock_usdg.hex` — `contracts/compile-mock.mjs` |

`vapurr-ui` is a workspace member, not linked by the shell. `dist/`, `target/`, crash dumps, and `%LOCALAPPDATA%\vapurr` are not in git.

## RPC / chain (code)

From `crates/vapurr-rhc/src/lib.rs`:

- Chain id `4663` / `eip155:4663`
- RPC `https://rpc.mainnet.chain.robinhood.com`
- Explorer `https://robinhoodchain.blockscout.com`
- USDG `0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168` (6 decimals)
- Native gas: ETH
- Testnet `46630` — econ deploys here until mainnet has gas. RPC `https://rpc.testnet.chain.robinhood.com`. Official Paxos testnet USDG is not mintable; desk deploys `MockUsdg.sol`.

## How to run

```
.\run.ps1
```

Or `cargo +stable-x86_64-pc-windows-gnu run -p vapurr-shell --release` (`target\x86_64-pc-windows-gnu\release\vapurr.exe`). `run.ps1` launches via WMI so the process outlives the packing shell.
