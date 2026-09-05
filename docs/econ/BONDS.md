# Bonds (canon stub)

Relic lock 2026-09-05. Product surface — not Fed plumbing essay.

**User sees:** park an asset → get **gV at a discount** after a wait. That is how RFV shows up for normals.

**One Bonds surface** in the app: asset tabs, plain **discount + wait**. No external chain brand words.

## v1 bond assets (exogenous RFV in)

| Asset | Role | Notes |
|-------|------|-------|
| **ETH** | Major exogenous collateral | Bond in → gV out at discount |
| **USDG** | Chain dollar | Bond in → gV out at discount (not product `$PUSD`) |
| **Major stocks** | Testnet/ops set (e.g. AMZN / TSLA / AMD / NFLX / PLTR where live) | Bond in → gV out at discount; same Bonds UI tab pattern |

`$PUSD` is **not** the headline bond asset — it is spend/mint rail. Bonds are how outside value becomes protocol RFV + discounted gV.

## Product copy (ship)

- Verb: **Bond** (not "stake into treasury essay")
- Show: asset, discount %, vesting / wait, gV you receive
- Hide: RFV battery internals, Fed printer, rebase math

## Out of scope this stub

- No bond contracts yet
- No capacity / OMO schedule yet (see `ROUTING.md` open eng)
- No UI implementation here — frontend owns one Bonds surface + tabs

## Related

- `ROUTING.md` — bonds as RFV inflow
- `HOUSE_PAIR.md` — gV / wgV after bond claim path
- Frontend: one Bonds surface, asset tabs, discount + wait only