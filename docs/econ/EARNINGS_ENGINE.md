# Earnings engine (NERDOMICS)

Relic lock 2026-09-05. Dense canon for who pays whom, what redeem delivers, and the flat-V solvency frame.
Cross-refs: `ROUTING.md`, `SPUSD.md`, `PUSD_V_REDEEM.md`, `BONDS.md`, `PUSD_LIQUIDITY.md`, `BrowserStream` in `GvFed.sol`.

Vapurr-native names only. No banned externals.

---

## 0. One diagram (cash + equity)

```
Exogenous in (bonds: ETH/USDG/stocks) -----> Fed/Treasury RFV battery
                                              |
                                              +--> gV inventory out (discounted claim)
                                              +--> BrowserStream earmark (already-minted V)

Branch surplus (realized $PUSD only):
  House protocol carve  --\
  Lithe mint-spread     ---> FeeAttribution (source tag) -> RemittanceSink (runway floor) -> sPUSD / CD
  Oliver interest/liq   --/

Equity print (separate pipe):
  Fed policy -> gV.rebase @ dynamic 1-9%/yr from bond util (ONLY V inflation to stakers; mid ~3.5% unbound)
  Browse/earn -> BrowserStream.drip (transfer; never mint)
```

Hard split: **cash surplus path != equity rebase path**. Browse never funds from rebase mint.

---

## 1. Who pays whom

| Payer (source) | What they pay | Asset | Receiver path |
|----------------|---------------|-------|---------------|
| **House** | Protocol fee carve (not LP fees) | realized `$PUSD` | `HouseFeeRemit` / `HouseUniSkim` -> **FeeAttribution(House)** -> `RemittanceSink` |
| **Lithe** | Mint/redeem spread + market fee inventory | realized `$PUSD` (`yieldReserve`) | `PusdMarket.remitSurplus` -> **FeeAttribution(Lithe)** -> sink |
| **Oliver** | Realized borrow interest / liq surplus | realized `$PUSD` (`realizedReserve`) | `PusdLoop.remitReserve` -> **FeeAttribution(Oliver)** -> sink |
| **Bond buyers** | Exogenous asset at haircut | ETH / USDG / stocks | Fed/Treasury RFV; payout = **gV inventory** (not cash yield) |
| **Fed** | Policy inflation | new `$VAPURR` into gV index | **gV stakers only** (dynamic 1-9%/yr; see `POLICY_RATE.md`) |
| **Treasury** | Already-minted float migration | `$VAPURR` earmark | **BrowserStream** (global 50k / 3y) |

Savers (`sPUSD` / CD) are paid **only** from sink surplus **above** `RunwayFloor`. They do not receive gV rebase.

UI/TVL "who paid the yield": read `FeeAttribution.breakdown()` / `shareBps(Source)`. Untagged direct remits to sink do not update the ledger.

---

## 2. BrowserStream: 50k V is GLOBAL

Contract: `BrowserStream` (`GvFed.sol`).

| Param | Value | Meaning |
|-------|-------|---------|
| `CAP` | `50_000 ether` | **Global** release ceiling over the stream life |
| `DURATION` | `3 * 365 days` | Linear vest clock |
| Funding | `fund()` transferFrom | **Already-minted** treasury V; `drip` never calls `mint` |

Budget math (linear, inventory-capped):

```
vested(t)     = min(balance+released, CAP) * min(t, DURATION) / DURATION
releasable(t) = min(vested - released, balance)
global rate   ~ CAP / DURATION
              ~ 50_000 / (3*365) V per day
              ~ 45.66 V/day across ALL claimants combined
```

### Contribution-per-user identity (claims != budget)

- **Supply budget** is one global CAP. There is no per-user 50k, no per-install mint allotment, no second printer.
- **Claim identity** (distributor policy): `install_id` + KYC (and whatever sybil gates ops set). Identity decides *eligibility / share of drip*, not the size of the earmark.
- Per-user lifetime receipt is a **slice of the global vest**, not a parallel budget. Sum of all successful `drip` <= `CAP` (and <= funded inventory).

If product copy says "earn V by browsing", the nerds' reading is: **pro-rata / policy share of a fixed 50k treasury earmark**, not inflationary APY.

---

## 3. Inflation vs browse

| Path | Mints V? | Who gets it |
|------|----------|-------------|
| `gVAPURR.rebase` (Fed `RebasePolicy`) | **YES** — sole inflation | gV stakers via index (`REBASE_BPS = 350` flat / year) |
| `BrowserStream.drip` | **NO** — transfer only | browse/earn distributor recipients |
| Lithe `swapPusdToV` | **NO** — seigniorage redeem (mint V) | redeemer (burns `$PUSD`, takes extant V) |
| Bonds claim | **NO** — inventory transfer | bond claimer (pre-funded gV/wgV) |

gV applies **`policyRateBps()`** (clamp 100–900; mid 350 when bonds unbound) annualized per settlement interval to current supply. Repeated settlements compound slightly. `accrue()` is permissionless; `rebase()` is the policy-only alias. Hot bond util -> toward 1%; cold -> toward 9%. See `POLICY_RATE.md`. Equity issuance, separate from Oliver leverage and cash earnings.

---

## 4. `$PUSD` redeem delivers what (incl. stress)

Canonical redeem surfaces:

| Action | Delivers | Under stress |
|--------|----------|--------------|
| Lithe `swapPusdToV` | Extant `$VAPURR` from **market inventory** | Reverts `INV` / empty inventory — **never** Fed mint |
| Lithe mint/redeem ~par (cash rail) | `$PUSD` <-> inventory / spread path | Spread widens; still no V print |
| Exit vs outside money | No `$PUSD`/USDG cash market; USDG = **bond/treasury intake only** | Peg = mint-redeem ~par; House = equity exit |
| sPUSD redeem | Underlying `$PUSD` at vault NAV | NAV can lag; seigniorage V mint on redeem into shortfall |
| V "backing" intuition | **Not** a dollar claim on Fed RFV | Equity token; mcap != RFV |

**Forced float:** social trust is ~par mint/redeem (`PUSD_LIQUIDITY.md`). **USDG** is BondAssetTag / Fed treasury intake only — not a competing cash or peg book. Sink-held nominal `$PUSD` is **not** dollar solvency.

Stress sentence: *If inventory is empty, `$PUSD` does not conjure V or dollars — it waits on inventory or discounted equity exit via House. No USDG cash/peg pool fills the gap.*

---

## 5. mcap != backing

| Measure | Formula (sketch) | Counts as |
|---------|------------------|-----------|
| Equity **mcap** | price(V or gV) * supply | Market's equity story |
| **RFV / backing** | exogenous assets (bond inflows, USDG/ETH POL, etc.) + sink `accountedRfv` (nominal) | Treasury battery / runway |
| Recursive inventory | protocol-held `$PUSD` / V locked in Lithe or House | **Not** exogenous dollar solvency |

Do not quote mcap as "backed by." Do not quote sink `$PUSD` alone as dollar peg trust. TVL panels that sum House + Lithe + Oliver + sink without stripping circular legs **lie**.

---

## 6. Rebase vs purchasing power

- **Rebase** increases gV **balances** (index up at the live policy rate, 1–9%/yr). Staker share of *equity supply* is preserved among stakers; unstaked V is diluted relative to staked.
- **Purchasing power** vs dollars is `balance_gV * price_V_in_USD` (or House wgV/`$PUSD` quote). Rebase does not print dollars, invent a `$PUSD`/USDG cash book, or credit sPUSD.
- Holding gV through rebase is an **equity index claim**, not a cash-yield coupon. Cash coupon = sPUSD path from branch surplus.

---

## 7. Bonds as financing (dilution vs RFV)

Bonds: exogenous asset **in** -> discounted **gV inventory out** (`BONDS.md`).

| Lens | Reading |
|------|---------|
| Financing | Protocol raises RFV now; pays with equity claim later (vesting) |
| Dilution | gV that could have stayed in treasury inventory is sold at a **discount** to face |
| RFV | Haircut + asset valuation credits treasury battery (exogenous) |
| Not | A second V mint; a `$PUSD` printer; peg defense by itself |

Discount = explicit financing cost. Capacity / `enabled` gates exist because blind bonding dilutes inventory without reliable RFV valuation.

---

## 8. Oliver leverage = banking, not APY subsidy

`PusdLoop` (Oliver): supply `$PUSD`, collateral V, borrow `$PUSD`.

| Param | Value | Banking read |
|-------|-------|--------------|
| `LTV_BPS` | 8500 (85%) | Max debt / collat value |
| `LLTV_BPS` | 9000 (90%) | Liq threshold |
| Implied max leverage on equity | `1 / (1 - 0.85) â‰ˆ 6.67x` | **~6x banking leverage** |

This leverage is **borrow vs collateral** (balance-sheet). It is **not**:

- reward APY on deposits styled as "6x",
- rebase-as-yield / reward-APY-as-leverage marketing,
- a claim that Fed policy-rate rebase is levered into cash.

Oliver realized interest (after repay/liq realization) remits to the sink. Unpaid `pendingReserve` is **not** remittable RFV.

---

## 9. Flat-V / no-new-bonds solvency test

Frame (stress, not a happy path):

**Assumptions**

1. **Flat V:** no new Fed rebase mint credited to cash solvency (rebase may still run for equity optics; cash test ignores it).
2. **No new bonds:** no fresh exogenous RFV inflows.
3. BrowserStream only moves already-earmarked inventory (cannot refill from mint).

**Pass conditions (cash / peg)**

| Bucket | Must hold |
|--------|-----------|
| Lithe V redeem | Inventory >= expected `swapPusdToV` demand or redeem cleanly reverts (no mint) |
| Oliver | Depositor claims backed by `cash + totalBorrowAssets`; remits only `realizedReserve`; owner LTV after remit |
| RemittanceSink | `accountedRfv` respects single `RunwayFloor`; `forwardSurplus` cannot pierce floor |
| Peg trust | Mint-redeem ~par / social proof — **no** `$PUSD`/USDG depth book; sink nominal `$PUSD` insufficient alone |
| sPUSD / CD | Coupons / NAV only from surplus above floor; underfunded CD pays principal first, never mints |

**Fail tells**

- Any path that mints V to meet `$PUSD`->V redeem.
- Counting unpaid Oliver interest or depositor principal as RFV (**circular**).
- Dual local runway floors on Oliver/Lithe that double-retain vs sink.
- TVL that sums recursive legs as exogenous backing.
- Treating House wgV/`$PUSD` volume as peg defense.

This is the **earnings-engine solvency sketch**: yield to savers is downstream of realized branch surplus after one shared floor; equity print and browse earmark are separate; bonds are optional RFV financing, not a stealth printer.

---

## 10. Double-count rules (fixed overnight — do not reopen)

Consolidated from `Remittance.sol`, `RunwayRfv.t.sol`, Lithe burn-before-drip, ROUTING:

1. **Realized only.** Remit cash in hand (`realizedReserve` / `yieldReserve` / `feeReserve`). `pendingReserve` accrues but does not remit until repay/liq realization.
2. **No circular RFV.** Depositor principal and unpaid interest are user claims, not treasury RFV.
3. **One sink floor.** `RunwayFloor` enforced on `RemittanceSink.accountedRfv` only. Branches remit full realized; they do not hold a second retain floor.
4. **Lithe single surplus pool.** Fee inventory burned/consumed before holder drip so drip + remittance do not double-claim the same fee dollar.
5. **FeeAttribution is tagging, not a second sink.** It forwards to `RemittanceSink`; it must not retain runway or invent surplus.
6. **TVL hygiene.** Exclude recursive protocol inventory when quoting exogenous backing; attribute saver yield via `FeeAttribution` source shares, not mcap.

Proofs: `RunwayRfv.t.sol`, `HouseFeeRemit.t.sol`, `FeeAttribution.t.sol`, Lithe/Oliver routing fences.

---

## 11. Implementation map

| Piece | Path |
|-------|------|
| Sink + floor | `contracts/Remittance.sol` |
| Source ledger | `contracts/FeeAttribution.sol` (`House` / `Lithe` / `Oliver`) |
| House carve | `HouseFeeRemit.sol`, `HouseUniSkim.sol` |
| Lithe | `PusdMarket.sol` (`yieldReserve`, `remitSurplus`) |
| Oliver | `PusdLoop.sol` (`realizedReserve`, `remitReserve`) |
| Savings | `SPUSD.sol`, `SpusdCd.sol` |
| Equity + stream | `GvFed.sol` (`gVAPURR`, `BrowserStream`) |
| Bonds | `BondMarket.sol` |

Suggested wire for tagged TVL: `setRemittance(FeeAttribution)` on branches; `FeeAttribution.register(branch, Source)`; sink `setForward(SavingsRouter)` for shared liquid/CD allocation, or direct `setForward(sPUSD)` for liquid only. SavingsRouter starts disabled; see `SPUSD.md` for wiring and funded coupon rules.

---

## STATUS one-liner

Earnings = House + Lithe + Oliver **realized** `$PUSD` -> **FeeAttribution** -> **RemittanceSink** (runway) -> sPUSD/CD; V inflation = gV policy rate (1-9%) only; BrowserStream 50k/3y **global** treasury transfer; leverage = Oliver ~6x LTV banking; flat-V/no-bond stress forbids mint-to-redeem and circular RFV.

## 2026-09-05 source review and savings implementation

See [STACK_ECON_REVIEW_2026-09-05.md](STACK_ECON_REVIEW_2026-09-05.md) for the source/release map, self-loop economics, and remaining provenance/solvency gaps. SavingsRouter now splits post-floor surplus between liquid sPUSD and CD coupon cash. CD terms are fixed at entry; underfunded coupons share available cash in proportion to all open targets, including unmatured positions. Targets are contingent; closing extinguishes any unpaid portion. Source integration is tested, not deployed. The 6x label describes gross claims/debt; loop() does not add cash or multiply underlying Lithe earnings.
