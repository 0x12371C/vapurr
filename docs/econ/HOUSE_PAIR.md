# House pair (canon)

Relic lock 2026-09-05.

## Pair

**House AMM leg = `wgVAPURR` / `$PUSD`**, not raw rebasing `gVAPURR`.

| Leg | Token | Why |
|-----|-------|-----|
| Equity | **wgVAPURR** | Non-rebasing shares (wstETH pattern). Rebase accrues in exchange rate, not balance â€” LP / CL math stays honest. |
| Cash | **$PUSD** | Product dollar (Lithe index separate). |

## Hard invariant (enforce)

1. **Raw `gVAPURR` is never a House pool currency.** Uni v4 / CPMM `PoolKey` currencies must be exactly `{wgV, $PUSD}` (either order).
2. Deploy / factory path MUST call `HousePairConfig.requireHousePair` (or `HousePairFactory.validateAndMark`) before `initializePool` / seed. Revert: `RawGvNotHouseEquity` if either currency is gV.
3. Equity leg check: `requireHouseEquity(token)` â€” only `wgV` passes; raw gV and raw `$VAPURR` fail.
4. Wrapping gV â†’ wgV fixes **only the equity leg**. It does **not** fix cash-leg rebase accounting (see PUSD note below).

Code SoT: `contracts/HousePairConfig.sol` (+ thin `HousePairFactory`). Proofs: `contracts/test/HousePairGuard.t.sol`.

## Do not pair

- Raw **gVAPURR** in Uni v4 / CPMM â€” index rebase soft-taxes the pool (balance drift without swaps).
- Raw **$VAPURR** as the house equity leg once staking is live â€” stakers receive the 3.5%/yr; house should quote claim-wrapped gV.

## Wrap path

1. Stake `$VAPURR` â†’ `gVAPURR` (index rebase, Fed policy only).
2. Wrap `gVAPURR` â†’ `wgVAPURR` for AMM / LP.
3. Unwrap `wgVAPURR` â†’ `gVAPURR` (more gV after rebase) â†’ unstake to `$VAPURR`.

## $PUSD rebase note (P1)

Naked `$PUSD` **is** rebasing in this stack (`PusdToken`: shares Ã— Lithe index; `drip` lifts index). **sPUSD** is the savings vault (non-rebasing shares claiming rebasing PUSD) â€” not the House cash leg.

Pool-held `$PUSD` therefore accrues Lithe drip to whoever holds the pool balance. Ordinary Uni v4 reserve math does **not** allocate that gain to LPs correctly without an explicit hook / accounting path. Treat **pool-held PUSD rebase accounting** as **P1** (same class as the CODEX attack note: wgV fixes one leg only).

## Live wiring gap

`HouseLp` / `HouseSwap` constructors now take `HousePairConfig` and set equity/cash immutables from `config.wgV()` / `config.pusd()` (not `market.vapurr()`). `seed` / `unlockCallback` call `requireHousePair` before PositionManager / PoolManager work.

Still open for **live** Uni v4:
- Deploy `HousePairConfig` + pass its address into HouseLp/HouseSwap (Rust `house_deploy` / `swap_deploy` ABI must encode `pairConfig` first).
- Bootstrap must seed **wgV** inventory (wrap gV first), not raw `$VAPURR` / market.vapurr balances.
- PositionManager + Permit2 approvals for wgV on the live book; PoolManager unlock/settle e2e fork proofs.
- Pool-held `$PUSD` Lithe-index rebase settlement / LP allocation remains **P1** (settle hair is interim only).

## Hard wall (reminder)

BrowserStream / browse earn **never** call gV rebase mint. Stream pays from **already-minted treasury** earmark only. See `ROUTING.md`.

## House fee carve -> remittance (2026-09-05 sketch)

Uni v4 LP fees stay with LPs. Protocol carve (ops / hook / swapper skim) lands as **realized $PUSD inventory** in contracts/HouseFeeRemit.sol, then emitSurplus -> RemittanceSink (sink-level runway floor). No second local floor on House.

- creditFees pulls inventory only (never mints).
- emitSurplus requires a wired sink; empty reserve reverts TINY.
- Proofs: contracts/test/HouseFeeRemit.t.sol (credit+remit, empty, partial, unset sink).
- Still open: live Uni v4 hook / swapper skim wiring into creditFees; deploy + Rust bootstrap.
