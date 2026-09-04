# Mint and redeem

Constants in `PusdMarket`:

| Name | Value | Meaning |
|---|---|---|
| `BASE_POOL` | 1,000,000 × 1e18 | virtual pool |
| `POOL_RECOVERY_PERIOD` | 14,400 | blocks to replenish the virtual delta |
| `MIN_STABILITY_SPREAD` | 2e16 | 2% floor |
| `MAX_APY_BPS` | 900 | Lithe cap, 9% |
| `YEAR` | 365 days | drip clock |
| `GENESIS` | 1,000,000 $VAPURR | once, to the deployer |

A trade that would empty the virtual pool reverts `THIN`.

## Mint $PUSD — burn $VAPURR

1. First-spot the oracle for this block.
2. Accrue Lithe.
3. Size the ask at oracle minus spread (≥ 2%).
4. Take and burn your $VAPURR.
5. Mint $PUSD to you.
6. Mint the spread as $PUSD into `yieldReserve`.

## Redeem $PUSD — mint $VAPURR

1. First-spot. Accrue.
2. Size the ask at oracle minus spread.
3. Burn your $PUSD.
4. Mint $VAPURR to you. The $VAPURR spread is **not minted**.

No USDG moves. There is no `addLiquidity`. Deploy is what puts 1,000,000 $VAPURR on the deploying device.

Supply, borrow, and loop are a second contract (`PusdLoop`). They do not mint. [Supply and borrow](euler.md).
