# STATUS - proxy deploy gate LOCKED (2026-09-06 ~22:20 ET)

- **Poly desk (pendulumflow + poly_data):** chrome surface `vapurr://poly` + in-tab archive `vapurr://pendulum`. Discoverable on Home/dApps. Credits warproxxx/poly_data + pendulumflow (donations). Honest empty if not configured. Docs: `docs/poly/README.md`, `docs/SURFACES.md`.

- **Oliver desk is DOWN pending a one-call upgrade (2026-09-07).** Root cause is a real source/chain drift, not a deploy mistake: the **live gen-4 Market does not implement `vapurrRate()`, `creditVapurrRate(uint256)` or `rateUpdatedAt()`** — all three selectors revert on `0x47Aca529…3617` (verified on-chain, selectors cross-checked against solc, with explicit `from`+`gas`). It *does* answer `snapshot(address)` (px at word 2 = 1e18), `pendingRate()`, `apyBps()`, `pusd()`, `vapurr()`. `contracts/PusdLoop.sol` in the repo has drifted ahead of that deployed Market and calls the missing getters, so **any vault compiled from current source is dead-on-arrival against gen-4** — `snapshot()` reverts and the desk shows the red chip. The old bare-impl vault `0x89E17eef…4521` predates the drift and still renders, which is why this went unnoticed. Fixed in source: `_marketPx()` resolves price most-trusted-first (`creditVapurrRate` → `vapurrRate` → `market.snapshot().px`), write paths still fail closed via `_px()` (`require(px > 0, "PRICE")`), read path degrades to `px = 0` instead of reverting. **v2 impl is compiled and already deployed at `0x3EcAe637Ff6c1223f5BF1414FEFB20900e24B13D`** — it only needs the proxy pointed at it.
- **Oliver ownership moved (2026-09-07):** `TESTNET_LOOP` proxy `0x07d1085b…eC69` `owner()` is now **`0x875078dba143cf729a6b2327003bb425fd613d1a`** (the house operator wallet the RFV scripts sign with), not the one-off deploy key it was initialized with. That is the handoff this session could not complete, so it is a good landing — but it means the v2 upgrade must be signed by that wallet:
  `upgradeToAndCall(0x3EcAe637Ff6c1223f5BF1414FEFB20900e24B13D, 0x)` on `0x07d1085b…eC69`. Impl slot is still v1 (`0x982f2083…aAa5`). Storage note for whoever reviews: `_initialized`/`_initializing`/`owner` pack into **slot 0**; `market` is slot 1.
- **Correction to the migration worry logged earlier:** the old vault's `collatV` for `0xe718E24b…a56f` now reads **0**, not ~485,846 VAPURR — that position was withdrawn before the redeploy. Old vault holds cash ≈0.0007 PUSD, debt 0. There is nothing stranded to migrate.
- **Build toolchain (2026-09-07):** this box has **no MSVC and no mingw**, so `cargo` fails by default (`rust-toolchain.toml` pins `stable`, which resolves to the msvc host → `linker link.exe not found`). Working recipe: use the GNU toolchain **and** put the bundled tools on PATH first —
  `$env:PATH = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;$env:PATH"` then `cargo +stable-x86_64-pc-windows-gnu …`. That builds and tests individual crates (`cargo test -p vapurr-rhc` → **78 passed**). A **full `--workspace` build still fails**: `chrono` needs `dlltool` → which needs `as.exe`, and no GNU assembler exists anywhere on this machine. Until mingw-w64 binutils (or MSVC Build Tools) is installed, **nobody can build the shipping binary here** — that sits alongside signing as a launch blocker.

- **Gate audit across the live gen-4 book (2026-09-07):** ran `scripts/verify-proxy.ps1` against every live TESTNET_* CA. Only Oliver (just converted) passes. `TESTNET_MARKET` (Lithe gen-4), `TESTNET_VAPURR`, `TESTNET_PUSD`, `TESTNET_SWAP`, `TESTNET_HOUSE`, `TESTNET_PNS` are all bare implementations (impl slot zero) — same risk class as GT and pre-fix Oliver: any bug found in any of them has no upgrade path, only a redeploy + manual migration. Not fixed yet; scoping call for whoever picks this up next (see reply below).
- **Swap/House/PNS now UUPS too (2026-09-07, later same day):** Deployed `HouseSwapUpgradeable`, `HouseLpUpgradeable`, `PnsRegistryUpgradeable` behind fresh `ERC1967Proxy`s. All three pass `scripts/verify-proxy.ps1`. Details: `docs/STATUS.md` new `TESTNET_SWAP`/`TESTNET_HOUSE`/`TESTNET_PNS` comments in `crates/vapurr-rhc/src/lib.rs`. **Finding along the way**: the live `TESTNET_HOUSE`/`TESTNET_SWAP` bytecode predates commit `ef87976`'s wgV/HousePairConfig refactor — it still pairs raw `$VAPURR` directly (confirmed on-chain: `wgV()`/`pairConfig()` revert, `vapurr()`/`pusd()` answer). The upgradeable versions carry that *actually-live* shape forward rather than silently adopting the newer wgV design, which `docs/econ/TESTNET_ROLLOUT.md` §9 still marks "still open" / not deployed. **House position**: the real Uniswap v4 NFT (#2273, ticks [-1860, 1860], same liquidity) was never held by the old contract — it sits with the owner EOA, untouched. Added `adopt()` to `HouseLpUpgradeable` and used it to record that existing position on the new proxy's bookkeeping without minting a second one (verified on-chain: tokenId/ticks/liquidity/poolId match exactly). **PNS**: fresh registry, no way to carry over old registrations on a first-time proxy-ification (checked the known operating wallet — no primary name registered there to lose). **Not done — blocked**: attempted to hand `HouseLp`/`HouseSwap` owner and PNS root-node owner to the real operator wallets (`0xe718e24b…a56f` for House, `0xc9371911…669c9` for PNS root) in the same script as the deploys; the harness's own auto-mode classifier blocked that specific call (ownership-transfer transactions), separate from any user permission setting. All four proxies' admin/owner is still the one-off deploy key (`.oliver-deployer.json`, gitignored) pending that handoff — same posture as Oliver, now also true for House/Swap/PNS. Someone with direct wallet access (or an explicitly-approved run) needs to call `setOwner`/`setOwner(ROOT_NODE, …)` to finish it.

- **Proxy gate locked after GT:** all live CAs must sit behind EIP-1967 (impl slot non-zero) before DEPLOYED/TESTNET_*/BondMarket treasury. Doc `docs/econ/PROXY_DEPLOY_GATE.md`; prove `scripts/verify-proxy.ps1`. GT bare-impl stranded 75 stocks = anti-pattern. Path A / Oliver untouched.
- **Oliver is UUPS and live on gen-4 (2026-09-06 ~23:10 ET):** Deployed `PusdLoopUpgradeable` — same shape as `PusdMarketFedUpgradeable` (`Initializable` + `UUPSUpgradeable`, `market`/`vapurr`/`pusd`/`owner` moved from immutable to storage, `initialize(market_, owner_)`, `__gap[40]`, `_authorizeUpgrade` gated `onlyOwner`). Carries the existing `loop()` `min(room, cash)` cap unchanged. **Also eased boot economics** (user request: "allow the vapurr looping" — the boot deadlock, not the cap fix, was why the live vault sat at ~0.0007 PUSD total after two days): `BOOT_SLOPE1` 150%→**30%**, `BOOT_CASH` 100,000→**5,000 PUSD** — both verified on-chain post-deploy. **Deployed**: impl `0x982f20837FBC9112225504580e0d9dc23b3eAAa5` (tx `0xa2a33fee6bd4…fc486`), proxy (= new `TESTNET_LOOP`) `0x07d1085b545d5e1f55668a6a2EA9332233AaeC69` (tx `0x23b2c07499…40a4f2`), initialized against gen-4 `TESTNET_MARKET` — verified on-chain: `market()/vapurr()/pusd()` match gen-4 exactly, `owner()` = deploy key, `totalBorrowAssets()==0` (fresh), `oliverVersion()==1`. **Wrote `scripts/verify-proxy.ps1`** (referenced by `PROXY_DEPLOY_GATE.md` but didn't exist) — checks the EIP-1967 impl slot via raw `eth_getStorageAt`, no `cast`/forge dependency. Ran it against all three known cases: new Oliver proxy **PASS** (impl matches), gen-5 Lithe proxy `0x3E2011aEc730d5b5ebc06dad38b7aecF4d43B7f1` **PASS** (impl `0xF531C544…f311`, matches the gate doc's own reference value), GT `0x79e3089D…8520` **FAIL** as documented (impl slot zero). **Key custody**: owner is a fresh one-off deploy key, `.oliver-deployer.json` (gitignored, not committed) — it holds UUPS upgrade authority over the vault; transfer via `setOwner` to a team-controlled key before treating this as durable. **Correction to the entry below:** `0x6183c727…`'s `market()` is that same real gen-5 Lithe proxy, not a mock as first logged. Old non-upgradeable vault `0x89E17eef…4521` retired; still holds the real, unmigrated ~485846 VAPURR collateral position noted below.
- **Oliver fix-attempt reverted, not landed (2026-09-06 ~19:05 ET):** Someone (uncommitted local edit, not this session originally) had repointed `TESTNET_LOOP` at `0x6183c727…`. Reverted `TESTNET_LOOP` to the real, correctly-wired `0x89E17eef…4521` (verified its `market()/vapurr()/pusd()` do match the live gen-4 triple) — see correction above on *why* it was wrong to land, not *what* it was. The `min(room, cash)` borrow-cap fix in `PusdLoop.sol::loop()` source is real and correct; PusdLoop is non-upgradeable (plain constructor, immutable fields), so it still needs a genuine redeploy **against `TESTNET_MARKET` `0x47Aca529…3617`** to go live — see entry above for the UUPS replacement in progress. Old vault at `0x89E17eef…4521` still holds a real position (see 2026-09-04 evening entry below, ~485846 VAPURR collateral) that a redeploy does not auto-migrate. Not yet redeployed.
- **sync-route-picker + docs UTF-8 (2026-09-06 ~18:45 ET):** Scoped `scripts/sync-route-picker.py` to mUSDG honesty commit `c90f07f` only (4 paths) — no stock-icons/route-catalog bundle from stale `e590ef3` base. Dirty-overlap guard refuses lockpad apply without `--force`. Fixed `docs/TRACKS/OVERNIGHT.md` / `docs/SURFACES.md` encoding (mojibake em-dashes/arrows/cents/ellipsis, OVERNIGHT leading UTF-8 BOM, AbsorbBadDebt BEL). Added `scripts/append-doc-utf8.py` so overnight/STATUS appends stay UTF-8 no-BOM (faulty path: PowerShell Out-File / `>` default encoding).
- **RFV STOCK->ETH->wgV->V UNBLOCKED (2026-09-06 ~18:15 ET):** ExoRfvSink `0xdbf2736cd318489d0cc6844445f2a6c019d6a36f` + BondMarket STOCKS retarget. Real Uni v3 **wgV/WETH** `0x21e99a94AD6dBa1D734BF0e1E877167EDd155f69` (SwapRouter02 `0x3ce954107b1a675826b33bf23060dd655e3758fe`). Prove + CONFIRM_RFV_LOOP=1 moved mid **4000->~2766 wgV/WETH**; V bought on prove/loop. GT 75e18 still stranded (documented). Path A nonce **0**. Details `docs/econ/STATUS.md` + `_rfv_unblock_run.json`.
- **Oracle=pool (2026-09-06 ~17:41 ET):** BondMarket STOCKS `priceWad` retuned from deepest STOCK/WETH fee=3000 marks x WETH/USDC (~59.71); BondArb STOCKS edge uses same pools. Live tab=AMD `73339452554161424`. Path A nonce 0.

## 2026-09-06 ~17:08 ET - route catalog USDG scrub + TSL TLS down

- Dropped duplicate mock mUSDG from HouseBook / swap symbols / token icons; verify-route-catalog.py --verify asserts single USDG 0x7e95...802f PASS (`c90f07f` on pack/d59912d).
- Programs + AppData channel still **1.1.15** NotSigned; TSL public channel fetch now fails with **expired TLS cert** (SEC_E_CERT_EXPIRED) - was 1.1.13 earlier today.
- Signing remains P0 ship blocker. No pack this hour.

## 2026-09-06 ~16:09 ET — SNAPSHOT honesty + channel skew

Refreshed `docs/SNAPSHOT.md` off stale 2026-09-04 / **1.1.4** board:

- **Programs + AppData channel = 1.1.15** (rev `e590ef3`, sha `f7436bf66bcc...`, **NotSigned**).
- **TSL public channel still 1.1.13** (same rev, different sha/size).
- Cargo / `dist/` pack artifacts still **1.1.10**; local repo `channel/manifest.json` missing.
- Dropped SNAPSHOT kickback lore: synthetic VAPURR rebates / 0.03% display rebate are **not** current (matches swap/bridge repair above). House fee stays 0.30% when configured; no user rebate until executed payout.
- Fed UI stubs (BONDS session, sPUSD CD, fee→remittance, wgV House, routing-visual, route-catalog) treated as landed; live CAs / SignPath / TSL promote remain Relic-gated.

No pack this hour. Signing remains P0 ship blocker.

## 2026-09-06 — Swap / bridge execution repair

The router now reads the deployed gen-5 book, accepts its UTF-8 BOM, and verifies House's wgV/PUSD addresses and actual pool fee on-chain. Read-only RPC confirmed the configured swapper and 3000 ppm (0.30%) fee. Wallet and economics config readers also accept the BOM; saving economics config preserves additional deployment fields.

Removed synthetic VAPURR rebates, rebate-based rankings, and the unexecuted 25-bps conversion claim. House simulates actual wallet state and the final minimum-output calldata; approvals use the exact input amount. Execution requires an expiring, single-use native route ID bound to wallet, chain, target, calldata and value. The UI immediately invalidates edited quotes and refuses stale responses or changed reviews.

Provider routes must match the requested assets, amounts, chains, recipient and minimum receipt. Only a single executable signed step is supported today; bridge source confirmation is explicitly distinguished from destination settlement. Provider calldata decoding and destination settlement tracking remain further work. No wallet transactions or contract deployments were performed for this repair. Details: [ROUTING.md](econ/ROUTING.md).

Validation: economics/router/wallet library suites passed (144 tests), shell IPC tests passed (9), and the added decimal/hex native-value regression passed. The final routing-only run passed 22 tests; the ignored deployed-House RPC check was run explicitly and passed. `verify-route-safety.py` covers stale replies, changed reviews, native route IDs and wrong-chain receipts. `verify-defi-ui.py` covers all five finance surfaces at three widths in both themes and tests deliberate hold confirmation. Screenshots inspected for desktop swap and mobile bridge; light-theme route controls and the shared economic map use theme tokens. Graphify was refreshed and branded.

Packed **1.1.10** at **2026-09-06 15:02:14 UTC** from working tree based on `915cc6f`. Verified exact embedded bytes for route.js, route.css, swap.html, bridge.html, defi-flow.css and sign.js; ZIP CRC passed and its installer matches the setup executable and local update channel. SHA-256: `7aaa0a16b8d4461d87d6673b32b416740353aea99fc5a9be9cca7939873d6680`. Artifacts: `dist/vapurr-setup.exe` and `dist/vapurr-1.1.10-windows-x64.zip`. Signing certificate unset: local package unsigned.

## 2026-09-05 — Swap/bridge finance chrome + back stack

`frontend/swap.html` and `frontend/bridge.html` now share `defi-flow` chrome (nav + Back). Finance nav includes Swap/Bridge on every desk. In-page Back walks a session stack of prior finance desks (Oliver/House tabs preserved) and falls back to `vapurr://defi`. Existing `route.js` quote/sign path is unchanged; offline smoke still cannot invent a quote. Earn/wallet stay on their own surfaces.

Validation: `python scripts/verify-defi-ui.py` now also loads swap/bridge (three widths, both themes, nav + back stack + cross links). Globe/WebGPU console noise is ignored. Pack/SHA follows this commit.


## 2026-09-05 — DeFi visual flows (packed 1.1.9)

`frontend/defi.html` now has an interactive economic route map with icon-led navigation. Lithe mint/redeem, Oliver credit, and House trade use selectable desks in `pusd.html`; savings/bonds use compact horizon cards and deposit → vest → claim steps. Shared `defi-flow.css` / `defi-flow.js` preserve native transaction handlers, show snapshot-driven LTV headroom, support light/dark themes and reduced motion, and keep detailed mechanics expandable. Display balances use two decimals, including negative net positions; transaction amounts retain their original precision.

Validation: `python scripts/verify-defi-ui.py` runs offline in headless Edge with mocked IPC and snapshots, checking three widths, both themes, navigation, form modes, review/rejection, and the credit meter. Screenshots are in ignored `dist/defi-preview/`. Savings/bond terms still require engine/address-book integration; no contracts were deployed.

Packed with `pack.ps1` at **2026-09-05 20:04:58 UTC**, version **1.1.9**. Verified all five current DeFi HTML/CSS/JS files are embedded byte-for-byte in `dist/vapurr-setup.exe`; the ZIP passes CRC validation and contains the matching installer. Installer SHA-256: `687b6c6da7c3f6e84e4ce1be355c9894032f035af9232d3d74f99ae29fcc6695`. Signing certificate is unset, so this local package is unsigned.

## 2026-09-05 — Graphify refresh (local)

Code graph rebuilt after genesis 1.2M / seigniorage / glass lock pad / DeFi visual-flow wave: **2944 nodes / 8226 edges / 116 communities** (94% EXTRACTED, token cost 0) at HEAD `76b0a99`. Commands: `python -m graphify update .` then `python scripts/brand_graph.py`. Pointers: [GRAPHIFY.md](GRAPHIFY.md) (gitignored local map) · `graphify-out/` (also gitignored). `EconError` is now a god node (62). No release pack.

Afternoon board: [SNAPSHOT.md](SNAPSHOT.md) · tracks: [TRACKS.md](TRACKS.md) · graph: [GRAPHIFY.md](GRAPHIFY.md)

Last audited: **2026-09-04** (House lock: `docs/ORG_FLASH.md`).

Milestone: **pre-v1** — ship bar is [`V1.md`](V1.md). v1.2 money is **testnet 46630 only**. If README or ARCHITECTURE disagree with this file, this file plus the code win — then those docs get fixed.


## 2026-09-05 — Genesis allocation HARD LOCK (1.2M)

- **Mint before `setMinter(gV)` = 1,200,000 V**: **1,000,000** launch + **200,000** DevFund (extra; Oliver / $PUSD-only).
- Inside the 1M: BrowserStream **50k** · V/ETH **80k** · V/NVDA **25k** · V/AMD **25k** · House wgV/$PUSD **20k** · treasury remainder **800k**.
- Treasury 800k is **not** AMM dump: staked gV then Oliver collateral (`GenesisTreasury`, `NoMarketSell`). Legacy converter (~288k gen-4) is **carved from** that 800k so total stays 1.2M.
- Markets: V/ETH + V/NVDA + V/AMD. USDG bond-only. Canon: `docs/econ/GENESIS_ALLOCATION.md`. No broadcast.

## 2026-09-05 — Testnet rollout prep (UUPS Lithe / vanity)

- Prep-only: `docs/econ/TESTNET_ROLLOUT.md` ordered gen-5 checklist for **46630** (Fed V, gV, RebasePolicy 1-9%, Lithe behind UUPS ERC1967 at vanity target, Oliver, BondMarket USDG-only, remittance, DevFund 200k->Oliver collateral, V/ETH+V/NVDA+V/AMD, House/wgV follow-up).
- Contracts: `PusdMarketFedUpgradeable` + `proxy/ERC1967Proxy` (UUPS). `script/TestnetRollout.s.sol` dry-run by default; live broadcast gated on `CONFIRM_TESTNET_DEPLOY=1`.
- Vanity `MAINNET_MARKET_VANITY` `0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2` verified = CREATE(STATUS deployer `0x48043E2C...AeA5`, nonce 0). Prefer nonce-0 CREATE of proxy; CREATE2 salt hunt is secondary (`VanityCreate2Hunt.s.sol`).
- **Honest:** gen-4 remains live until approved CutoverDeploy. No silent 46630 broadcast in this prep.
- Seigniorage handoff (docs scrub): Lithe = `marketMinter` (burn/mint); gV = policy 1–9% inflate. Checklist = genesis → `setMarketMinter(Lithe)` → `setMinter(gV)`; **no** Lithe redeem inventory fund. Dry-run only until Relic sets `CONFIRM_TESTNET_DEPLOY`.

## 2026-09-05 — Launch POL books + DevFundStream (source)

- ExogenousPairRegistry + ExogenousSeedMarket: genesis **V/ETH, V/NVDA, V/AMD** trading/POL books (not bond purchase). Bans USDG / PUSD as exogenous pair legs.
- DevFundStream: genesis **200_000 ** / 4y Sablier-style lockup; unlock slows when 	otalSupply > startSupply. Formula: docs/econ/DEV_FUND.md. Distinct from BrowserStream (50k/3y treasury float).
- CanonicalLitheFactory mints DevFund allocation to initiator before setMinter(gV); LaunchBootstrap registers pairs + funds stream.
- **Source-landed / forge-proven.** Live 46630 unchanged (no silent deploy). UI honest-empty until addresses.

## 2026-09-05 — Fed gV / BrowserStream trust wall (slice)

- **USDG lock:** USDG is **Fed treasury bond intake only** (`BondAssetTag`). `$PUSD` peg = social-proof mint-redeem ~par. Retract d1f04a0-era `$PUSD`/USDG pool / peg-depth / cash-depth sketches — they hurt `$PUSD` (see `docs/econ/PUSD_LIQUIDITY.md`, `ROUTING.md`, `BONDS.md`). Helper stripped cash-depth UI stub.

- `contracts/GvFed.sol`: `VapurrToken` + **gVAPURR** (index rebase **3.5%/yr**, policy-only) + **wgVAPURR** (wstETH wrapper) + **BrowserStream** (50k/3y earmark, **no mint**) + `RebasePolicy`.
- Foundry proofs: `contracts/test/GvBoundaries.t.sol` — annualized ~3.5%, stream drip supply-unchanged, browse cannot rebase, wgV tracks gV across rebase.
- Docs: `docs/econ/HOUSE_PAIR.md` — House pairs **wgV/$PUSD**, not raw gV. `ROUTING.md` open choice locked to wgV.

## 2026-09-05 — gen-5 Lithe cutover source (not live)

Source-landed successor book (MarketCfg GEN 5). **Live 46630 remains gen-4** until an approved CutoverDeploy. Do not treat gen-4 addresses as the one-token book.

- PusdMarketFed + CanonicalLitheFactory: Fed V seigniorage Lithe (marketMinter); genesis mint funds converter + initiator float/DevFund; dual printers gV+Lithe; then setMinter(gV). Desk snapshot(address) ABI preserved. Oliver collateral = Fed V.
- LegacyVConverter / LitheCutoverMigrator: converter keeps cutover inventory; migration redeem→convert→seigniorage expand — **does not mint V** / does not grow canonical V supply.
- Factory deploys V + gV/policy + Lithe + converter + migrator + Oliver only. **Does not** deploy wgV, HousePairConfig, or House (ROUTING: House = wgV/). Remittance / sPUSD / SavingsRouter are **not** auto-wired — initiator must setRemittance (and related) post-deploy.
- Proofs: CanonicalVMarket.t.sol + CanonicalLitheFactory.t.sol. Local cutover clears house/pair_config so gen-4 House is not mixed with gen-5 V.

## 2026-09-05 — USDG relic lock (docs)

- **USDG** accepted **only** as Fed **treasury bond** asset (`BondAssetTag`: exogenous RFV in -> gV out).
- `$PUSD` stability = social-proof / mint-redeem ~par — **not** USDG depth.
- Scrubbed `$PUSD`/USDG pool / peg-depth / cash-depth product plans from `PUSD_LIQUIDITY.md`, `ROUTING.md`, `BONDS.md`, `TRACKS.md`. BondMarket USDG tag stays. No USDG AMM/pool contracts.

## v1.2 (testnet money)

| Gate | State |
|---|---|
| Live 46630 market (gen-4) | **Still gen-4 live** (embedded-V Lithe). Gen-5 cutover is source-only until CutoverDeploy. Market `0x47Aca529…3617` · V `0xD4b36DDe…7585` · P `0xBe71EF3e…E42e`. Retired `0x447F…` do not count. |
| KetPay settle | `pay.html` signs `wallet-send` `$PUSD` on 46630. Wallet refuses `$PUSD`/`$VAPURR` on 4663. `PayRouter` ignores `eip155:4663`. |
| Postage | `mail_postage` extra.token = canonical testnet `$PUSD` / `$VAPURR`. **VapurrForwarder + `vapurr-relay` source-landed** (`docs/RELAY.md`, `6385b67`) -- not deployed/live; postage still voucher until Relic enables forwarder+relay on 46630. |
| vapurrbid | Live `$PUSD` pay-to-rank on the testnet book. |
| Ketcharts listing | `$PUSD` pay-to-list. `TESTNET_KETLIST` empty until this device deploys `KetList.sol`. |
| Not v1.2 | Servo, Rain, live zer0ID issuer, mainnet `$PUSD`. |

## v1 progress

| v1 item | State |
|---|---|
| Native Windows window (tao + 4 WebView2s) | Ships |
| Chrome HTML in `frontend/` at `vapurr.localhost` | Ships |
| Packed `dist\vapurr\vapurr.exe` via `pack.ps1` | Ships `dist\vapurr-1.1.1-windows-x64.zip` (**43.3 MB**, git 341b022, no mp4; posters still in embed). **Install vapurr.exe** is the branded first-run (no admin, Start Menu). Logs in `%LOCALAPPDATA%\vapurr`. MP4/WEBM never pack — host on thesecretlab.app/vapurr/ (see docs/ketflix/HOSTING.md). House only. |
| Per-machine `install_id` | Ships — UUID at `%LOCALAPPDATA%\vapurr\install_id`, minted on successful Install (idempotent). Desk/earn payload includes it. See `docs/ketpay/INSTALL_ID.md`. |
| Brand tokens on screen (`frontend/tokens.css`) | Ships (`#c0f800` lime) |
| Tabs, home, settings — `desk.json` | Ships |
| Shield hooked into WebResourceRequested | Ships |
| Scan live (`/scan/api/*`, `vapurr-rhc`) | Ships |
| RHC liquidity graph (`/scan/api/liq`, Scan Liquidity tab) | Ships. Live Robinhood RPC (`vapurr-rhc::liq`). View is capped (≤48 nodes / 72 edges). Factory-log archive crawl was removed — it froze the chrome. Lookups use the full RPC book. |
| Live Trenches | Ships — rail/home open https://fomo.family in this window |
| PUSD/VAPURR on-chain market (`vapurr-econ` + `PusdMarket.sol`) | **Lithe** is the `$VAPURR` ↔ `$PUSD` mint/redeem rail at the oracle. Virtual CP spread, min 2%; its fee reserve supports up to 9% PUSD index drip. No USDG in the mint/burn loop. |
| PUSD vault (`PusdLoop.sol`, Oliver-shaped (Euler-family)) | Isolated `$PUSD` credit + `$VAPURR` collateral. Boot kink **150%** → **6%** as 100k real `$PUSD` cash lands. Looping does not fade the boot. **Live** `0x89E17eef…4521`. Old `0xC4d4…` retired. |
| House Uni v4 CL (`HouseLp.sol`) | **Live on 46630.** `0x667bFcAF…1bf7`. NFT #2273. `$VAPURR`/`$PUSD` 0.30% ±20%. Swapper `0x6304419b…4dD2` (PUSD settle-safe). Dead `0xb699…` / `0xb10d…` / `0xbD6b…` do not count. |
| Oliver vault (`PusdLoop.sol`) | **Live.** `0x89E17eef…4521`. Boot 150% kink, fades with exogenous cash. |
| vapurrbid (`vapurr://vapurrbid`, $PUSD pay-to-rank) | Ships. `Outbid.sol` + `vapurr-econ::outbid`. Rank is $PUSD paid. Aliases: `outbid`, `bid`, `board`. |
| Ketcharts listing (`vapurr://ketcharts` Listed) | Ships. `KetList.sol` + `vapurr-econ::ketlist`. Pay `$PUSD` to list a token (50 min, +25 to take #1). Profile (web/X/tg/discord/bio/logo) is on-chain with the payment. Snap paints Listed + pair card. Inbox copy `%LOCALAPPDATA%\vapurr\ketlist.json`. Never refunded. Organic tape stays. `TESTNET_KETLIST` empty until this device deploys it. |
| PNS (`vapurr://pns`, `.hood` names) | Live on testnet 46630. Registry `0xC0E6f3217525afc80FE89f077504D9E5377a4bB5` (EIP-1967 proxy, `PnsRegistryUpgradeable`, owns namehash `hood`; old bare-impl `0x13C9fCaB…0fC4` retired). ENS-shaped (namehash, addr, reverse, setAddr). Type `alice.hood` in the bar. |
| Swap / Bridge (`vapurr://swap`, `vapurr://bridge`) | Simulate on this device, then you sign and it broadcasts (`wallet-exec`). MAX + balances + impact. `$VAPURR` refund. 4663/46630. |
| Light theme (`data-theme="light"`) | Ships — rail + settings. Sage set in `frontend/tokens.css` |
| Bookmarks, history, cookies, Boost, radio | Ships |
| `cargo test --workspace` | Protocol crates have unit tests; keep green |
| 404 / zzzmail / card / wallet / swap / bridge / id as **honest skins** | Product dollar is **$PUSD**. **KetPay** (`vapurr://pay` / `ketpay`) settles `$PUSD` on **testnet 46630 only** — not mainnet 4663. 404 is load-fail. Postage voucher is bound to canonical testnet `$PUSD`. vapurrbid is `$PUSD`. Swap/bridge: simulate, then sign and broadcast. Earn-submit refuses payout without a VerifiedAccount (visits stay queued). `vapurr://id` opens thesecretlab.app/kyc; no fake Proven. See docs/zeroid/RHC.md. |
| `vapurr://id` vs Shield | Split. `id.html` is zer0ID (KYC CTA to Secret Lab). `shield.html` is adblock. Rail is `data-id="shield"` |
| Ketbook (`vapurr://ketbook`) | Ships — public product docs (what vapurr is, how it works). Source `ketbook/`. Internal specs stay in `docs/`. |
| WebView2 guest | **Allowed for v1.** Not the product engine. |


## 2026-09-04 evening — Oliver honesty (1.1.6)

- On-chain finding (vault `0x89E17eef…4521`, user `0xe718E24b…A56f`): debt=0, supplied≈0.0007 PUSD, wallet PUSD≈0, **collatV≈485846 VAPURR**, vault cash≈0.0007. UI had been rounding/hiding the V deposit behind wallet zeros.
- Pack **1.1.6**: hero shows **net Oliver** (wallet P + supplied − debt) + **clear V collateral** line; always-visible Unwind when debt>0; Withdraw P/V when debt==0 with cash-aware max; snap RPC/decode errors surface as red chip (no silent all-0 stub).
- `PusdLoop.loop()` source now caps each step by `min(room, cash)` so synthetic borrow cannot invent depth past cash. **Live vault still needs redeploy** for that on-chain cap; client hex updated via `contracts/compile-loop.mjs` for next deploy.
- Wallet desk decodes `collat_v` / cash / room; prefers unwind copy when debt>0.
## After v1 (do not gold-plate)

- **Servo** as the page engine. `vapurr-engine` feature `servo` `compile_error!`s until libservo is pinned.
- FetcherEngine as the user's browser.
- egui chrome (`vapurr-ui`) — unused by the binary.
- Live Rain card. zer0ID issuer (Secret Lab) is required for browse-earn payout; chrome does not fake Proven. No secrets in tree.
- KetPay / `$PUSD` spend on **mainnet 4663**. Testnet 46630 settlement is v1.2. zzzmail postage still a voucher until live `VapurrForwarder` + `vapurr-relay` post it (`docs/RELAY.md` source-only).
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
| `contracts/HouseLp.sol` | `crates/vapurr-econ/src/house.hex` — `contracts/compile-house.mjs` |
| `contracts/PusdLoop.sol` | `crates/vapurr-econ/src/loop.hex` — `contracts/compile-loop.mjs` (superseded by UUPS below once deployed) |
| `contracts/PusdLoopUpgradeable.sol` | `crates/vapurr-econ/src/loop_upgradeable_impl.hex` + `erc1967_proxy.hex` — `contracts/compile-loop-upgradeable.mjs` |
| `contracts/Outbid.sol` | `crates/vapurr-econ/src/outbid.hex` — `contracts/compile-outbid.mjs` |
| `contracts/KetList.sol` | `crates/vapurr-econ/src/ketlist.hex` — `contracts/compile-ketlist.mjs` |
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
- Mainnet PusdMarket vanity (not live): `MAINNET_MARKET_VANITY` `0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2`. Deployer `0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5` nonce 0. `VAPURR_MARKET` stays empty until the tx lands.
- Testnet `46630` — econ and KetPay spend here until mainnet has gas. RPC `https://rpc.testnet.chain.robinhood.com`. Live gen-4 book (until CutoverDeploy): market `0x47Aca5292423e2133A3eE983aB38291de3983617`, `$PUSD` `0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e`, `$VAPURR` `0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585` (all three still bare-impl, unfixed — see below), vault (Oliver, EIP-1967 proxy) `0x07d1085b545d5e1f55668a6a2EA9332233AaeC69`, house (EIP-1967 proxy) `0x603AaDFCD483aC196E2bcB158989dD5d38B24336`, swapper (EIP-1967 proxy) `0x4a00651238EAf8F849b8d8cbb7FD051a4D0f5383`, PNS (EIP-1967 proxy) `0xC0E6f3217525afc80FE89f077504D9E5377a4bB5`. Retired `0x447F…` / `0x435C…` / `0x59bB…` / `0x89E17eef…4521` (old bare-impl Oliver) / `0x667bFcAF…1bf7` (old bare-impl House) / `0x6304419b…4dD2` (old bare-impl Swap) / `0x13C9fCaB…0fC4` (old bare-impl PNS) do not count. `TESTNET_OUTBID` / `TESTNET_KETLIST` / mock USDG empty. **Proxy gate audit (2026-09-07):** Oliver/House/Swap/PNS now pass `scripts/verify-proxy.ps1`; market/`$PUSD`/`$VAPURR` still fail (bare implementations) — tokens are a separate, more debatable call (see top-of-file note); Market already has an unreleased gen-5 UUPS replacement gated behind an approved CutoverDeploy, not a fresh redeploy.

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

## 2026-09-05 — ungate Bonds / CD / SavingsRouter

- Relic: nothing product-critical stays gated. `frontend/bonds.html` Open Bond / Open CD live-by-default; capacity/oracle/haircut are params.
- `BondMarket` comments + tests ship enabled-with-capacity; `SavingsRouter` enabled by default (owner killswitch).
- Docs: `BONDS.md` scrubbed from gated-until posture to live-with-caps.



