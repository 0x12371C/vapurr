# $PUSD liquidity (canon)

Relic lock 2026-09-05. **`$PUSD` stability = social-proof / forced float** (mint-redeem ~par). USDG books are **exit depth**, not the peg story.

## Thesis

Forced `$PUSD` float is trusted when mint/redeem stays near **par** (social proof). **Outside depth** (USDG primary) makes that float usable — it does **not** replace mint-redeem as the peg mechanism. Inside inventory and House equity flow do not substitute for either.

## Books

| Book | Pair | Job |
|------|------|-----|
| **Cash (primary depth)** | **`$PUSD` / USDG** | Exit / spend float vs chain dollar. **Exogenous** depth around the forced float — **not** the peg definition. |
| **Cash (secondary)** | `$PUSD` / ETH (optional) | Extra outside cash leg when useful - not a substitute for USDG depth. |
| **Equity (House)** | **wgV / `$PUSD`** | Equity meets cash (see `HOUSE_PAIR.md`). **Not** peg defense. |

## Hard rules

1. **Peg = mint-redeem ~par / social proof.** Do not narrate USDG depth as what “holds the peg.” Depth supports exit quality around the forced float.
2. **Exogenous depth still matters.** Forced `$PUSD` should feel deep against **outside** stables/cash (**USDG** primary). Do not fake tightness with recursive protocol inventory alone.
3. **House is equity, not peg.** wgV/`$PUSD` can be thick and still leave exit soft if `$PUSD`/USDG is thin. Quote House as the equity market; keep dollar **par** via mint-redeem policy.
4. **Bond inflows thicken RFV.** Bonding ETH / USDG / stocks (`BONDS.md`) grows the Fed/Treasury battery and can seed POL - it does **not** replace mint-redeem ~par or a live `$PUSD`/USDG depth book.
5. **Forced-float trust.** Users judge `$PUSD` by par mint/redeem **and** how cheaply they can enter/exit against USDG — not by how busy House is.
6. **Names.** Ship vapurr-native only (`$PUSD`, USDG, wgV, House). No external brand copy.

## Frontend hint

Cash surfaces show **exogenous depth** (USDG book first). House/equity depth lives on the equity / House surface - never collapse both into one "liquidity" number.

## Pool params (P1 sketch - 2026-09-05)

Proposed Uni v4 defaults for the **primary depth book** `$PUSD` / USDG. **Not deployed. Not live quotes.** House stays `wgV/$PUSD` at 0.30% / ±20% - different job.

| Param | Proposed | Why |
|-------|----------|-----|
| **Fee tier** | **0.05%** (500) | Stable exogenous pair; tighter than House equity desk |
| **Tick spacing** | Fee-tier default (Uni v4 0.05% spacing) | Match fee; no custom weirdness |
| **Active range** | **+/-1% around par (1.0)** | Forced-float trust is near-par exit depth, not wide equity bands |
| **Secondary** | `$PUSD` / ETH optional later | Never a substitute for USDG primary |
| **Seed** | Fed/Treasury RFV + real outside USDG | Never sink-only / recursive `$PUSD` inventory as "depth" |
| **UI posture** | Show book + proposed params as **not live** until address book | Same honesty pattern as Bonds capacity (`-` / not configured) |

Still open after this sketch: deploy scripts, pool addresses in `STATUS.md`, and any hook if pool-held `$PUSD` Lithe drip needs LP allocation (see `HOUSE_PAIR.md` P1 - same cash-leg class).

## Out of scope (for later)

- Deploy scripts and addresses (record in `STATUS.md` when live)
- Contracts beyond naming the books + these param defaults


## Related

- `ROUTING.md` / Lithe: mint-spread `yieldReserve` can remit surplus above runway to sPUSD (branch cash path), separate from exogenous `$PUSD`/USDG depth books

- `ROUTING.md` - Fed/branches map; visible Cash points at exogenous books
- `HOUSE_PAIR.md` - wgV / `$PUSD` House leg
- `BONDS.md` - exogenous RFV in
- `TESTNET_SHAPE.md` - bootstrap context (do not treat House CL alone as peg)
