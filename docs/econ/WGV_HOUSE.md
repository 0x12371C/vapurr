# wgV House notes (operator)

Relic lock 2026-09-05. Short checklist for House equity leg. Pair canon stays in `HOUSE_PAIR.md`.

## One rule

House AMM / LP quotes **wgVAPURR / $PUSD** only. Raw rebasing **gVAPURR** is never a pool currency.

## Wrap path (ops)

1. Stake `$VAPURR` to `gVAPURR` (Fed dynamic 1–9%/yr index from bond util; policy-only; see `POLICY_RATE.md`).
2. Wrap `gVAPURR` to `wgVAPURR` before any House seed / LP / swap.
3. Unwrap `wgVAPURR` to `gVAPURR` (more gV after rebase), then unstake if needed.

Bootstrap that seeds raw `$VAPURR` or market.vapurr into House is wrong. Wrap first.

## Enforce in code

| Check | Where |
|-------|-------|
| `requireHousePair` / `requireHouseEquity` | `HousePairConfig.sol` (+ factory mark) |
| Equity/cash immutables from config | `HouseLp.sol` / `HouseSwap.sol` constructors |
| Guard proofs | `HousePairGuard.t.sol`, `HouseLpWiring.t.sol`, `HouseSwapWiring.t.sol` |

Revert class: `RawGvNotHouseEquity` if either Uni currency is raw gV.

## Green vs open (2026-09-05)

**Green (sketch / in-tree):**

- PairConfig walls + HouseLp/HouseSwap equity = wgV
- HouseFeeRemit (fee carve -> RemittanceSink)
- HouseUniSkim (authorized skim -> creditFees)
- BrowserStream / browse never call gV rebase mint

**Open (needs Relic go):**

- Live Uni v4 deploy: HousePairConfig address into HouseLp/HouseSwap; Rust `house_deploy` / `swap_deploy` ABI encodes `pairConfig` first
- PositionManager + Permit2 approvals for wgV; PoolManager unlock/settle e2e
- Full Uni v4 `IHooks` / swapper integration beyond HouseUniSkim inventory bridge
- Pool-held `$PUSD` Lithe-index rebase allocation to LPs (**P1** — wgV fixes equity leg only)

## Do not

- Pair raw gV or raw `$VAPURR` as House equity once staking is live
- Treat House volume as `$PUSD` peg depth (peg books are `$PUSD`/USDG — see `PUSD_LIQUIDITY.md`)
- Fund browse earn from gV rebase mint (BrowserStream = treasury earmark only)

## UI visual stub (2026-09-05)

`frontend/pusd.html` House tab now labels the book **wgV / $PUSD** with an explicit wrap-first gate and fee-skim to RemittanceSink note. Live `econ-house-*` cmds unchanged; backend still surfaces `house.vapurr` until deploy encodes `pairConfig` / wgV inventory.

## Pointers

- Pair canon: `HOUSE_PAIR.md`
- Routing map: `ROUTING.md`
- Fee path: `HouseFeeRemit.sol`, `HouseUniSkim.sol`
