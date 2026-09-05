# swapPusdToV - V out semantics (canon)

Relic lock 2026-09-05. Fence target. Vapurr-native names only.

Cross-refs: `ROUTING.md` wall 1, `GvFed.sol` (`gVAPURR.rebase` + `BrowserStream`), `HOUSE_PAIR.md`, `RENAME_FLAGS.md`.

## Decision

**Inventory unwrap - not Fed mint.**

`$PUSD` to `$VAPURR` redeem releases **already-extant V** from market inventory. It must **never** call `vapurr.mint`. Sole V inflation: Fed policy via `gVAPURR.rebase`. Browse/earn: `BrowserStream` transfers only.

## Mechanics

| Path | Must do | Must not |
|------|---------|----------|
| `swapVToPusd` | Lock trader V into **market inventory**, mint `$PUSD` minus spread to Lithe | Destroy inventory needed for redeem (default: **no burn on this rail**) |
| `swapPusdToV` | Burn `$PUSD`, **transfer** `ask` V from inventory | Call `vapurr.mint` / Fed rebase / BrowserStream |
| Inventory empty | Revert | Emergency mint |
| Seed | Genesis / treasury pre-fund with already-minted V | Seed via on-redeem mint |

Oracle / pool symbols: `vapurrRate`, `poolDelta`, `stablePool` / `vapurrPool` in curve math.

## Tests

1. `swapPusdToV` does not increase `vapurr.totalSupply`.
2. Round-trip conserves supply (default: no sinks).
3. Empty inventory reverts.
4. `gV.rebase` still increases supply; BrowserStream `drip` does not.

## STATUS one-liner

`swapPusdToV` = inventory unwrap of extant `$VAPURR`; Fed/`gV.rebase` is the only V print.