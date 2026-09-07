# Bonds (canon)

Relic lock 2026-09-05. **Visible product surface** — not a Fed plumbing essay.

Bonds are how outside value becomes protocol RFV and how normals get **gV at a discount**.
Markets ship **live with caps**: capacity, haircuts, and valuation are live params that can reject oversized or mispriced txs — they must not hide the product surface.

## What the user does

1. Open **Bonds**.
2. Pick an asset tab: **ETH**, **USDG**, or a **major stock** (testnet/ops set).
3. Enter how much to bond.
4. See **discount %**, **wait / vesting**, and **gV you receive**.
5. Confirm → asset moves in; a vesting claim for gV starts.
6. After the wait, **claim gV**. Optionally stake / wrap for House (`wgV` / `$PUSD`).

That is the whole loop. No essay. No external chain brand words.

## Live params (not UI gates)

| Param | Why |
|-------|-----|
| **Payout inventory** | Bond pays **gV/wgV from pre-funded inventory only** — never mints Fed supply |
| **Vesting ownership** | Unclaimed payout sits reserved in the bond book until unlock; claim transfers inventory |
| **Per-asset capacity** | Credited RFV capacity must be > 0; depletes on each bond; oversized bonds revert `CAP` / `CLOSED` |
| **Haircut** | Conservative cut on asset valuation before discount/face payout |
| **Valuation** | Fed-set `priceWad` until reliable RFV oracles (STOCKS session/early-close/halt/corp-action Fed-ops tables live; RFV oracle eng still open) |
| **`enabled` killswitch** | Prefer **true** with non-zero capacity on ship; `setEnabled(false)` is the safety pause |

Oversized, mispriced, or inventory-short bonds **revert on-chain** — UI stays actionable and surfaces the error. Do not gray-gate tabs as Unavailable by default.

Contract: `contracts/BondMarket.sol` (`BondAssetTag` = ETH / USDG / STOCKS). Proofs: `BondMarketTest`.

## One Bonds surface

| UI shows | UI hides |
|----------|----------|
| Asset, amount, discount %, vesting / wait, gV out | RFV battery internals, Fed printer, rebase math, OMO schedule jargon |
| Verb: **Open Bond** | "Stake into treasury" theatre |
| Live params (capacity / haircut / oracle) | Fake Unavailable / Gated as product posture |

Same tab pattern for every v1 asset. Capacity / valuation are Fed policy params — see `ROUTING.md` open eng for live oracle / stock handling.

## v1 bond assets (exogenous RFV in)

**Relic lock:** USDG is accepted **only** as this bond asset (exogenous RFV in -> gV out). No `$PUSD`/USDG AMM, peg-depth books, or competing stable markets — those hurt `$PUSD` (see `PUSD_LIQUIDITY.md`).

| Asset | Role | Notes |
|-------|------|-------|
| **ETH** | Major exogenous collateral | Bond in → gV out at discount (WETH/ERC20 path in skeleton) |
| **USDG** | Fed treasury bond asset (`BondAssetTag`) | Bond in -> gV out at discount. **Only** role for USDG: exogenous RFV in. **Not** product `$PUSD`, **not** a `$PUSD`/USDG pool or peg-depth book. |
| **Major stocks** | Testnet/ops set (e.g. AMZN / TSLA / AMD / NFLX / PLTR where live) | Bond in → gV out at discount; same Bonds UI tab pattern; valuation params apply |

`$PUSD` is **not** the headline bond asset — it is the spend/mint rail. Bonds are how **outside** value becomes protocol RFV + discounted gV.

**Relic lock:** USDG is **BondAssetTag only** (Fed treasury bond intake → discounted gV). No `$PUSD`/USDG pool, peg-depth, or cash-depth product — that hurts `$PUSD` (see `PUSD_LIQUIDITY.md`).

## Vesting / wait (plain copy)

- Bonding is **not instant equity**. You wait.
- Screen copy: **"Wait X · then claim gV at Y% discount."**
- Early exit is out of scope for v1 UI unless eng ships a break path later.
- Claimed gV is yours: hold, stake for the Fed **dynamic 1–9%/yr** rebase (bond-util; see `POLICY_RATE.md`), or wrap to **wgV** for House.
- Unclaimed reserved inventory does **not** mint; claim is a transfer from the bond book.

## Where RFV goes

Bonded assets land as **exogenous RFV** in **Fed / Treasury** reserves (cash / POL battery via the market `treasury` sink). That battery backs policy: runway floor, later surplus routing, and the equity story — **not** a second dollar mint.

Downstream product map:

| Layer | What user sees |
|-------|----------------|
| **Fed / Treasury** | Macro RFV + policy (bonds feed here; gV rebase is Fed-only) |
| **Equity** | **gV** (stake) → optional **wgV** wrap |
| **House** | Interbank: **wgV / $PUSD** (see `HOUSE_PAIR.md`) |
| **Cash** | **$PUSD** spend/mint · **sPUSD** savings (branch surplus after runway) |

Pointing rule: Bonds → **Fed/Treasury RFV**. Equity after claim → **gV / wgV**. Trading equity for cash → **House wgV/$PUSD**. Do not tell users bonds mint `$PUSD` or Fed V.

## Product copy (ship)

- Verb: **Open Bond** / **Open CD**
- Show: asset · discount % · vesting / wait · gV you receive · live params
- Hide: RFV internals · Fed printer · rebase math
- Banned: external protocol brand names — vapurr-native symbols only

## Stock session / corporate-action (Fed ops)

STOCKS tabs stay **visible and actionable** (same live-by-default rule as ETH / USDG). Session and corp-action are **valuation / killswitch policy**, not UI gray-gates.

| Event | Fed action | User-facing |
|-------|------------|-------------|
| **Regular session closed** (nights / weekends / holidays) | Prefer `setValuation` freeze (hold last good `priceWad`) or `setEnabled(false)` on `BondAssetTag.STOCKS` until open | CTA stays; on-tx may revert `CLOSED` / misprice — surface the error, do not invent overnight marks |
| **Early close** (day before Independence Day observance / day after Thanksgiving / Christmas Eve; 13:00 ET) | Same as session closed after 13:00 ET | Banner shows early-close open/done; CTA stays |
| **Halt / circuit breaker** | Same as session closed until Fed clears | Honest banner copy once address-book reads land ("valuation paused") |
| **Corporate action** (split, merger, special dividend, ticker change) | Pause market (`setEnabled(false)`), reset wrapper/`priceWad`/`haircutBps`, then re-enable | No auto-repricing in the client; Fed publishes new terms |

ETH / USDG do **not** use the equity session calendar. USDG remains BondAssetTag-only RFV intake (no `$PUSD`/USDG pool).

## ETH / USDG valuation (Fed ops)

ETH and USDG tabs stay **visible and actionable** 24/7. There is no equity open/close clock — Fed ops are **oracle / valuation / killswitch**, not gray-gates.

| Event | Fed action | User-facing |
|-------|------------|-------------|
| **Oracle stale / heartbeat miss** | Hold last good `priceWad` via `setValuation` or `setEnabled(false)` on `BondAssetTag.ETH` / `USDG` until feed recovers | CTA stays; on-tx may revert misprice / disabled — surface the error, do not invent marks |
| **Oracle jump / circuit** | Same freeze or killswitch until Fed clears | Honest banner once address-book reads land ("valuation paused") |
| **Haircut / capacity retune** | `setHaircutBps` / capacity params live — no UI gate | Quote may show thinner credit; CTA stays |
| **USDG intake pause** | `setEnabled(false)` on USDG tag only | Tab stays; USDG remains BondAssetTag-only (never a `$PUSD`/USDG pool) |

**Still open eng:** reliable RFV oracles. STOCKS UI banner uses America/New_York regular-session calendar (09:30-16:00 ET + 2026 holiday table + early-close table Jul 2 / Nov 27 / Dec 24 at 13:00 ET) plus empty `US_EQUITY_FED_OPS` halt/corp-action advisory map (banner-only). Fed setValuation/setEnabled still owns live halt/corp-action - CTA never gray-gated. ETH/USDG `session: n/a` shows valuation-ops copy plus empty `CRYPTO_BOND_FED_OPS` stale/jump/intake-pause advisory map (banner-only).

## Status

- **`BondMarket`:** quote + `bond`/`claim`, inventory fund, capacity, haircut; ship **enabled with sane capacity**. Killswitch via `setEnabled(false)`.
- **UI:** `frontend/bonds.html` - live-by-default tabs; Open Bond / Open CD CTAs actionable; example labels remain on placeholder numbers. Address-book / IPC: empty `TESTNET_BOND_MARKET` + `MarketCfg.bond_market` + snap `bonds`; `econ-bond` parses and returns **NeedBondMarket** until Relic fills the CA. Missing market address surfaces a clear on-tx error, not a gray gate.
- **Still open:** reliable RFV valuation oracles; live UI↔BondMarket reads once addresses land. STOCKS session/early-close/Fed-ops advisory banner live (hours/weekend/holiday/early-close + empty halt/corp map). ETH/USDG empty `CRYPTO_BOND_FED_OPS` valuation advisory map live (banner-only).
- **sPUSD CD:** `SpusdCd.sol` open live (no disabled flag). `SavingsRouter` enabled by default; owner may `setAllocation(false, ...)`.

## Policy rate signal

Bond offtake feeds the Fed gV **policy rate** (not a second printer):

- Higher capacity utilization / hotter offtake → policy rate **down toward 1%/yr**
- Colder book → **up toward 9%/yr**
- Unbound / empty signal → ~**3.5%/yr** mid

Formula and clamp: `POLICY_RATE.md`. Contracts: `BondMarket.capacityUtilizationWad` → `RebasePolicy.policyRateBps` → `gVAPURR.accrue`.

## Related

- `ROUTING.md` — bonds as visible RFV inflow + product map
- `POLICY_RATE.md` — Fed gV rate from bond utilization
- `HOUSE_PAIR.md` — gV → wgV → House **wgV/$PUSD**
- `SPUSD.md` — cash savings (not bond output)
- `CODEX_BRAIN_PASS.md` — bonds executable progress
- Frontend: `vapurr://bonds` — Cash / Equity / Bonds / House product map

## Trading / POL books (not bond purchase)

At genesis/cutover, stand up **V paired with each bondable exogenous asset** as **trading/POL books**:

| Pair | Role |
|------|------|
| **V/ETH** | Hub POL / trading depth (genesis) |
| **V/NVDA** | POL / trading depth for NVDA stock wrapper (genesis) |
| **V/AMD** | POL / trading depth for AMD stock wrapper (genesis) |
| (+ configured stock tags) | Same pattern via ExogenousPairRegistry.registerPair |

**Locked genesis POL seed (of the 1M launch):** V/ETH **80k** · V/NVDA **25k** · V/AMD **25k**. Full 1.2M table in `GENESIS_ALLOCATION.md`.

These are **distinct from Open Bond** (exogenous in → gV out). Bond purchase still uses BondMarket / BondAssetTag.

**Banned as pairs:** V/USDG, PUSD/USDG. USDG remains **bond intake only**.

Contracts: ExogenousPairRegistry.sol (+ optional ExogenousSeedMarket stub). Wire: LaunchBootstrap. Proofs: ExogenousPairRegistry.t.sol. Live Uni v4 pool addresses remain empty until deploy — UI honest-empty.

### NeedBondMarket honesty UI (2026-09-06)

frontend/bonds.html #bonds paints #bond-book-note from snap.bonds.configured (empty book -> NeedBondMarket). Open Bond CTA stays live; capital strip shows BondMarket/Savings CA status. Live quote ABI still Relic-gated after deploy.

### Live ops honesty (2026-09-06 ~21:14 ET)

Robinhood testnet **46630** BondArb session drained **availableInventory gV to 0** (reserved payout still vesting). App address-book TESTNET_BOND_MARKET remains empty, so UI stays **NeedBondMarket** until Relic fills the CA (live inventory reads land with that wire).

**STOCKS RFV sink:** BondMarket STOCKS treasury retargeted to **ExoRfvSink** so stock offtake is spendable RFV (WETH to wgV to V). Distinct from Open Bond UX: treasury destination is Fed ops, not a tab gray-gate. ETH / USDG / STOCKS tabs stay live-by-default; USDG remains **BondAssetTag only** (no $PUSD/USDG pool).

Prove: scripts/verify-bonds-book.py.

### IPC ABI stubs (2026-09-07)

Client stubs on disk (not live-wired): `crates/vapurr-econ/src/bond_market.abi.json` (+ `spusd_cd.abi.json` / `savings_router.abi.json`). `open_bond` stays `NeedBondMarket` until Relic fills the CA and reviews IPC. Prove: `scripts/verify-bond-cd-abi.py`.

### ExoRfvSink ABI stub (2026-09-07)

STOCKS treasury sink client stub on disk (ops/keeper spend path, not Open Bond UX): `crates/vapurr-econ/src/exo_rfv_sink.abi.json`. Live CA on 46630 is documented in `STATUS.md` / `RFV_STOCK_ETH_V_LOOP.md`. No user IPC cmd yet — stub only. Prove: `scripts/verify-exo-rfv-abi.py`.

## Exo RFV sink (ops)
- Source: `contracts/ExoRfvSink.sol` (keeper/owner spendable sink for BondMarket treasury retarget after deploy).
- ABI stub: `crates/vapurr-econ/src/exo_rfv_sink.abi.json` — prove `scripts/verify-exo-rfv-abi.py`.
- Not a user IPC path; GenesisTreasury V-carve stays untouched.

### ExogenousPairRegistry ABI stub (2026-09-07)
Client stub on disk (POL / bootstrap ops; not Open Bond UX): crates/vapurr-econ/src/exogenous_pair_registry.abi.json + exogenous_seed_market.abi.json (seed stub). Source: contracts/ExogenousPairRegistry.sol (+ ExogenousSeedMarket in same file). Live Uni v4 pool addresses stay empty until Relic deploy - UI honest-empty. No user IPC cmd yet - stub only. Prove: scripts/verify-exo-pair-abi.py.

