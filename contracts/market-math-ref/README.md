# Market stability-pool math (reference)

Internal reference for `$VAPURR` / `$PUSD` mint-redeem curve + pool recovery.
Ship surfaces and symbols use vapurr-native names only.

## Symbol map (code)

| Concept | Symbol |
|---------|--------|
| Equity | `$VAPURR` |
| Product dollar | `$PUSD` |
| Spot oracle | `vapurrRate` |
| V sold for PUSD | `swapVToPusd` |
| PUSD redeemed for V | `swapPusdToV` (inventory unwrap; no V mint) |
| Stability pool delta | `poolDelta` |

Do not reintroduce external chain brand names in ship code, docs, or UI.