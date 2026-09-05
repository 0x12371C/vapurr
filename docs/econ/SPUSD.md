# sPUSD (canon)

Pegged **$PUSD** = spend/mint rail (~par redeem for social trust).

**sPUSD** = savings claim on branch remittance (Lithe / House fee carve / Oliver interest) after RFV runway floor.

- **Liquid sPUSD** â€” base yield, on-demand exit (ERC-4626-style).
- **Time-locked tranches** â€” higher yield (CD-shaped); early exit pays break fee into the surplus pool.
- Does **not** receive the 3.5% gV equity rebase.
- Browse earn never mints into sPUSD from V inflation.

## Implementation stub (2026-09-05)

- contracts/SPUSD.sol — liquid ERC-4626-style vault; remittance via eceiveRemittance / creditYield.
- Time-locked CD tranches: **TODO** (break fee into surplus pool).
- Does not receive gV 3.5% rebase.

