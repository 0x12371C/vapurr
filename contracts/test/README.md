# GvFed boundary tests

From `contracts/`:

```
git clone --depth 1 https://github.com/foundry-rs/forge-std lib/forge-std
forge test -vv
```

Proves: 3.5%/yr rebase, BrowserStream no mint, browse cannot rebase, wgV tracks gV.

## Canonical V cutover (gen-5 source)

orge test --offline --match-path test/Canonical*.t.sol -vv proves that
canonical Lithe uses the Fed V address as inventory without minting it, Oliver
uses that same V as collateral, the desk's 12-word snapshot ABI remains intact,
and both direct V conversion and legacy-Lithe redeem -> V conversion -> inventory-V
swap (PUSD mint) pay only pre-funded canonical inventory. CanonicalLitheFactory.t.sol
covers one-tx successor wiring with no leftover factory mint role. Live 46630 remains gen-4.

## Earnings and savings integration

`forge test --offline --summary` runs the existing suite plus SavingsRouter.t.sol, SpusdCd.t.sol, and EarningsEngine.t.sol. The latter uses the real local market, lending, fee-attribution, sink, and savings contracts. No broadcast or deployed-address mutation. The suite includes Canonical cutover proofs after the 2026-09-05 gen-5 source land.
