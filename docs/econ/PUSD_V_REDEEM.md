# swapPusdToV / swapVToPusd — seigniorage (canon)

Relic lock 2026-09-05 (seigniorage rewrite). Fence target. Vapurr-native names only.

Cross-refs: `ROUTING.md`, `MINT_AUTHORITY.md`, `GvFed.sol`, `PusdMarketFed.sol`.

## Decision

**Terra-style seigniorage — not seigniorage redeem (mint V).**

| Path | Must do | Must not |
|------|---------|----------|
| `swapVToPusd` (expand) | **Burn** trader V, mint `$PUSD` minus spread to Lithe reserve | Lock V into redeem inventory |
| `swapPusdToV` (contract) | Burn `$PUSD`, **mint** `ask` V to trader | Pay from pre-funded inventory / INV gate |
| Lithe on Fed V | Hold `marketMinter` | Steal policy minter from gV |
| gV rebase | Additional inflate to stakers (1-9% bond dial) | Claim to be policy V printer (Lithe seigniorage also prints on redeem) |
| BrowserStream | Transfer already-minted float | Mint or setMinter |

## Tests

1. `swapVToPusd` decreases `vapurr.totalSupply` by offer.
2. `swapPusdToV` increases `vapurr.totalSupply` by ask.
3. Empty market balance does **not** block redeem (mints).
4. `gV.rebase` still increases supply; BrowserStream `drip` does not.

## STATUS one-liner

`swapVToPusd` burns `$VAPURR` / mints `$PUSD`; `swapPusdToV` burns `$PUSD` / mints `$VAPURR`. gV policy rebase is an additional printer.
