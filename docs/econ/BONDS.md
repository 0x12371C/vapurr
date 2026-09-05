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
| **Valuation** | Fed-set `priceWad` until reliable RFV oracles (stocks: closure / corporate-action still open) |
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

| Asset | Role | Notes |
|-------|------|-------|
| **ETH** | Major exogenous collateral | Bond in → gV out at discount (WETH/ERC20 path in skeleton) |
| **USDG** | Chain dollar | Bond in → gV out at discount (**not** product `$PUSD`) |
| **Major stocks** | Testnet/ops set (e.g. AMZN / TSLA / AMD / NFLX / PLTR where live) | Bond in → gV out at discount; same Bonds UI tab pattern; valuation params apply |

`$PUSD` is **not** the headline bond asset — it is the spend/mint rail. Bonds are how **outside** value becomes protocol RFV + discounted gV.

## Vesting / wait (plain copy)

- Bonding is **not instant equity**. You wait.
- Screen copy: **"Wait X · then claim gV at Y% discount."**
- Early exit is out of scope for v1 UI unless eng ships a break path later.
- Claimed gV is yours: hold, stake for the Fed **3.5%/yr** rebase, or wrap to **wgV** for House.
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

## Status

- **`BondMarket`:** quote + `bond`/`claim`, inventory fund, capacity, haircut; ship **enabled with sane capacity**. Killswitch via `setEnabled(false)`.
- **UI:** `frontend/bonds.html` — live-by-default tabs; Open Bond / Open CD CTAs actionable; example labels remain on placeholder numbers. Address-book wire still open — missing market address surfaces a clear on-tx error, not a gray gate.
- **Still open:** reliable RFV valuation oracles; stock market-closure / corporate-action handling; live UI↔BondMarket reads once addresses land.
- **sPUSD CD:** `SpusdCd.sol` open live (no disabled flag). `SavingsRouter` enabled by default; owner may `setAllocation(false, ...)`.

## Related

- `ROUTING.md` — bonds as visible RFV inflow + product map
- `HOUSE_PAIR.md` — gV → wgV → House **wgV/$PUSD**
- `SPUSD.md` — cash savings (not bond output)
- `CODEX_BRAIN_PASS.md` — bonds executable progress
- Frontend: `vapurr://bonds` — Cash / Equity / Bonds / House product map
