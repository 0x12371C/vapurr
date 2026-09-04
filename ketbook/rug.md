# Can't rug

No pause. No upgrade proxy. No `withdraw`. No `setFee`. No `mintTo` for friends. No `removeLiquidity` — there is no USDG book to pull.

$VAPURR’s minter is the market, immutable. $PUSD’s minter is the market, immutable. Nobody prints either token except through mint and redeem.

There **is** an owner. It is set in the constructor and cannot be transferred. That owner can `feed` the oracle: a pending rate, live on the first swap of the next block. That is the remaining admin surface. This page does not say “no admin.”

vapurrbid takes $PUSD into a second contract. Bids are never refunded. Nothing in that contract sends the pot out.

The vault (`PusdLoop`) is a third contract. Owner is the deployer and cannot be transferred. Owner receives reserve shares of borrow interest (10%). Owner cannot pull deposits, pause, or change LTV. Liquidators repay `$PUSD` and seize `$VAPURR` / supplied `$PUSD` at a 5% bonus. No USDG in that contract.

What this does **not** claim: smart-contract bugs, a bad `feed`, or a virtual pool that reverts `THIN`. Those are market and code risk.
