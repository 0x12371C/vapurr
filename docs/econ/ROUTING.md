# VAPURR economic routing (canon)

Relic lock 2026-09-05. Living truth for Fed/branches split.

**Earnings / NERDOMICS:** `EARNINGS_ENGINE.md` - who pays whom, BrowserStream 50k **global** budget, flat-V solvency, Oliver ~6x banking leverage, `FeeAttribution` source ledger (House/Lithe/Oliver) -> RemittanceSink -> sPUSD.

## Institutional map

| Layer | Role | Surfaces |
|-------|------|----------|
| **Fed / Treasury** | Macro reserves, POL, policy inflation | RFV, **bonds** (visible: ETH / USDG / stocks), gV **dynamic 1–9%/yr** rebase (bond-util; mid ~3.5% unbound), BrowserStream earmark, runway floor |
| **Mint / branches** | Working cash + credit | Lithe + mint-spread ($PUSD), Oliver (secured $PUSD vs gV/V) |
| **Interbank market** | Equity meets cash | House: **wgV / $PUSD** (locked - see HOUSE_PAIR.md; not raw rebasing gV) |
| **Cash depth books** | Outside-money exit depth (not the peg story) | **`$PUSD` / USDG** (primary depth); optional `$PUSD` / ETH - see `PUSD_LIQUIDITY.md` |
| **Savings** | Slow-growth cash claim | **sPUSD**: liquid base yield; time locks earn more (CD-shaped, break fee) |

## Visible product map (user-facing)

What normals open in the app - not plumbing labels:

| Surface | Tokens / assets | One-liner |
|---------|-----------------|-----------|
| **Cash** | `$PUSD` · `sPUSD` · **`$PUSD`/USDG** | Spend/mint rail · savings after runway · **outside depth** (USDG book aids exit, does not define the peg) |
| **Equity** | `gV` · stake | Bond claim / stake path; Fed **1–9%/yr** policy rate to stakers only |
| **Bonds** | **ETH · USDG · major stocks** | Park asset -> get **gV at a discount** after a wait. Exogenous RFV in. |
| **House** | **wgV / $PUSD** | Equity meets cash. Wrap gV -> wgV before LP. **Not** peg defense. |

Bonds are a **first-class visible surface** (see `BONDS.md` + `vapurr://bonds`). They are not a hidden OMO footnote.

`$PUSD` **stability = social-proof / forced float** (mint-redeem ~par). USDG books are **exit depth**, not the peg mechanism — see `PUSD_LIQUIDITY.md`. Do not read House volume as dollar tightness.

## Hard walls

1. **Only Fed prints V** - **dynamic 1–9%/yr** rebase to gV stakers from bond-market utilization (mid ~3.5% when unbound). Browse never funded by this mint. See `POLICY_RATE.md`.
2. **BrowserStream** - **50k $VAPURR / 3y** from **already-minted treasury** (float migration). No USD cap (intentional convexity). Claim: install_id + KYC.
3. **$PUSD** - forced product float; mint/redeem ~**par** for social trust (**this** is the peg story). Not an equity lottery. USDG (primary) thickens **outside exit depth** — it does not replace mint-redeem social proof.
4. **Branch remittance** - Lithe/House/Oliver realized surplus -> one RemittanceSink (sink-level runway floor) -> **sPUSD** (not into gV rebase).
5. **404 != payments** - pay is HTTP 402 / x402.

## Inflows -> RFV battery

- House fees, Lithe + mint-spread, Oliver interest/liq surplus
- **Bonds (visible):** ETH / USDG / major stocks -> treasury cash / POL (see `BONDS.md`)
- **Exogenous $PUSD books:** LP / POL into **`$PUSD` / USDG** (primary **depth** book) and optional `$PUSD` / ETH - thickens exit liquidity around the forced float; bond RFV can seed but does not replace mint-redeem ~par (see `PUSD_LIQUIDITY.md`)

## Outflows

- BrowserStream (treasury V earmark)
- Post-stream $PUSD browse only from **surplus** above runway (later)
- gV rebase (policy mint to stakers only)
- sPUSD yield (cash surplus to savers)

## Open eng choices

- House leg: **wgV locked** (wstETH pattern) - see HOUSE_PAIR.md + WGV_HOUSE.md. Do not pair raw rebasing gV in AMM.
- Fed LOLR policy for Oliver bad debt
- Bond capacity / when not to bond
- BondMarket gated skeleton + HouseFeeRemit sketch + FeeAttribution ledger landed (P1 live enable still open; Uni skim adapter HouseUniSkim landed) - sPUSD CD sketch + Bonds `#spusd-cd` UI stub landed; Bonds tab Unavailable/Gated stub landed (GATE map; live wire open); House tab **wgV / $PUSD** visual stub on `pusd.html` (wrap-first gate; live pairConfig deploy still open)
- `$PUSD`/USDG pool params / fee / tick ranges (P1) - **sketch landed** in `PUSD_LIQUIDITY.md` (0.05% / +/-1% par proposed; deploy/addresses still open); Cash depth UI stub on `bonds.html`

## Market V redeem fence (2026-09-05)

`PusdMarket.swapPusdToV` redeems from **pre-funded / locked inventory only** (no `vapurr.mint`).
`swapVToPusd` locks V into the market instead of burning. Fed/gV rebase is the sole V printer.

## Shared runway + realized remittance (2026-09-05)

One `RemittanceSink` consolidates branch RFV cash; one `RunwayFloor` is enforced **at the sink** on `accountedRfv` (not as dual local pools on Oliver/Lithe). Branches remit **all realized** surplus into that sink; `forwardSurplus` cannot drain below the shared floor. Unpaid accrued interest and depositor principal are **not** exogenous RFV (circular if counted as both RFV and a user claim). Oliver: `pendingReserve`->`realizedReserve` on repay/liq, then remit realized (sole-owner cash OK). Lithe: inventory-backed `yieldReserve` remits in full to the same sink. See `RunwayRfv.t.sol` (`test_two_branches_remit_one_sink_floor`).


Tagged remits (UI/TVL "who paid"): wire branches through `FeeAttribution` (House/Lithe/Oliver) before `RemittanceSink`. Direct sink remits remain valid but unattributed. See `EARNINGS_ENGINE.md` + `FeeAttribution.t.sol`.

## Lithe remittance (2026-09-05)

`PusdMarket` (Lithe) remits realized `yieldReserve` to the same `IRemittance` / `RemittanceSink` path as Oliver (`setRemittance` / `remitSurplus`). Floor retain is sink-level (`ITreasuryRfv`); branches do not hold a second local floor. Holder drip (9% APY cap) still runs on accrue; remittance feeds sink -> sPUSD so branch fees can hit the savings path later. See `PUSD_LIQUIDITY.md` for exogenous peg books; remittance does not replace those. **Gap:** sink-held nominal $PUSD is still not outside USDG depth; peg remains mint-redeem ~par / social proof.

## 2026-09-05 - shared savings allocation

RemittanceSink -> SavingsRouter -> SPUSD / SpusdCd now implements the shared surplus split. The router starts disabled, accepts only its configured sink, checks matching assets, and cannot pierce the sink runway floor. Its CD allocation bps is a share of future receipts, not an APY. CD coupon targets and break fees are fixed per position; underfunding is proportional across open targets. Local tests are green; live deploy/address-book/IPC remains open. See [SPUSD.md](SPUSD.md) and [STACK_ECON_REVIEW_2026-09-05.md](STACK_ECON_REVIEW_2026-09-05.md).
