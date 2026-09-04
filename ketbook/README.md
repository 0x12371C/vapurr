# What we're building

vapurr is a browser with an on-chain dollar in it.

You browse on this PC. Robinhood Chain is home. **$PUSD** is what the window spends (404, postage, vapurrbid). **$VAPURR** is burned to mint that dollar. **Lithe** is 9% on $PUSD.

There is no USDG in the mint. Burn V, mint P, at the first-spot oracle, minus a stability spread of at least 2%. Mint spread funds Lithe.

The contract is a Terra Classic `x/market` fork. Source lives in `contracts/terra-fork/` from [classic-core x/market](https://github.com/terra-money/classic-core/blob/main/x/market/keeper/swap.go).

The loop: [The loop](loop.md).
