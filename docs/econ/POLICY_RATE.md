# Fed gV policy rate (canon)

Relic lock 2026-09-05 (corrected direction).

## Lock

- Fed gV annualized rebase rate is **dynamic from the bond market**, not a flat 3.5%.
- **Floor 1%/yr**, **ceiling 9%/yr** (bps 100–900; wad-year equivalents `[1e16, 9e16]` if expressed as wad).
- Still the **sole V inflation** path, and **only to gV stakers**.
- **BrowserStream** unchanged: treasury float transfers only — no mint.

## Direction (binding)

| Bond book | Policy rate |
|-----------|-------------|
| **Hot offtake / high capacity utilization** | **Down toward 1%/yr** (suppress V print) |
| **Cold bond book / low utilization** | **Up toward 9%/yr** |
| **Unbound BondMarket or empty signal** | **~3.5%/yr mid default** |

Do **not** ship hot→9%. Hot offtake suppresses inflation.

## Signal

From `BondMarket`:

- `lifetimeCreditedRfv` — cumulative credited RFV from successful `bond()` calls
- `remainingCapacity()` — sum of per-tab remaining capacity
- `capacityUtilizationWad = credited / (credited + remaining)` (0 = cold, 1e18 = fully offtaken)
- `hasBondBookSignal()` — false when both credited and remaining are zero (idle / unconfigured)

## Formula (`RebasePolicy.policyRateBps`)

```
MIN = 100 bps (1%/yr)
MAX = 900 bps (9%/yr)
MID = 350 bps (3.5%/yr)   // unbound or !hasBondBookSignal

if unbound or !hasBondBookSignal:
  rate = MID
else:
  util = capacityUtilizationWad          // [0, 1e18]
  rate = MAX - util * (MAX - MIN) / 1e18
  // util=0  => 900
  // util=1e18 => 100
  clamp to [MIN, MAX]
```

`gVAPURR.accrue()` / `rebase()` mints `(supply * currentRebaseBps() * dt) / 10_000 / YEAR` (linear in time per interval; repeated settlements compound slightly).

## Assumption (Relic may correct)

Stronger bond offtake / higher capacity utilization → **lower** policy rate toward 1%. Cold book → toward 9%. Neutral unbound default stays ~3.5%.

## Related

- `BONDS.md` — bond surface + RFV
- `ROUTING.md` — Fed wall / product map
- Contracts: `GvFed.sol` (`RebasePolicy`, `gVAPURR`), `BondMarket.sol` (utilization signal)
- Proofs: `GvBoundariesTest` (min / mid / max), `BondMarketTest.test_capacity_utilization_signal`
