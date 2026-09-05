# VAPURR economic routing (canon)

Relic lock 2026-09-05. Living truth for Fed/branches split.

## Institutional map

| Layer | Role | Surfaces |
|-------|------|----------|
| **Fed / Treasury** | Macro reserves, POL, policy inflation | RFV, **bonds** (visible: ETH / USDG / stocks), gV 3.5%/yr rebase, BrowserStream earmark, runway floor |
| **Mint / branches** | Working cash + credit | Lithe + mint-spread ($PUSD), Oliver (secured $PUSD vs gV/V) |
| **Interbank market** | Equity meets cash | House: **wgV / $PUSD** (locked - see HOUSE_PAIR.md; not raw rebasing gV) |
| **Cash peg books** | Forced-float depth vs outside money | **`$PUSD` / USDG** (primary, exogenous); optional `$PUSD` / ETH - see `PUSD_LIQUIDITY.md` |
| **Savings** | Slow-growth cash claim | **sPUSD**: liquid base yield; time locks earn more (CD-shaped, break fee) |

## Visible product map (user-facing)

What normals open in the app - not plumbing labels:

| Surface | Tokens / assets | One-liner |
|---------|-----------------|-----------|
| **Cash** | `$PUSD` Â· `sPUSD` Â· **`$PUSD`/USDG** | Spend/mint rail Â· savings after runway Â· **tight exogenous depth** (USDG book) |
| **Equity** | `gV` Â· stake | Bond claim / stake path; Fed 3.5%/yr to stakers only |
| **Bonds** | **ETH Â· USDG Â· major stocks** | Park asset -> get **gV at a discount** after a wait. Exogenous RFV in. |
| **House** | **wgV / $PUSD** | Equity meets cash. Wrap gV -> wgV before LP. **Not** peg defense. |

Bonds are a **first-class visible surface** (see `BONDS.md` + `vapurr://bonds`). They are not a hidden OMO footnote.

Cash peg trust is the **`$PUSD` / USDG** book (and other exogenous cash legs) - see `PUSD_LIQUIDITY.md`. Do not read House volume as dollar tightness.

## Hard walls

1. **Only Fed prints V** - flat **3.5%/yr rebase to gV stakers**. Browse never funded by this mint.
2. **BrowserStream** - **50k $VAPURR / 3y** from **already-minted treasury** (float migration). No USD cap (intentional convexity). Claim: install_id + KYC.
3. **$PUSD** - forced product float; mint/redeem ~**par** for social trust. Not an equity lottery. Depth against **outside** money (USDG primary).
4. **Branch remittance** - Lithe/House/Oliver realized surplus -> one RemittanceSink (sink-level runway floor) -> **sPUSD** (not into gV rebase).
5. **404 != payments** - pay is HTTP 402 / x402.

## Inflows -> RFV battery

- House fees, Lithe + mint-spread, Oliver interest/liq surplus
- **Bonds (visible):** ETH / USDG / major stocks -> treasury cash / POL (see `BONDS.md`)
- **Exogenous $PUSD books:** LP / POL into **`$PUSD` / USDG** (primary peg book) and optional `$PUSD` / ETH - thickens forced-float trust; bond RFV can seed but does not replace (see `PUSD_LIQUIDITY.md`)

## Outflows

- BrowserStream (treasury V earmark)
- Post-stream $PUSD browse only from **surplus** above runway (later)
- gV rebase (policy mint to stakers only)
- sPUSD yield (cash surplus to savers)

## Open eng choices

- House leg: **wgV locked** (wstETH pattern) - see HOUSE_PAIR.md. Do not pair raw rebasing gV in AMM.
- Fed LOLR policy for Oliver bad debt
- Bond capacity / when not to bond
- BondMarket gated skeleton + HouseFeeRemit sketch landed (P1 live enable still open; Uni skim adapter HouseUniSkim landed) · sPUSD CD sketch + Bonds `#spusd-cd` UI stub landed (live wire open)
- `$PUSD`/USDG pool params / fee / tick ranges (P1) - names locked in `PUSD_LIQUIDITY.md`

## Market V redeem fence (2026-09-05)

`PusdMarket.swapPusdToV` redeems from **pre-funded / locked inventory only** (no `vapurr.mint`).
`swapVToPusd` locks V into the market instead of burning. Fed/gV rebase is the sole V printer.

## Shared runway + realized remittance (2026-09-05)

One `RemittanceSink` consolidates branch RFV cash; one `RunwayFloor` is enforced **at the sink** on `accountedRfv` (not as dual local pools on Oliver/Lithe). Branches remit **all realized** surplus into that sink; `forwardSurplus` cannot drain below the shared floor. Unpaid accrued interest and depositor principal are **not** exogenous RFV (circular if counted as both RFV and a user claim). Oliver: `pendingReserve`->`realizedReserve` on repay/liq, then remit realized (sole-owner cash OK). Lithe: inventory-backed `yieldReserve` remits in full to the same sink. See `RunwayRfv.t.sol` (`test_two_branches_remit_one_sink_floor`).

## Lithe remittance (2026-09-05)

`PusdMarket` (Lithe) remits realized `yieldReserve` to the same `IRemittance` / `RemittanceSink` path as Oliver (`setRemittance` / `remitSurplus`). Floor retain is sink-level (`ITreasuryRfv`); branches do not hold a second local floor. Holder drip (9% APY cap) still runs on accrue; remittance feeds sink -> sPUSD so branch fees can hit the savings path later. See `PUSD_LIQUIDITY.md` for exogenous peg books; remittance does not replace those. **Gap:** sink-held nominal $PUSD is still not exogenous USDG backing.
