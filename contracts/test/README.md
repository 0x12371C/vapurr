# GvFed boundary tests

From `contracts/`:

```
git clone --depth 1 https://github.com/foundry-rs/forge-std lib/forge-std
forge test -vv
```

Proves: 3.5%/yr rebase, BrowserStream no mint, browse cannot rebase, wgV tracks gV.

## Earnings and savings integration

`forge test --offline --summary` runs the existing suite plus SavingsRouter.t.sol, SpusdCd.t.sol, and EarningsEngine.t.sol. The latter uses the real local market, lending, fee-attribution, sink, and savings contracts. No broadcast or deployed-address mutation. The suite has 102 passing tests after the 2026-09-05 savings slice.
