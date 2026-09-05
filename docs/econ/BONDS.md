# Bonds (canon)

Relic lock 2026-09-05. **Visible product surface** â€” not a Fed plumbing essay.

Bonds are how outside value becomes protocol RFV and how normals get **gV at a discount**.
Execution stays **gated** until Fed enables a market with inventory, capacity, haircuts, and valuation â€” not a fake open market.

## What the user does

1. Open **Bonds**.
2. Pick an asset tab: **ETH**, **USDG**, or a **major stock** (testnet/ops set).
3. Enter how much to bond.
4. See **discount %**, **wait / vesting**, and **gV you receive**.
5. Confirm -> asset moves in; a vesting claim for gV starts (**only if that tab is enabled**).
6. After the wait, **claim gV**. Optionally stake / wrap for House (`wgV` / `$PUSD`).

That is the whole loop. No essay. No external chain brand words.

## Gate (hard wall)

| Prerequisite | Why |
|--------------|-----|
| **Payout inventory** | Bond pays **gV/wgV from pre-funded inventory only** â€” never mints Fed supply |
| **Vesting ownership** | Unclaimed payout sits reserved in the bond book until unlock; claim transfers inventory |
| **Per-asset capacity** | Credited RFV capacity must be > 0; depletes on each bond |
| **Haircut** | Conservative cut on asset valuation before discount/face payout |
| **Valuation** | Fed-set `priceWad` until reliable RFV oracles (stocks: closure / corporate-action still open) |
| **`enabled` flag** | Default **false** (and/or **capacity 0**) until Fed turns a tab on |

Disabled or zero-capacity tabs **revert** on `bond()` â€” UI must not pretend the market is open.

Contract: `contracts/BondMarket.sol` (`BondAssetTag` = ETH / USDG / STOCKS). Proofs: `BondMarketTest`.

## One Bonds surface

| UI shows | UI hides |
|----------|----------|
| Asset, amount, discount %, vesting / wait, gV out | RFV battery internals, Fed printer, rebase math, OMO schedule jargon |
| Verb: **Bond** | "Stake into treasury" theatre |
| **Unavailable / gated** when disabled or no capacity | Fake open orderbook |

Same tab pattern for every v1 asset. Capacity / enable is Fed policy â€” see `ROUTING.md` open eng for live oracle / stock handling.

## v1 bond assets (exogenous RFV in)

| Asset | Role | Notes |
|-------|------|-------|
| **ETH** | Major exogenous collateral | Bond in -> gV out at discount (WETH/ERC20 path in skeleton) |
| **USDG** | Chain dollar | Bond in -> gV out at discount (**not** product `$PUSD`) |
| **Major stocks** | Testnet/ops set (e.g. AMZN / TSLA / AMD / NFLX / PLTR where live) | Bond in -> gV out at discount; same Bonds UI tab pattern; extra valuation gates |

`$PUSD` is **not** the headline bond asset â€” it is the spend/mint rail. Bonds are how **outside** value becomes protocol RFV + discounted gV.

## Vesting / wait (plain copy)

- Bonding is **not instant equity**. You wait.
- Screen copy: **"Wait X Â· then claim gV at Y% discount."**
- Early exit is out of scope for v1 UI unless eng ships a break path later.
- Claimed gV is yours: hold, stake for the Fed **3.5%/yr** rebase, or wrap to **wgV** for House.
- Unclaimed reserved inventory does **not** mint; claim is a transfer from the bond book.

## Where RFV goes

Bonded assets land as **exogenous RFV** in **Fed / Treasury** reserves (cash / POL battery via the market `treasury` sink). That battery backs policy: runway floor, later surplus routing, and the equity story â€” **not** a second dollar mint.

Downstream product map:

| Layer | What user sees |
|-------|----------------|
| **Fed / Treasury** | Macro RFV + policy (bonds feed here; gV rebase is Fed-only) |
| **Equity** | **gV** (stake) -> optional **wgV** wrap |
| **House** | Interbank: **wgV / $PUSD** (see `HOUSE_PAIR.md`) |
| **Cash** | **$PUSD** spend/mint Â· **sPUSD** savings (branch surplus after runway) |

Pointing rule: Bonds -> **Fed/Treasury RFV**. Equity after claim -> **gV / wgV**. Trading equity for cash -> **House wgV/$PUSD**. Do not tell users bonds mint `$PUSD` or Fed V.

## Product copy (ship)

- Verb: **Bond**
- Show: asset Â· discount % Â· vesting / wait Â· gV you receive Â· **gated when offline**
- Hide: RFV internals Â· Fed printer Â· rebase math
- Banned: external protocol brand names â€” vapurr-native symbols only

## Status

- **Skeleton (this branch):** `BondMarket` â€” quote + gated `bond`/`claim`, inventory fund, capacity, haircut, `enabled` default false / capacity 0.
- **UI unavailable / gated states:** landed stub in `frontend/bonds.html` (GATE map mirrors BondMarket defaults: `enabled=false` / `capacity=0`; banner + CTA say Unavailable/Gated — does not fake an open market). Live chain wire still open.
- **Still open before live enable:** reliable RFV valuation oracles; stock market-closure / corporate-action handling; rebase-ownership polish if payout token is live rebasing gV; live UI↔BondMarket reads.
- **sPUSD CD time-locks** â€” sketch landed (`SpusdCd.sol` + `SPUSD.md`); liquid sPUSD + CD coupon/break-fee proofs green. Live UI still open.

## Related

- `ROUTING.md` â€” bonds as visible RFV inflow + product map
- `HOUSE_PAIR.md` â€” gV -> wgV -> House **wgV/$PUSD**
- `SPUSD.md` â€” cash savings (not bond output)
- `CODEX_BRAIN_PASS.md` â€” bonds executable **partial** (gated)
- Frontend: `vapurr://bonds` â€” Cash / Equity / Bonds / House product map