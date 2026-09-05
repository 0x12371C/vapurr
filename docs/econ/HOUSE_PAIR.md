# House pair (canon)

Relic lock 2026-09-05.

## Pair

**House AMM leg = `wgVAPURR` / `$PUSD`**, not raw rebasing `gVAPURR`.

| Leg | Token | Why |
|-----|-------|-----|
| Equity | **wgVAPURR** | Non-rebasing shares (wstETH pattern). Rebase accrues in exchange rate, not balance — LP / CL math stays honest. |
| Cash | **$PUSD** | Product dollar (Lithe index separate). |

## Do not pair

- Raw **gVAPURR** in Uni v4 / CPMM — index rebase soft-taxes the pool (balance drift without swaps).
- Raw **$VAPURR** as the house equity leg once staking is live — stakers receive the 3.5%/yr; house should quote claim-wrapped gV.

## Wrap path

1. Stake `$VAPURR` → `gVAPURR` (index rebase, Fed policy only).
2. Wrap `gVAPURR` → `wgVAPURR` for AMM / LP.
3. Unwrap `wgVAPURR` → `gVAPURR` (more gV after rebase) → unstake to `$VAPURR`.

## Hard wall (reminder)

BrowserStream / browse earn **never** call gV rebase mint. Stream pays from **already-minted treasury** earmark only. See `ROUTING.md`.
