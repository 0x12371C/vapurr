# Genesis allocation (locked working default)

Relic / vapurrbot lock **2026-09-05** (fatter bootstrap — not lean 140k).
Allocation widget skipped; this file is the working default until Relic changes it.

Cross-refs: `DEV_FUND.md`, `BONDS.md`, `TESTNET_ROLLOUT.md`, `ROUTING.md`, `EARNINGS_ENGINE.md`.

## Launch markets (genesis POL / trading books)

| Pair | Role |
|------|------|
| **V/ETH** | Hub book |
| **V/NVDA** | Stock wrapper at genesis |
| **V/AMD** | Stock wrapper at genesis |

Registered via `LaunchBootstrap` → `ExogenousPairRegistry`. **Not** bond purchase (see `BONDS.md`).

**Banned:** V/USDG, PUSD/USDG, any USDG cash book. USDG remains **bond intake only** (`BondAssetTag`).

## Genesis mint buckets (before `setMinter(gV)`)

Factory / `TestnetRollout` mint shape:

| Bucket | Amount | Notes |
|--------|--------|-------|
| **DevFund** | **200_000** ether | Separate from bootstrap. → `DevFundStream` → Oliver collateral → **$PUSD-only** draw (`NoMarketSell`). See `DEV_FUND.md`. |
| **bootstrapV** | **200_000** ether | Working default (fatter). Split below. Env: `BOOTSTRAP_V`. |
| **Legacy converter** | live gen-4 `totalSupply` | `LEGACY_V_SUPPLY` on live path (~**288k** at last check). 1:1 converter inventory. Dry-run mocks if unset. |

Total new canonical float at cutover ≈ DevFund + bootstrapV + converter inventory (legacy supply).

## bootstrapV = 200_000 ether (split)

| Slice | Amount | Destination |
|-------|--------|-------------|
| **BrowserStream** | **50_000** | Already-minted treasury earmark (50k / 3y drip; never mint). Taken **from** this bootstrap float. |
| **V/ETH POL** | **80_000** | Hub trading / POL depth |
| **V/NVDA** | **25_000** | Stock book seed |
| **V/AMD** | **25_000** | Stock book seed |
| **House wgV/$PUSD** | **20_000** | House seed (wrap path: stake → gV → wgV before pool seed; see `HOUSE_PAIR.md` / `WGV_HOUSE.md`) |
| **Total** | **200_000** | |

Script: `contracts/script/TestnetRollout.s.sol` — `vm.envOr("BOOTSTRAP_V", uint256(200_000 ether))`.
Unset env → **200_000 ether**. Override only for experiments; do not treat lean 140k as current default.

## Still locked (do not reopen without Relic)

1. **DevFund 200k** — Oliver collateral / $PUSD-only; distinct from BrowserStream.
2. **BrowserStream 50k** — from bootstrap earmark; transfer-only; global 3y budget.
3. **Legacy converter** = live gen-4 supply at cutover (~288k last check); no invented addresses.
4. **USDG bond-only** — no V/USDG or PUSD/USDG cash books.
5. **Launch markets** = V/ETH + V/NVDA + V/AMD only at genesis.
6. **No silent broadcast** — `CONFIRM_TESTNET_DEPLOY` Relic-gated.

## Related

- `TESTNET_ROLLOUT.md` — ordered deploy; `BOOTSTRAP_V` default **200000 ether**
- `DEV_FUND.md` — 200k stream walls
- `BONDS.md` — exogenous POL books vs Open Bond; USDG bond-only
- `MINT_AUTHORITY.md` — genesis → `setMarketMinter(Lithe)` → `setMinter(gV)`