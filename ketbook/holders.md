# Holders

## $PUSD

You hold shares. The index goes up when Lithe drips.

Minting $PUSD pays the spread (≥ 2%) into `yieldReserve` as $PUSD. `accrue` pays that reserve by raising the index, so every $PUSD balance grows without a claim. Cap is **9%** (`MAX_APY_BPS = 900`). If the reserve is fat, it pays slowly. If it is thin, it pays what it has.

That is how mint volume comes back to $PUSD holders. Hold `$PUSD` because 404, postage, and vapurrbid spend it.

## $VAPURR

When people mint $PUSD, $VAPURR is burned. Supply falls. When they redeem, $VAPURR is minted. On redeem the $VAPURR spread is not minted — that supply never appears.

No staking contract. No emission after genesis. $VAPURR holders get paid if mint demand burns V faster than redemptions mint it.
