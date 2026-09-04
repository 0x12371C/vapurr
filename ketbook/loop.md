# The loop

Two tokens. One dollar the browser actually spends.

**$VAPURR** is equity. Burn it to mint **$PUSD**. Redeem $PUSD and $VAPURR is minted back. Oracle is first spot of the block. Stability spread ≥ 2%.

**$PUSD** is the vapurr dollar. Unit of account in the window. **Lithe** is 9% on every $PUSD balance (index drip, no extra token, no claim). Mint spread funds Lithe. On redeem, the $VAPURR fee is never minted.

**ETH** is gas. **USDG** is Robinhood Chain’s dollar — LI.FI quotes, merchants who only list USDG, the card world. It is not in the mint.

## Spend (the browser)

| Surface | What $PUSD does | Live? |
|---|---|---|
| **404** | HTTP 402 / x402. Pay the site to continue. Prefer $PUSD on `eip155:4663`. USDG if that is all they accept. Card passthrough for Visa-shaped merchants. | Sheet is live. Does **not** settle. |
| **zzzmail** | 0.25¢ postage voucher in $PUSD (or $VAPURR). 0 ETH. Body is a CID. Cap 1¢ if a relayer ever posts a pointer. | Seal + pin + voucher. Does **not** settle on chain yet. |
| **vapurrbid** | Rank on the home floor. First listing 10 $PUSD. Take #1 at top+5. Pot stays in the contract. | Live. |
| **Swap / bridge** | 25 bps vapurr scoop on the quote (`ROUTE_FEE_BPS`). | Quote live. Does **not** execute. |

`vapurr://earn` is browse-to-earn. Different product. Not Lithe.

## Why mint

404, postage, and bid are demand for $PUSD. Mint burns $VAPURR. Hold $PUSD and Lithe pays 9% from that mint spread. Hold $VAPURR if mint burns V faster than redeem mints it.

No other on-chain take. No founder wallet in the market.
