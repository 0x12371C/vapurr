# House pair (canon)

Relic lock 2026-09-05.

## Pair

**House AMM leg = `wgVAPURR` / `$PUSD`**, not raw rebasing `gVAPURR`.

| Leg | Token | Why |
|-----|-------|-----|
| Equity | **wgVAPURR** | Non-rebasing shares (wstETH pattern). Rebase accrues in exchange rate, not balance — LP / CL math stays honest. |
| Cash | **$PUSD** | Product dollar (Lithe index separate). |

## Hard invariant (enforce)

1. **Raw `gVAPURR` is never a House pool currency.** Uni v4 / CPMM `PoolKey` currencies must be exactly `{wgV, $PUSD}` (either order).
2. Deploy / factory path MUST call `HousePairConfig.requireHousePair` (or `HousePairFactory.validateAndMark`) before `initializePool` / seed. Revert: `RawGvNotHouseEquity` if either currency is gV.
3. Equity leg check: `requireHouseEquity(token)` — only `wgV` passes; raw gV and raw `$VAPURR` fail.
4. Wrapping gV → wgV fixes **only the equity leg**. It does **not** fix cash-leg rebase accounting (see PUSD note below).

Code SoT: `contracts/HousePairConfig.sol` (+ thin `HousePairFactory`). Proofs: `contracts/test/HousePairGuard.t.sol`.

## Do not pair

- Raw **gVAPURR** in Uni v4 / CPMM — index rebase soft-taxes the pool (balance drift without swaps).
- Raw **$VAPURR** as the house equity leg once staking is live — stakers receive the 3.5%/yr; house should quote claim-wrapped gV.

## Wrap path

1. Stake `$VAPURR` → `gVAPURR` (index rebase, Fed policy only).
2. Wrap `gVAPURR` → `wgVAPURR` for AMM / LP.
3. Unwrap `wgVAPURR` → `gVAPURR` (more gV after rebase) → unstake to `$VAPURR`.

## $PUSD rebase note (P1)

Naked `$PUSD` **is** rebasing in this stack (`PusdToken`: shares × Lithe index; `drip` lifts index). **sPUSD** is the savings vault (non-rebasing shares claiming rebasing PUSD) — not the House cash leg.

Pool-held `$PUSD` therefore accrues Lithe drip to whoever holds the pool balance. Ordinary Uni v4 reserve math does **not** allocate that gain to LPs correctly without an explicit hook / accounting path. Treat **pool-held PUSD rebase accounting** as **P1** (same class as the CODEX attack note: wgV fixes one leg only).

## Live wiring gap

`HouseLp` / `HouseSwap` still read `market.vapurr()` + `market.pusd()` and build PoolKeys from those. That is **not** yet the locked wgV/$PUSD pair. Full Uni v4 rewire (equity = wgV address, call PairConfig before init, settle hair for rebasing PUSD) remains open — PairConfig is the gate to land now.

## Hard wall (reminder)

BrowserStream / browse earn **never** call gV rebase mint. Stream pays from **already-minted treasury** earmark only. See `ROUTING.md`.
