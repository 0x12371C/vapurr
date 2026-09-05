# VAPURR economic routing (canon)

Relic lock 2026-09-05. Living truth for Fed/branches split.

## Institutional map

| Layer | Role | Surfaces |
|-------|------|----------|
| **Fed / Treasury** | Macro reserves, POL, policy inflation | RFV, **bonds** (visible: ETH / USDG / stocks), gV 3.5%/yr rebase, BrowserStream earmark, runway floor |
| **Mint / branches** | Working cash + credit | Lithe + mint-spread ($PUSD), Oliver (secured $PUSD vs gV/V) |
| **Interbank market** | Equity meets cash | House: **wgV / $PUSD** (locked — see HOUSE_PAIR.md; not raw rebasing gV) |
| **Savings** | Slow-growth cash claim | **sPUSD**: liquid base yield; time locks earn more (CD-shaped, break fee) |

## Visible product map (user-facing)

What normals open in the app — not plumbing labels:

| Surface | Tokens / assets | One-liner |
|---------|-----------------|-----------|
| **Cash** | `$PUSD` · `sPUSD` | Spend/mint rail · savings claim after runway |
| **Equity** | `gV` · stake | Bond claim / stake path; Fed 3.5%/yr to stakers only |
| **Bonds** | **ETH · USDG · major stocks** | Park asset -> get **gV at a discount** after a wait. Exogenous RFV in. |
| **House** | **wgV / $PUSD** | Equity meets cash. Wrap gV -> wgV before LP. |

Bonds are a **first-class visible surface** (see `BONDS.md` + `vapurr://bonds`). They are not a hidden OMO footnote.

## Hard walls

1. **Only Fed prints V** — flat **3.5%/yr rebase to gV stakers**. Browse never funded by this mint.
2. **BrowserStream** — **50k $VAPURR / 3y** from **already-minted treasury** (float migration). No USD cap (intentional convexity). Claim: install_id + KYC.
3. **$PUSD** — forced product float; mint/redeem ~**par** for social trust. Not an equity lottery.
4. **Branch remittance** — Lithe/House/Oliver surplus -> runway floor first -> **sPUSD** (not into gV rebase).
5. **404 != payments** — pay is HTTP 402 / x402.

## Inflows -> RFV battery

- House fees, Lithe + mint-spread, Oliver interest/liq surplus
- **Bonds (visible):** ETH / USDG / major stocks -> treasury cash / POL (see `BONDS.md`)

## Outflows

- BrowserStream (treasury V earmark)
- Post-stream $PUSD browse only from **surplus** above runway (later)
- gV rebase (policy mint to stakers only)
- sPUSD yield (cash surplus to savers)

## Open eng choices

- House leg: **wgV locked** (wstETH pattern) — see HOUSE_PAIR.md. Do not pair raw rebasing gV in AMM.
- Fed LOLR policy for Oliver bad debt
- Bond capacity / when not to bond
- Full bond contracts (P1) · sPUSD CD tranches (P1)

## Market V redeem fence (2026-09-05)

`PusdMarket.swapPusdToV` redeems from **pre-funded / locked inventory only** (no `vapurr.mint`).
`swapVToPusd` locks V into the market instead of burning. Fed/gV rebase is the sole V printer.
