# Terra Classic market — forked here

Not from memory. Pulled 2026-09-03 from:

- https://github.com/terra-money/classic-core/blob/main/x/market/keeper/swap.go
- https://github.com/terra-money/classic-core/blob/main/x/market/keeper/msg_server.go
- https://github.com/terra-money/classic-core/blob/main/x/market/types/params.go
- https://github.com/terra-money/classic-core/blob/main/x/market/abci.go
- https://github.com/terra-money/classic-core/blob/main/x/oracle/keeper/keeper.go (`GetLunaExchangeRate`)
- https://github.com/classic-terra/core/blob/main/x/market/keeper/keeper.go (`ReplenishPools`)

EVM port is `../PusdMarket.sol`.

Map:

| Terra | vapurr |
|---|---|
| Luna | $VAPURR |
| UST (`uusd`) | $PUSD |
| MsgSwap | `swapLunaToUst` / `swapUstToLuna` |
| `GetLunaExchangeRate(uusd)` | `lunaRate` (first spot of the block) |
| `GetLunaExchangeRate(uluna)` | `1e18` |
| SDR basket | collapsed to UST (one stable) |
| MinStabilitySpread | 2% (`params.go` DefaultMinStabilitySpread) |
| BasePool | 1_000_000e18 |
| PoolRecoveryPeriod | 14_400 |

No USDG in the mint/burn loop. Burn V ↔ mint P at the oracle. Spread funds **Lithe** (9%).
