# The loop

Two tokens. One dollar the browser actually spends.

**$VAPURR** is equity. Burn it to mint **$PUSD**. Redeem $PUSD and $VAPURR is minted back. Oracle is first spot of the block. Stability spread ≥ 2%.

**$PUSD** is the vapurr dollar. Unit of account in the window. **Lithe** is 9% on every $PUSD balance (index drip, no extra token, no claim). Mint spread funds Lithe. On redeem, the $VAPURR fee is never minted.

**ETH** is gas. **USDG** is Robinhood Chain’s dollar — LI.FI quotes, merchants who only list USDG, the card world. It is not in the mint. It is not in the house AMM.

House liquidity is **$VAPURR / $PUSD** only. No house pool vs USDG or ETH. You cannot dump V or P into Robinhood’s dollar from this desk. $PUSD has to be the dollar.

**Supply and borrow** (Euler-shaped, not a Euler fork) is how that dollar gets its own depth. Isolated `$PUSD` credit. `$VAPURR` is collateral. Supply P, borrow P, loop under 85% LTV. Utilization sets the rate. Liquidations at 90%. Lithe still drips on vault-held `$PUSD`. Vault `0xC4d4BC75…39Bb` is live on 46630. See [Supply and borrow](euler.md).

The **house book** is a Uniswap v4 concentrated `$VAPURR` / `$PUSD` position (0.30%, ±20% around the oracle). 50% of genesis `$VAPURR` stays treasury. Of the LP half, burn half to mint `$PUSD` and keep half as `$VAPURR`. No house `$PUSD/USDG`. House `0x667bFcAF…1bf7` · swapper `0xb699c0CD…4FE2`. `vapurr://house`

## Spend (the browser)

| Surface | What $PUSD does | Live? |
|---|---|---|
| **KetPay** | HTTP 402 / x402. Pay the site to continue. Prefer $PUSD on testnet `eip155:46630`. USDG if that is all they accept on that net. Card passthrough for Visa-shaped merchants. | Settles `$PUSD` on **46630**. Refuses mainnet. |
| **zzzmail** | 0.25¢ postage voucher in $PUSD (or $VAPURR). 0 ETH. Body is a CID. Cap 1¢ if a relayer ever posts a pointer. | Seal + pin + voucher. Does **not** settle on chain yet. |
| **vapurrbid** | Rank on the home floor. First listing 10 $PUSD. Take #1 at top+5. Pot stays in the contract. | Live. |
| **Swap / bridge** | Full route. Small `$VAPURR` refund (5 bps). Protocol 25 bps buys `$VAPURR`; the rest burns to mint `$PUSD`. Score is full out minus gas. Simulation before payable. | Quote + sim live. Does **not** execute. |

`vapurr://earn` is browse-to-earn. Different product. Not Lithe.

## Why mint

KetPay, postage, and bid are demand for $PUSD. Mint burns $VAPURR. Hold $PUSD and Lithe pays 9% from that mint spread. Hold $VAPURR if mint burns V faster than redeem mints it.

No other on-chain take. No founder wallet in the market.
