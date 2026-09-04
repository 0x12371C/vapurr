# Can't rug

No pause. No upgrade proxy. No `withdraw`. No `setFee`. No `mintTo` for friends. No `removeLiquidity` — there is no USDG book to pull.

$VAPURR’s minter is the market, immutable. $PUSD’s minter is the market, immutable. Nobody prints either token except through mint and redeem.

There **is** an owner. It is set in the constructor and cannot be transferred. That owner can `feed` the oracle: a pending rate, live on the first swap of the next block. That is the remaining admin surface. This page does not say “no admin.”

vapurrbid takes $PUSD into a second contract. Bids are never refunded. Nothing in that contract sends the pot out.

What this does **not** claim: smart-contract bugs, a bad `feed`, or a virtual pool that reverts `THIN`. Those are market and code risk.
