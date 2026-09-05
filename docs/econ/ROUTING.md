# VAPURR economic routing (canon)

Relic lock 2026-09-05. Living truth for Fed/branches split.

**Earnings / NERDOMICS:** `EARNINGS_ENGINE.md` - who pays whom, BrowserStream 50k **global** budget, flat-V solvency, Oliver ~6x banking leverage, `FeeAttribution` source ledger (House/Lithe/Oliver) -> RemittanceSink -> sPUSD.

## Institutional map

| Layer | Role | Surfaces |
|-------|------|----------|
| **Fed / Treasury** | Macro reserves, POL, policy inflation | RFV, **bonds** (visible: ETH / USDG / stocks), **exogenous POL books** (V/ETH, V/NVDA, V/AMD), gV **dynamic 1–9%/yr** rebase (bond-util; mid ~3.5% unbound), BrowserStream earmark, **DevFundStream** 200k/4y genesis, runway floor |
| **Mint / branches** | Working cash + credit | Lithe + mint-spread ($PUSD), Oliver (secured $PUSD vs gV/V) |
| **Interbank market** | Equity meets cash | House: **wgV / $PUSD** (locked - see HOUSE_PAIR.md; not raw rebasing gV) |
| **Savings** | Slow-growth cash claim | **sPUSD**: liquid base yield; time locks earn more (CD-shaped, break fee) |

## Visible product map (user-facing)

What normals open in the app - not plumbing labels:

| Surface | Tokens / assets | One-liner |
|---------|-----------------|-----------|
| **Cash** | `$PUSD` · `sPUSD` | Spend/mint rail · savings after runway. Peg = mint-redeem ~par (see `PUSD_LIQUIDITY.md`). |
| **Equity** | `gV` · stake | Bond claim / stake path; Fed **1–9%/yr** policy rate to stakers only |
| **Bonds** | **ETH · USDG · major stocks** | Park asset -> get **gV at a discount** after a wait. Exogenous RFV in. |
| **House** | **wgV / $PUSD** | Equity meets cash. Wrap gV -> wgV before LP. **Not** peg defense. |

Bonds are a **first-class visible surface** (see `BONDS.md` + `vapurr://bonds`). They are not a hidden OMO footnote.

`$PUSD` **stability = social-proof / forced float** (mint-redeem ~par). **USDG is BondAssetTag only** (Fed treasury bond -> gV); no `$PUSD`/USDG pools or peg-depth books — see `PUSD_LIQUIDITY.md`. Do not read House volume as dollar tightness.

## Hard walls

1. **Only Fed prints V** - **dynamic 1–9%/yr** rebase to gV stakers from bond-market utilization (mid ~3.5% when unbound). Browse never funded by this mint. See `POLICY_RATE.md`.
2. **BrowserStream** - **50k $VAPURR / 3y** from **already-minted treasury** (float migration). No USD cap (intentional convexity). Claim: install_id + KYC.
2b. **DevFundStream** - **200k $VAPURR / 4y** genesis mint; unlocked V auto-locks as Oliver collateral; $PUSD-only draw; expansion-aware. See `DEV_FUND.md`.
3. **$PUSD** - forced product float; mint/redeem ~**par** for social trust (**this** is the peg story). Not an equity lottery. USDG is **bond-in RFV only** - never a `$PUSD`/USDG depth/peg pool.
4. **Branch remittance** - Lithe/House/Oliver realized surplus -> one RemittanceSink (sink-level runway floor) -> **sPUSD** (not into gV rebase).
5. **404 != payments** - pay is HTTP 402 / x402.

## Inflows -> RFV battery

- House fees, Lithe + mint-spread, Oliver interest/liq surplus
- **Bonds (visible):** ETH / USDG / major stocks -> treasury cash / POL (see `BONDS.md`)

## Outflows

- BrowserStream (treasury V earmark)
- DevFundStream (genesis 200k lockup; expansion-aware)
- Post-stream $PUSD browse only from **surplus** above runway (later)
- gV rebase (policy mint to stakers only)
- sPUSD yield (cash surplus to savers)

## Open eng choices

- House leg: **wgV locked** (wstETH pattern) - see HOUSE_PAIR.md + WGV_HOUSE.md. Do not pair raw rebasing gV in AMM.
- Fed LOLR policy for Oliver bad debt
- Bond capacity / when not to bond
- BondMarket gated skeleton + HouseFeeRemit sketch + FeeAttribution ledger landed (P1 live enable still open; Uni skim adapter HouseUniSkim landed) - sPUSD CD sketch + Bonds `#spusd-cd` UI stub landed; Bonds tab Unavailable/Gated stub landed (GATE map; live wire open); House tab **wgV / $PUSD** visual stub on `pusd.html` (wrap-first gate; live pairConfig deploy still open)
- **Banned:** `$PUSD`/USDG AMM/pool/peg-depth product (hurts `$PUSD`). USDG stays BondAssetTag only — see `PUSD_LIQUIDITY.md` / `BONDS.md`

## Market V redeem fence (2026-09-05)

`PusdMarket` / `PusdMarketFed` are **seigniorage**: `swapVToPusd` **burns** V and mints PUSD; `swapPusdToV` **mints** V and burns PUSD.
Fed/gV rebase is an **additional** V printer (staker policy inflate 1-9%). Lithe holds `marketMinter` on Fed V.

## Shared runway + realized remittance (2026-09-05)

One `RemittanceSink` consolidates branch RFV cash; one `RunwayFloor` is enforced **at the sink** on `accountedRfv` (not as dual local pools on Oliver/Lithe). Branches remit **all realized** surplus into that sink; `forwardSurplus` cannot drain below the shared floor. Unpaid accrued interest and depositor principal are **not** exogenous RFV (circular if counted as both RFV and a user claim). Oliver: `pendingReserve`->`realizedReserve` on repay/liq, then remit realized (sole-owner cash OK). Lithe: fee-cash `yieldReserve` remits in full to the same sink. See `RunwayRfv.t.sol` (`test_two_branches_remit_one_sink_floor`).


Tagged remits (UI/TVL "who paid"): wire branches through `FeeAttribution` (House/Lithe/Oliver) before `RemittanceSink`. Direct sink remits remain valid but unattributed. See `EARNINGS_ENGINE.md` + `FeeAttribution.t.sol`.

## Lithe remittance (2026-09-05)

`PusdMarket` (Lithe) remits realized `yieldReserve` to the same `IRemittance` / `RemittanceSink` path as Oliver (`setRemittance` / `remitSurplus`). Floor retain is sink-level (`ITreasuryRfv`); branches do not hold a second local floor. Holder drip (9% APY cap) still runs on accrue; remittance feeds sink -> sPUSD so branch fees can hit the savings path later. See `PUSD_LIQUIDITY.md`: peg remains mint-redeem ~par / social proof. Remittance and sink-held nominal `$PUSD` do **not** invent a USDG depth book (USDG = bond asset only).

## 2026-09-05 - shared savings allocation

RemittanceSink -> SavingsRouter -> SPUSD / SpusdCd now implements the shared surplus split. The router starts disabled, accepts only its configured sink, checks matching assets, and cannot pierce the sink runway floor. Its CD allocation bps is a share of future receipts, not an APY. CD coupon targets and break fees are fixed per position; underfunding is proportional across open targets. Local tests are green; live deploy/address-book/IPC remains open. See [SPUSD.md](SPUSD.md) and [STACK_ECON_REVIEW_2026-09-05.md](STACK_ECON_REVIEW_2026-09-05.md).
