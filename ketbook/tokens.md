# The two tokens

Both are 18 decimals. Only the market can mint or burn either. That minter is set in the constructor and cannot change.

## $VAPURR

On deploy the contract mints **1,000,000 $VAPURR** to the deployer. That is the only unearned mint. After that, $VAPURR appears only when someone redeems $PUSD, and disappears only when someone mints $PUSD.

## $PUSD

Purr USD. The dollar the window spends (KetPay, postage, vapurrbid). Balances are shares times an **index**. Lithe raises the index. Every holder is paid in place. No claim transaction.

`vapurr://pusd` · `vapurr://lithe`

## Price

The oracle is $PUSD per 1 $VAPURR, 18 decimals. The owner posts a pending rate with `feed`. The first swap of a block snapshots it. Between swaps, a virtual constant-product (`BASE_POOL` = 1,000,000, replenish over 14,400 blocks) sizes the stability spread. Floor is **2%**. No USDG sits in that pool.

## Vault

Isolated `$PUSD` credit. `$VAPURR` is the only extra collateral. Same oracle. Loop under 85% LTV. Not deployed until the desk does it. [Supply and borrow](euler.md).
