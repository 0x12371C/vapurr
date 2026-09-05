# Rename flags - banned external names

Relic standing rule 2026-09-05: no Luna / UST / Olympus (or lookalike) in vapurr builds.

## Status on `fix/pusdloop-routing-gaps`

Scrub applied (symbols + comments + docs):

| Was | Now |
|-----|-----|
| `swapUstToLuna` | `swapPusdToV` |
| `swapLunaToUst` | `swapVToPusd` |
| `lunaRate` / `lunaRate_` | `vapurrRate` / `vapurrRate_` |
| `getLunaExchangeRate` | `getVapurrExchangeRate` |
| `offerLuna` / `askLuna` / `bool luna` | `offerV` / `askV` / `bool isV` |
| `terraPoolDelta` | `poolDelta` |
| `terraPool` (Snap) | `stablePool` |
| `lunaPool` (local) | `vapurrPool` |
| `contracts/terra-fork/` | `contracts/market-math-ref/` |
| `docs/econ/SWAP_UST_TO_LUNA.md` | `docs/econ/PUSD_V_REDEEM.md` |

Rust ABI strings in `crates/vapurr-econ` updated to match. Desk/ketbook `vapurrRate` wording updated. ROUTING "Olympus layer" -> Fed/Treasury.

Re-scan before ship: search `luna|olympus|\bust\b|terra-fork` excluding `market-math-ref` and RENAME_FLAGS — should be clean of brands (allow "trust"/"dust"/"must").
