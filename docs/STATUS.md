## 2026-09-05 ~17:55 ET — CutoverDeploy prep (no broadcast)

- Tip `fix/gv-spusd-guards` @ `41df999` (worktree `vapurr-lockpad`).
- Focused forge cutover suites **43/43 PASS**. Dry-run `TestnetRollout` exit 0, `CONFIRM=0`, gas ~52.4M, savings disabled in script.
- Bobby `0x875078…` ~0.002 ETH (gas OK at ~0.01 gwei). Path A deployer sk on disk but **0 ETH**. Bobby PK not in env.
- **PREP GO / BROADCAST NO-GO.** Canon: `docs/econ/READY.md`. Relic must confirm + wire key before `CONFIRM_TESTNET_DEPLOY=1`.


# vapurr status

## 2026-09-05 â€” Swap/bridge finance chrome + back stack

`frontend/swap.html` and `frontend/bridge.html` now share `defi-flow` chrome (nav + Back). Finance nav includes Swap/Bridge on every desk. In-page Back walks a session stack of prior finance desks (Oliver/House tabs preserved) and falls back to `vapurr://defi`. Existing `route.js` quote/sign path is unchanged; offline smoke still cannot invent a quote. Earn/wallet stay on their own surfaces.

Validation: `python scripts/verify-defi-ui.py` now also loads swap/bridge (three widths, both themes, nav + back stack + cross links). Globe/WebGPU console noise is ignored. Pack/SHA follows this commit.


## 2026-09-05 â€” DeFi visual flows (packed 1.1.9)

`frontend/defi.html` now has an interactive economic route map with icon-led navigation. Lithe mint/redeem, Oliver credit, and House trade use selectable desks in `pusd.html`; savings/bonds use compact horizon cards and deposit â†’ vest â†’ claim steps. Shared `defi-flow.css` / `defi-flow.js` preserve native transaction handlers, show snapshot-driven LTV headroom, support light/dark themes and reduced motion, and keep detailed mechanics expandable. Display balances use two decimals, including negative net positions; transaction amounts retain their original precision.

Validation: `python scripts/verify-defi-ui.py` runs offline in headless Edge with mocked IPC and snapshots, checking three widths, both themes, navigation, form modes, review/rejection, and the credit meter. Screenshots are in ignored `dist/defi-preview/`. Savings/bond terms still require engine/address-book integration; no contracts were deployed.

Packed with `pack.ps1` at **2026-09-05 20:04:58 UTC**, version **1.1.9**. Verified all five current DeFi HTML/CSS/JS files are embedded byte-for-byte in `dist/vapurr-setup.exe`; the ZIP passes CRC validation and contains the matching installer. Installer SHA-256: `687b6c6da7c3f6e84e4ce1be355c9894032f035af9232d3d74f99ae29fcc6695`. Signing certificate is unset, so this local package is unsigned.

## 2026-09-05 â€” Graphify refresh (local)

Code graph rebuilt after genesis 1.2M / seigniorage / glass lock pad / DeFi visual-flow wave: **2944 nodes / 8226 edges / 116 communities** (94% EXTRACTED, token cost 0) at HEAD `76b0a99`. Commands: `python -m graphify update .` then `python scripts/brand_graph.py`. Pointers: [GRAPHIFY.md](GRAPHIFY.md) (gitignored local map) Â· `graphify-out/` (also gitignored). `EconError` is now a god node (62). No release pack.

Afternoon board: [SNAPSHOT.md](SNAPSHOT.md) Â· tracks: [TRACKS.md](TRACKS.md) Â· graph: [GRAPHIFY.md](GRAPHIFY.md)

Last audited: **2026-09-04** (House lock: `docs/ORG_FLASH.md`).

Milestone: **pre-v1** â€” ship bar is [`V1.md`](V1.md). v1.2 money is **testnet 46630 only**. If README or ARCHITECTURE disagree with this file, this file plus the code win â€” then those docs get fixed.


## 2026-09-05 â€” Genesis allocation HARD LOCK (1.2M)

- **Mint before `setMinter(gV)` = 1,200,000 V**: **1,000,000** launch + **200,000** DevFund (extra; Oliver / $PUSD-only).
- Inside the 1M: BrowserStream **50k** Â· V/ETH **80k** Â· V/NVDA **25k** Â· V/AMD **25k** Â· House wgV/$PUSD **20k** Â· treasury remainder **800k**.
- Treasury 800k is **not** AMM dump: staked gV then Oliver collateral (`GenesisTreasury`, `NoMarketSell`). Legacy converter (~288k gen-4) is **carved from** that 800k so total stays 1.2M.
- Markets: V/ETH + V/NVDA + V/AMD. USDG bond-only. Canon: `docs/econ/GENESIS_ALLOCATION.md`. No broadcast.

## 2026-09-05 â€” Testnet rollout prep (UUPS Lithe / vanity)

- Prep-only: `docs/econ/TESTNET_ROLLOUT.md` ordered gen-5 checklist for **46630** (Fed V, gV, RebasePolicy 1-9%, Lithe behind UUPS ERC1967 at vanity target, Oliver, BondMarket USDG-only, remittance, DevFund 200k->Oliver collateral, V/ETH+V/NVDA+V/AMD, House/wgV follow-up).
- Contracts: `PusdMarketFedUpgradeable` + `proxy/ERC1967Proxy` (UUPS). `script/TestnetRollout.s.sol` dry-run by default; live broadcast gated on `CONFIRM_TESTNET_DEPLOY=1`.
- Vanity `MAINNET_MARKET_VANITY` `0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2` verified = CREATE(STATUS deployer `0x48043E2C...AeA5`, nonce 0). Prefer nonce-0 CREATE of proxy; CREATE2 salt hunt is secondary (`VanityCreate2Hunt.s.sol`).
- **Honest:** gen-4 remains live until approved CutoverDeploy. No silent 46630 broadcast in this prep.
- Seigniorage handoff (docs scrub): Lithe = `marketMinter` (burn/mint); gV = policy 1â€“9% inflate. Checklist = genesis â†’ `setMarketMinter(Lithe)` â†’ `setMinter(gV)`; **no** Lithe redeem inventory fund. Dry-run only until Relic sets `CONFIRM_TESTNET_DEPLOY`.

## 2026-09-05 â€” Launch POL books + DevFundStream (source)

- ExogenousPairRegistry + ExogenousSeedMarket: genesis **V/ETH, V/NVDA, V/AMD** trading/POL books (not bond purchase). Bans USDG / PUSD as exogenous pair legs.
- DevFundStream: genesis **200_000 ** / 4y Sablier-style lockup; unlock slows when 	otalSupply > startSupply. Formula: docs/econ/DEV_FUND.md. Distinct from BrowserStream (50k/3y treasury float).
- CanonicalLitheFactory mints DevFund allocation to initiator before setMinter(gV); LaunchBootstrap registers pairs + funds stream.
- **Source-landed / forge-proven.** Live 46630 unchanged (no silent deploy). UI honest-empty until addresses.

## 2026-09-05 â€” Fed gV / BrowserStream trust wall (slice)

- **USDG lock:** USDG is **Fed treasury bond intake only** (`BondAssetTag`). `$PUSD` peg = social-proof mint-redeem ~par. Retract d1f04a0-era `$PUSD`/USDG pool / peg-depth / cash-depth sketches â€” they hurt `$PUSD` (see `docs/econ/PUSD_LIQUIDITY.md`, `ROUTING.md`, `BONDS.md`). Helper stripped cash-depth UI stub.

- `contracts/GvFed.sol`: `VapurrToken` + **gVAPURR** (index rebase **3.5%/yr**, policy-only) + **wgVAPURR** (wstETH wrapper) + **BrowserStream** (50k/3y earmark, **no mint**) + `RebasePolicy`.
- Foundry proofs: `contracts/test/GvBoundaries.t.sol` â€” annualized ~3.5%, stream drip supply-unchanged, browse cannot rebase, wgV tracks gV across rebase.
- Docs: `docs/econ/HOUSE_PAIR.md` â€” House pairs **wgV/$PUSD**, not raw gV. `ROUTING.md` open choice locked to wgV.

## 2026-09-05 â€” gen-5 Lithe cutover source (not live)

Source-landed successor book (MarketCfg GEN 5). **Live 46630 remains gen-4** until an approved CutoverDeploy. Do not treat gen-4 addresses as the one-token book.

- PusdMarketFed + CanonicalLitheFactory: Fed V seigniorage Lithe (marketMinter); genesis mint funds converter + initiator float/DevFund; dual printers gV+Lithe; then setMinter(gV). Desk snapshot(address) ABI preserved. Oliver collateral = Fed V.
- LegacyVConverter / LitheCutoverMigrator: converter keeps cutover inventory; migration redeemâ†’convertâ†’seigniorage expand â€” **does not mint V** / does not grow canonical V supply.
- Factory deploys V + gV/policy + Lithe + converter + migrator + Oliver only. **Does not** deploy wgV, HousePairConfig, or House (ROUTING: House = wgV/). Remittance / sPUSD / SavingsRouter are **not** auto-wired â€” initiator must setRemittance (and related) post-deploy.
- Proofs: CanonicalVMarket.t.sol + CanonicalLitheFactory.t.sol. Local cutover clears house/pair_config so gen-4 House is not mixed with gen-5 V.

## 2026-09-05 â€” USDG relic lock (docs)

- **USDG** accepted **only** as Fed **treasury bond** asset (`BondAssetTag`: exogenous RFV in -> gV out).
- `$PUSD` stability = social-proof / mint-redeem ~par â€” **not** USDG depth.
- Scrubbed `$PUSD`/USDG pool / peg-depth / cash-depth product plans from `PUSD_LIQUIDITY.md`, `ROUTING.md`, `BONDS.md`, `TRACKS.md`. BondMarket USDG tag stays. No USDG AMM/pool contracts.

## v1.2 (testnet money)

| Gate | State |
|---|---|
| Live 46630 market (gen-4) | **Still gen-4 live** (embedded-V Lithe). Gen-5 cutover is source-only until CutoverDeploy. Market `0x47Aca529â€¦3617` Â· V `0xD4b36DDeâ€¦7585` Â· P `0xBe71EF3eâ€¦E42e`. Retired `0x447Fâ€¦` do not count. |
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
| Packed `dist\vapurr\vapurr.exe` via `pack.ps1` | Ships `dist\vapurr-1.1.1-windows-x64.zip` (**43.3 MB**, git 341b022, no mp4; posters still in embed). **Install vapurr.exe** is the branded first-run (no admin, Start Menu). Logs in `%LOCALAPPDATA%\vapurr`. MP4/WEBM never pack â€” host on thesecretlab.app/vapurr/ (see docs/ketflix/HOSTING.md). House only. |
| Per-machine `install_id` | Ships â€” UUID at `%LOCALAPPDATA%\vapurr\install_id`, minted on successful Install (idempotent). Desk/earn payload includes it. See `docs/ketpay/INSTALL_ID.md`. |
| Brand tokens on screen (`frontend/tokens.css`) | Ships (`#c0f800` lime) |
| Tabs, home, settings â€” `desk.json` | Ships |
| Shield hooked into WebResourceRequested | Ships |
| Scan live (`/scan/api/*`, `vapurr-rhc`) | Ships |
| RHC liquidity graph (`/scan/api/liq`, Scan Liquidity tab) | Ships. Live Robinhood RPC (`vapurr-rhc::liq`). View is capped (â‰¤48 nodes / 72 edges). Factory-log archive crawl was removed â€” it froze the chrome. Lookups use the full RPC book. |
| Live Trenches | Ships â€” rail/home open https://fomo.family in this window |
| PUSD/VAPURR on-chain market (`vapurr-econ` + `PusdMarket.sol`) | **Lithe** is the `$VAPURR` â†” `$PUSD` mint/redeem rail at the oracle. Virtual CP spread, min 2%; its fee reserve supports up to 9% PUSD index drip. No USDG in the mint/burn loop. |
| PUSD vault (`PusdLoop.sol`, Oliver-shaped (Euler-family)) | Isolated `$PUSD` credit + `$VAPURR` collateral. Boot kink **150%** â†’ **6%** as 100k real `$PUSD` cash lands. Looping does not fade the boot. **Live** `0x89E17eefâ€¦4521`. Old `0xC4d4â€¦` retired. |
| House Uni v4 CL (`HouseLp.sol`) | **Live on 46630.** `0x667bFcAFâ€¦1bf7`. NFT #2273. `$VAPURR`/`$PUSD` 0.30% Â±20%. Swapper `0x6304419bâ€¦4dD2` (PUSD settle-safe). Dead `0xb699â€¦` / `0xb10dâ€¦` / `0xbD6bâ€¦` do not count. |
| Oliver vault (`PusdLoop.sol`) | **Live.** `0x89E17eefâ€¦4521`. Boot 150% kink, fades with exogenous cash. |
| vapurrbid (`vapurr://vapurrbid`, $PUSD pay-to-rank) | Ships. `Outbid.sol` + `vapurr-econ::outbid`. Rank is $PUSD paid. Aliases: `outbid`, `bid`, `board`. |
| Ketcharts listing (`vapurr://ketcharts` Listed) | Ships. `KetList.sol` + `vapurr-econ::ketlist`. Pay `$PUSD` to list a token (50 min, +25 to take #1). Profile (web/X/tg/discord/bio/logo) is on-chain with the payment. Snap paints Listed + pair card. Inbox copy `%LOCALAPPDATA%\vapurr\ketlist.json`. Never refunded. Organic tape stays. `TESTNET_KETLIST` empty until this device deploys it. |
| PNS (`vapurr://pns`, `.hood` names) | Live on testnet 46630. Registry `0x13C9fCaB70e8f7eED688A5548B0E3849B1ae0fC4` (owns namehash `hood`). ENS-shaped (namehash, addr, reverse, setAddr). Type `alice.hood` in the bar. |
| Swap / Bridge (`vapurr://swap`, `vapurr://bridge`) | Simulate on this device, then you sign and it broadcasts (`wallet-exec`). MAX + balances + impact. `$VAPURR` refund. 4663/46630. |
| Light theme (`data-theme="light"`) | Ships â€” rail + settings. Sage set in `frontend/tokens.css` |
| Bookmarks, history, cookies, Boost, radio | Ships |
| `cargo test --workspace` | Protocol crates have unit tests; keep green |
| 404 / zzzmail / card / wallet / swap / bridge / id as **honest skins** | Product dollar is **$PUSD**. **KetPay** (`vapurr://pay` / `ketpay`) settles `$PUSD` on **testnet 46630 only** â€” not mainnet 4663. 404 is load-fail. Postage voucher is bound to canonical testnet `$PUSD`. vapurrbid is `$PUSD`. Swap/bridge: simulate, then sign and broadcast. Earn-submit refuses payout without a VerifiedAccount (visits stay queued). `vapurr://id` opens thesecretlab.app/kyc; no fake Proven. See docs/zeroid/RHC.md. |
| `vapurr://id` vs Shield | Split. `id.html` is zer0ID (KYC CTA to Secret Lab). `shield.html` is adblock. Rail is `data-id="shield"` |
| Ketbook (`vapurr://ketbook`) | Ships â€” public product docs (what vapurr is, how it works). Source `ketbook/`. Internal specs stay in `docs/`. |
| WebView2 guest | **Allowed for v1.** Not the product engine. |


## 2026-09-04 evening â€” Oliver honesty (1.1.6)

- On-chain finding (vault `0x89E17eefâ€¦4521`, user `0xe718E24bâ€¦A56f`): debt=0, suppliedâ‰ˆ0.0007 PUSD, wallet PUSDâ‰ˆ0, **collatVâ‰ˆ485846 VAPURR**, vault cashâ‰ˆ0.0007. UI had been rounding/hiding the V deposit behind wallet zeros.
- Pack **1.1.6**: hero shows **net Oliver** (wallet P + supplied âˆ’ debt) + **clear V collateral** line; always-visible Unwind when debt>0; Withdraw P/V when debt==0 with cash-aware max; snap RPC/decode errors surface as red chip (no silent all-0 stub).
- `PusdLoop.loop()` source now caps each step by `min(room, cash)` so synthetic borrow cannot invent depth past cash. **Live vault still needs redeploy** for that on-chain cap; client hex updated via `contracts/compile-loop.mjs` for next deploy.
- Wallet desk decodes `collat_v` / cash / room; prefers unwind copy when debt>0.
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
| `contracts/PusdMarket.sol` | `crates/vapurr-econ/src/market.hex` â€” `contracts/compile-market.mjs` |
| `contracts/HouseLp.sol` | `crates/vapurr-econ/src/house.hex` â€” `contracts/compile-house.mjs` |
| `contracts/PusdLoop.sol` | `crates/vapurr-econ/src/loop.hex` â€” `contracts/compile-loop.mjs` |
| `contracts/Outbid.sol` | `crates/vapurr-econ/src/outbid.hex` â€” `contracts/compile-outbid.mjs` |
| `contracts/KetList.sol` | `crates/vapurr-econ/src/ketlist.hex` â€” `contracts/compile-ketlist.mjs` |
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
- Testnet `46630` â€” econ and KetPay spend here until mainnet has gas. RPC `https://rpc.testnet.chain.robinhood.com`. Live gen-4 book (until CutoverDeploy): market `0x47Aca5292423e2133A3eE983aB38291de3983617`, `$PUSD` `0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e`, `$VAPURR` `0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585`, vault `0x89E17eefa58B99d025145970c0FBAe7768a14521`, house `0x667bFcAF9D3Ee809336788Bf52511D35AE9C1bf7`, swapper `0x6304419b838Efb12D0Cdf931dd9579c5b4084dD2`. Retired `0x447Fâ€¦` / `0x435Câ€¦` / `0x59bBâ€¦` do not count. `TESTNET_OUTBID` / `TESTNET_KETLIST` / mock USDG empty.

## How to run

```
.\run.ps1
```

Or `cargo +stable-x86_64-pc-windows-gnu run -p vapurr-shell --release` (`target\x86_64-pc-windows-gnu\release\vapurr.exe`). `run.ps1` launches via WMI so the process outlives the packing shell.

## 2026-09-05 - savings engine source integration

- Source review starts at a825573 on fix/gv-spusd-guards; Cargo remains 1.1.9. Newest local zip is the September 4 1.1.9 archive and predates the September 5 earnings contracts.
- contracts/SavingsRouter.sol: live-by-default split of one sink's post-floor PUSD surplus to liquid sPUSD and CD coupons (owner setAllocation(false) killswitch). Same-asset checks, sink-only intake, atomic allocation, and empty-liquid-vault gate.
- SpusdCd.sol: entry-time coupon target/fee/maturity, received-balance principal, explicit principal/coupon accounting, proportional underfunding including unmatured targets, and previewClose. Closing cancels unpaid targets; no guaranteed coupon or new mint.
- frontend/bonds.html: examples explicitly labeled targets per term; Bonds/Open CD CTAs live-by-default (capacity/haircut as params).
- Foundry: **102 passed**, including 17 new tests and two 256-case fuzz tests. EarningsEngine.t.sol traces real local branch remittances to both savings legs at a flat V price.
- Rust: **workspace tests passed**, four live tests ignored, using the GNU toolchain, offline dependencies, and an isolated temporary LOCALAPPDATA/APPDATA profile. Existing unused-code/import warnings remain.
- No deployment, mainnet action, pack, or installation. Savings address-book/IPC integration remains open. Review: [econ/STACK_ECON_REVIEW_2026-09-05.md](econ/STACK_ECON_REVIEW_2026-09-05.md).

## 2026-09-05 â€” ungate Bonds / CD / SavingsRouter

- Relic: nothing product-critical stays gated. `frontend/bonds.html` Open Bond / Open CD live-by-default; capacity/oracle/haircut are params.
- `BondMarket` comments + tests ship enabled-with-capacity; `SavingsRouter` enabled by default (owner killswitch).
- Docs: `BONDS.md` scrubbed from gated-until posture to live-with-caps.

