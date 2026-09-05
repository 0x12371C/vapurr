# $PUSD liquidity (canon)

Relic lock 2026-09-05. **`$PUSD` stability = social-proof / forced float** (mint-redeem ~par). **Not** USDG depth.

## Thesis

Forced `$PUSD` float is trusted when mint/redeem stays near **par** (social proof). That is the whole peg story.

**USDG** is accepted **only** as a Fed **treasury bond** asset: exogenous RFV in -> **gV** out (`BONDS.md`, `BondAssetTag`). Doing anything else with USDG - `$PUSD`/USDG pools, peg-depth books, cash-depth stubs, competing stable markets - **hurts `$PUSD`** and must not ship.

Inside inventory and House equity flow do not substitute for mint-redeem ~par.

## Books (what ships)

| Book | Pair | Job |
|------|------|-----|
| **Cash (product)** | **`$PUSD` / `sPUSD`** | Spend/mint rail / savings after runway. Peg trust = mint-redeem ~par. |
| **Equity (House)** | **wgV / `$PUSD`** | Equity meets cash (see `HOUSE_PAIR.md`). **Not** peg defense. |
| **Bonds (RFV in)** | **ETH / USDG / stocks** | Outside assets -> discounted gV. USDG here is **BondAssetTag only**. |

**Do not ship:** `$PUSD`/USDG AMM or Uni depth books, peg-depth product surfaces, cash-depth stubs that promote USDG as exit/peg liquidity, or any competing USDG<->`$PUSD` stable market.

## Hard rules

1. **Peg = mint-redeem ~par / social proof.** Never narrate USDG depth, books, or pools as what "holds the peg."
2. **USDG = bond asset only.** Bond USDG into Fed/Treasury RFV for gV at a discount. No `$PUSD`/USDG pool params, deploy plans, or UI that sells USDG depth as product.
3. **House is equity, not peg.** Quote House as the equity market; keep dollar **par** via mint-redeem policy.
4. **Bond inflows thicken RFV.** Bonding ETH / USDG / stocks (`BONDS.md`) grows the Fed/Treasury battery - it does **not** create a `$PUSD`/USDG depth book and must not be framed that way.
5. **Forced-float trust.** Users judge `$PUSD` by par mint/redeem - not by USDG pool depth or how busy House is.
6. **Names.** Ship vapurr-native only (`$PUSD`, USDG-as-bond-tag, wgV, House). No external brand copy.

## Frontend hint

Cash surfaces: **`$PUSD` / `sPUSD`** honesty. Bonds surface may show USDG as a **bond asset tab**. Never promote a USDG depth / peg-pool book. Honest empty on cash-depth stubs is OK. House/equity depth lives on the equity / House surface - never collapse both into one "liquidity" number.

## Out of scope (banned product)

- `$PUSD` / USDG Uni v4 (or any AMM) pool params, fee tiers, tick ranges, seed plans, deploy scripts
- Cash-depth UI that frames USDG as primary exit / peg liquidity
- Treating sink-held nominal `$PUSD` or recursive inventory as "outside USDG depth"

## Related

- `ROUTING.md` / Lithe: mint-spread `yieldReserve` can remit surplus above runway to sPUSD (branch cash path). Remittance does not invent a USDG depth book; peg remains mint-redeem ~par.
- `HOUSE_PAIR.md` - wgV / `$PUSD` House leg
- `BONDS.md` - USDG as BondAssetTag (exogenous RFV in)
- `TESTNET_SHAPE.md` - bootstrap context (do not treat House CL alone as peg)
