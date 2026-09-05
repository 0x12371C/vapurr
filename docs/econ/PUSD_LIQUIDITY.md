# $PUSD liquidity (canon)

Relic lock 2026-09-05. Peg trust needs **tight exogenous books**, not only House vs equity.

## Books

| Book | Pair | Job |
|------|------|-----|
| **Cash (primary)** | **`$PUSD` / USDG** | Tight peg / spend float vs chain dollar. Exogenous depth. |
| **Cash (secondary)** | `$PUSD` / ETH (optional) | Extra outside cash leg when useful — not a substitute for USDG. |
| **Equity (House)** | **wgV / `$PUSD`** | Equity meets cash (see `HOUSE_PAIR.md`). Not the peg book. |

## Hard rules

1. **Exogenous first for peg.** Forced `$PUSD` float must feel deep against **outside** stables/cash (USDG primary). Do not fake tightness with recursive protocol inventory alone.
2. **House is equity, not peg.** wgV/`$PUSD` can be thick and still leave the dollar soft if `$PUSD`/USDG is thin.
3. **No recursive-only RFV.** Bond inflows (ETH / USDG / stocks — `BONDS.md`) and **exogenous LP** both thicken the cash leg. RFV from bonding does not replace a real `$PUSD`/USDG book.
4. **Names.** Ship vapurr-native only (`$PUSD`, USDG, wgV, House). No external brand copy.

## Frontend hint

Cash tab shows **exogenous depth** (USDG book first). House/equity depth lives on the equity surface — do not conflate the two in one "liquidity" number.

## Out of scope this stub

- Pool params / fee / tick ranges
- Deploy scripts and addresses (record in `STATUS.md` when live)
- Contracts beyond naming the books

## Related

- `ROUTING.md` — Fed/branches map
- `HOUSE_PAIR.md` — wgV / `$PUSD` House leg
- `BONDS.md` — exogenous RFV in
- `TESTNET_SHAPE.md` — bootstrap context (do not treat House CL alone as peg)