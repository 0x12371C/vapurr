# Supply and borrow

Euler-shaped. Not a Euler fork. Not Euler Finance. Isolated `$PUSD` credit on the vapurr book.

The mint still burns `$VAPURR` to create `$PUSD`. The vault does not print either token. It lets `$PUSD` that already exists sit as lendable cash, and lets `$VAPURR` (plus supplied `$PUSD`) borrow that cash. Looping is recursive supply/borrow in one transaction. That is **new `$PUSD` depth** — utilization of P, not a pool vs USDG or ETH.

`vapurr://euler` · `vapurr://loop` · same desk as `vapurr://pusd`

Live on testnet 46630 at `0x89E17eefa58B99d025145970c0FBAe7768a14521`. Mainnet is empty. The desk shows the vault.

## Two silos

| Silo | Role |
|---|---|
| `$PUSD` | Only credit asset. Supply shares. Debt shares. Cash in the vault earns **Lithe** because `$PUSD` is shares × index. |
| `$VAPURR` | Collateral only. Not borrowable. Priced at the market oracle (`lunaRate`, `$PUSD` per 1 `$VAPURR`). |

No USDG vault. No ETH vault. No WETH vault. You cannot dump V or P into Robinhood’s dollar from this desk.

## Parameters (`PusdLoop.sol`)

| Name | Value | Meaning |
|---|---|---|
| `LTV_BPS` | 85% | max borrow / collateral |
| `LLTV_BPS` | 90% | liquidation threshold |
| `LIQ_BONUS_BPS` | 5% | liquidator discount |
| `RESERVE_BPS` | 10% | cut of borrow interest, as supply shares to the deployer |
| `KINK` | 90% util | IRM kink |
| `BOOT_SLOPE1` | 150% | borrow APY at kink while cash is 0 |
| `BASE_SLOPE1` | 6% | borrow APY at kink once 100k $PUSD cash is in |
| `BOOT_CASH` | 100,000 P | exogenous cash that fades the boot. Looping does not count. |
| `SLOPE2` | 100% | extra borrow APY from kink to 100% util |
| `MAX_STEPS` | 16 | loop / unwind cap |
| `VIRTUAL` | 1e6 wei | share-price offset (inflation guard) |

Collateral value = supplied `$PUSD` + `$VAPURR` × oracle. Self-collateral: supplied P backs more P. A pure P loop cannot be liquidated (P/P = 1). V is the risk.

Max leverage on a P loop is `1 / (1 − 0.85) ≈ 6.67×` after enough steps. Eight steps is the desk default.

## Utilization IRM

`util = borrows / (cash + borrows)`

Below the kink, borrow APY rises linearly to the live slope. That slope starts at **150%** when vault cash is 0 and fades to **6%** as real `$PUSD` cash hits 100k. Recursive loop does not raise cash, so it cannot farm the boot forever. Real suppliers and Lithe drip can. Above the kink, SLOPE2 still jumps. Supply APY is borrow APY × util × (1 − reserve). Lithe is separate: 9% on vault-held `$PUSD` cash.

Looping does not send tokens out. Cash stays. Borrows and supply shares rise together. That is the depth.

## Liquidations

If debt > collateral × 90%, anyone can repay `$PUSD` and seize V first, then supplied P, at a 5% bonus. They cannot seize more value than the bonus math allows. Leftover bad debt can remain. That is market risk, same family as a bad `feed`.

## What this is not

Not Euler V2. Not EVK. Not a house `$PUSD/USDG` pool. The desk will not fake a CA.
