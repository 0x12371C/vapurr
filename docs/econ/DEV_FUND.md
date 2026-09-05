# Developer fund stream (canon)

Relic lock 2026-09-05 (HARD LOCK: Oliver collateral / $PUSD-only).

## What it is

Genesis allocation of **200_000 $VAPURR** into a **Sablier-style linear lockup** (`DevFundStream`).

Unlocked V is **never** paid to a wallet or AMM. It **automatically locks as Oliver collateral**; the recipient may **only draw $PUSD**.

| | DevFundStream | BrowserStream |
|--|---------------|---------------|
| Amount | **200_000** V | **50_000** V |
| Duration (base) | **4 years** | **3 years** |
| Funding | **Genesis mint** (before `setMinter(gV)`) | Already-minted **treasury float** |
| Claim asset | **$PUSD only** (Oliver borrow) | `$VAPURR` drip to browse distributor |
| V custody | Stream -> **Oliver `collatV[stream]`** | Stream inventory -> drip transfer |
| Market sell | **Forbidden** (`NoMarketSell`) | N/A (browse earmark) |

Hard walls:

1. DevFund **never** holds a V minter role. After cutover, **gV** is policy minter and **Lithe** is marketMinter (seigniorage).
2. **No V transfer** to recipient / open market / AMM. `withdrawV` / `claimV` revert `NoMarketSell`.
3. `settle()` is the only unlock sink: `depositV` into immutable Oliver.
4. `drawPusd` borrows against stream-owned collateral and forwards **$PUSD** to soulbound `recipient` (frozen at `startStream`).

## Formula (expansion slows remaining unlock)

State at `startStream()`:

- `deposited` ? V pulled into the stream (expect `GENESIS_AMOUNT = 200_000e18`)
- `startSupply` ? `vapurr.totalSupply()` at start
- `BASE_DURATION` ? `4 * 365 days`

```
expansionWad = max(1e18, totalSupply * 1e18 / startSupply)

unlockPerSecond = deposited * 1e18 / (BASE_DURATION * expansionWad)

pending = min(deposited - accrued, unlockPerSecond * dt)
```

Then:

```
unsettleable = vested - lockedInOliver
settle() -> oliver.depositV(unsettleable)   # collatV[stream] += ...
drawPusd(x) -> settle(); oliver.borrow(x); pusd.transfer(recipient, x)
```

Properties:

1. **Flat supply**: full unlock accrual in `BASE_DURATION` (linear).
2. **Supply expands**: unlock rate slows by `startSupply/totalSupply` (remaining stretches).
3. **Accrual is path-dependent** on expansion at each tip (Sablier-compatible rate adjust).
4. Accrued V becomes Oliver collateral; recipient liquidity is **$PUSD credit**, not V inventory.

## Cutover wiring

1. `CanonicalLitheFactory` genesis-mints **exactly 1.2M** (1M launch + 200k DevFund). Legacy converter is carved from the 800k treasury remainder (not minted on top). Then `setMarketMinter(Lithe)` → `setMinter(gV)`.
2. `LaunchBootstrap` allocates the 1M launch (BrowserStream 50k / POL 80+25+25 / House 20k / treasury remainder → `GenesisTreasury` gV+Oliver) and funds DevFund 200k into Oliver.

## Status

- **Source-landed:** `DevFundStream.sol`, `LaunchBootstrap.sol`, proofs in `DevFundStream.t.sol` / `LaunchBootstrap.t.sol`.
- **Not live** on 46630 until approved CutoverDeploy. UI honest-empty until addresses exist.

## Distinct from the 1M launch pile

DevFund **200k is extra** — not taken from the 1M launch float. The 1M is BrowserStream 50k / V/ETH 80k / V/NVDA 25k / V/AMD 25k / House 20k / treasury 800k (gV + Oliver, not AMM dump). Canon: `GENESIS_ALLOCATION.md`.

## Related

- `GENESIS_ALLOCATION.md` — 1.2M mint (1M launch + 200k DevFund)
- `ROUTING.md` ? Fed outflows / BrowserStream wall
- `BONDS.md` ? exogenous bond intake vs POL trading books
- `EARNINGS_ENGINE.md` ? BrowserStream global 50k budget
- Oliver: `PusdLoop.sol` (`depositV` / `borrow` / no DevFund `withdrawV` path)
