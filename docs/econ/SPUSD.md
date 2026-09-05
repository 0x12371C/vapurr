# sPUSD (canon)

Pegged **$PUSD** = spend/mint rail (~par redeem for social trust).

**sPUSD** = savings claim on branch remittance (Lithe / House fee carve / Oliver interest) after RFV runway floor.

- **Liquid sPUSD** â€” base yield, on-demand exit (ERC-4626-style).
- **Time-locked CD tranches** â€” higher yield share of the same surplus pool; early exit pays a **break fee** back into the surplus (boosts remaining savers / RFV path).
- Does **not** receive the 3.5% gV equity rebase.
- Browse earn never mints into sPUSD from V inflation.

## Implementation (2026-09-05)

- `contracts/SPUSD.sol` â€” liquid ERC-4626-style vault; remittance via `receiveRemittance` / `creditYield`; donation guards (virtual + dead shares + min deposit).
- `contracts/SpusdCd.sol` â€” time-lock CD sketch (see below). Proofs: `contracts/test/SpusdCd.t.sol`.
- Does not receive gV 3.5% rebase.

## CD sketch (product + eng)

One surplus pool feeds both liquid sPUSD NAV and CD coupon. CDs are **not** a second money printer.

| Field | v1 sketch |
|-------|-----------|
| Principal | $PUSD locked in `SpusdCd` (or shares escrowed from liquid sPUSD) |
| Term | Fixed seconds (`term`); ops sets allowed terms (e.g. 30d / 90d) |
| Coupon | Fixed `couponBps` of principal paid from surplus at maturity (not from V mint) |
| Early exit | Allowed; `breakFeeBps` of principal â†’ surplus / remittance sink; remainder returned to user |
| At maturity | Principal + coupon â†’ user; unpaid coupon stays in surplus if underfunded (no silent mint) |

### Invariants

1. **No V inflation into CD** â€” coupon is surplus-funded only (`creditSurplus` / remittance). Underfunded maturity pays principal first, then pro-rata coupon, never mints $PUSD.
2. **Break fee is RFV-positive** â€” early-exit fee stays in the savings/surplus path (same family as remittance), not burned to nowhere and not paid as equity rebase.
3. **Liquid vs CD** â€” liquid sPUSD stays on-demand; CD is the higher-yield locked tranche. Same $PUSD asset; different exit rules.
4. **Sink order unchanged** â€” branch realized surplus â†’ `RemittanceSink` (runway floor) â†’ sPUSD / CD surplus credit. CD does not bypass the floor.

### Out of scope this sketch

- House fee carve sketch: `HouseFeeRemit.sol` (creditFees -> RemittanceSink). Live Uni v4 skim hook still open.
- Live deploy / wired deposit+claim UI (visual stub on `vapurr://bonds` `#spusd-cd` landed 2026-09-05).
- Variable floating CD rates.

