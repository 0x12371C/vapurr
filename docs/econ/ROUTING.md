# VAPURR economic routing (canon)

Relic lock 2026-09-05. Living truth for Fed/branches split.

## Institutional map

| Layer | Role | Surfaces |
|-------|------|----------|
| **Fed / Treasury** | Macro reserves, POL, policy inflation | RFV, bonds (OMO), gV 3.5%/yr rebase, BrowserStream earmark, runway floor |
| **Mint / branches** | Working cash + credit | Lithe + mint-spread ($PUSD), Oliver (secured $PUSD vs gV/V) |
| **Interbank market** | Equity meets cash | House: **wgV / $PUSD** (locked ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â see HOUSE_PAIR.md; not raw rebasing gV) |
| **Savings** | Slow-growth cash claim | **sPUSD**: liquid base yield; time locks earn more (CD-shaped, break fee) |

## Hard walls

1. **Only Fed prints V** ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â flat **3.5%/yr rebase to gV stakers**. Browse never funded by this mint.
2. **BrowserStream** ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â **50k $VAPURR / 3y** from **already-minted treasury** (float migration). No USD cap (intentional convexity). Claim: install_id + KYC.
3. **$PUSD** ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â forced product float; mint/redeem ~**par** for social trust. Not an equity lottery.
4. **Branch remittance** ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â Lithe/House/Oliver surplus ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ runway floor first ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ **sPUSD** (not into gV rebase).
5. **404 ÃƒÂ¢Ã¢â‚¬Â°Ã‚Â  payments** ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â pay is HTTP 402 / x402.

## Inflows ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ RFV battery

- House fees, Lithe + mint-spread, Oliver interest/liq surplus, bonds (ETH / USDG / major stocks - see BONDS.md) -> treasury cash / POL.

## Outflows

- BrowserStream (treasury V earmark)
- Post-stream $PUSD browse only from **surplus** above runway (later)
- gV rebase (policy mint to stakers only)
- sPUSD yield (cash surplus to savers)

## Open eng choices

- House leg: **wgV locked** (wstETH pattern) ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â see HOUSE_PAIR.md. Do not pair raw rebasing gV in AMM.
- Fed LOLR policy for Oliver bad debt
- Bond capacity / when not to bond

## Market V redeem fence (2026-09-05)

`PusdMarket.swapPusdToV` redeems from **pre-funded / locked inventory only** (no `vapurr.mint`).
`swapVToPusd` locks V into the market instead of burning. Fed/gV rebase is the sole V printer.
