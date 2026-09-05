# Genesis allocation (HARD LOCK)

Relic lock **2026-09-05**. Replaces any lean / fat-200k-only bootstrap sketch.

**Total mint before `setMinter(gV)` = 1,200,000 V**

| Bucket | Amount | Notes |
|--------|--------|-------|
| **Launch float** | **1,000,000** V | Split below. Not a free AMM dump pile. |
| **DevFund Sablier** | **+ 200,000** V | Extra. Oliver collateral / $PUSD-only (`DEV_FUND.md`). |
| **Total** | **1,200,000** V | Factory + `TestnetRollout` mint this exact amount. |

Cross-refs: `DEV_FUND.md`, `BONDS.md`, `TESTNET_ROLLOUT.md`, `MINT_AUTHORITY.md`, `ROUTING.md`.

## Inside the 1,000,000 launch

| Slice | Amount | Destination |
|-------|--------|-------------|
| **BrowserStream** | **50,000** | 3y treasury earmark (transfer-only drip; never mints) |
| **V/ETH POL** | **80,000** | Hub trading / POL book |
| **V/NVDA POL** | **25,000** | Stock wrapper book at genesis |
| **V/AMD POL** | **25,000** | Stock wrapper book at genesis |
| **House wgV/$PUSD** | **20,000** | Seed earmark (wrap path later: V → gV → wgV). Not AMM dump. |
| **Treasury / ignition remainder** | **800,000** | **Not free float.** Staked as gV (yield + governance), then collateralized in Oliver. $PUSD credit only. `NoMarketSell`. |
| **Total** | **1,000,000** | |

Constants: `contracts/GenesisAllocation.sol`.

## Cutover carve (do not mint legacy on top)

Legacy converter inventory (~**288k** gen-4 `totalSupply`) is **carved from the 800k treasury remainder**.

```
treasuryNet = 800_000 − legacyVSupply     # require legacy ≤ 800k
mint        = 1_200_000                   # never + legacy
converter   = legacyVSupply
```

Example at ~288k gen-4: converter 288k · treasury Oliver/gV 512k · launch earmarks 200k · DevFund 200k = **1.2M**.

## Markets at genesis

| Pair | Role |
|------|------|
| **V/ETH** | Hub book |
| **V/NVDA** | Stock wrapper at genesis |
| **V/AMD** | Stock wrapper at genesis |

USDG is **bond intake only** (`BondAssetTag`). No V/USDG, PUSD/USDG, or USDG cash book.

## Wire

1. `CanonicalLitheFactory` mints `GENESIS_MINT`, funds converter from the 800k remainder, sends `1.2M − legacy` to initiator, then `setMarketMinter(Lithe)` → `setMinter(gV)`.
2. `LaunchBootstrap.fundAndStart()` pulls that remainder and allocates the table above. `GenesisTreasury` stakes V as gV then `collateralizeOliver` (PUSD-only; `withdrawV` reverts `NoMarketSell`).
3. `TestnetRollout` composes the same 1.2M + carve. `BOOTSTRAP_V` env is ignored if it disagrees with `LAUNCH_V`. Dry-run mock legacy = 288k. No silent broadcast.

## Still locked

1. DevFund **200k extra** — Oliver / $PUSD-only; distinct from BrowserStream.
2. BrowserStream **50k** — already-minted 3y earmark.
3. Treasury **800k − carve** — gV + Oliver; not AMM dump.
4. Legacy converter = live gen-4 supply; no invented addresses; **not minted on top**.
5. USDG bond-only. Launch markets = V/ETH + V/NVDA + V/AMD.
6. No silent broadcast — `CONFIRM_TESTNET_DEPLOY` Relic-gated.
