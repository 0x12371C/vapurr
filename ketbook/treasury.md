# Take

On chain, the take is the market and the board. No founder wallet.

**Mint spread (≥ 2%).** Minted as $PUSD into `yieldReserve`. Lithe (9%) pays it into every $PUSD balance through the index.

**Redeem spread.** The $VAPURR that would have been the fee is not minted.

**vapurrbid.** $PUSD paid for rank sits in the bid contract. Never refunded. Never swept to an admin.

**KetPay / x402.** On testnet 46630: merchant is paid in `$PUSD`. vapurr does not custody.

**zzzmail postage.** 0.25¢ voucher in $PUSD or $VAPURR. Settles later, under a cent.

**Swap / bridge.** The route is not cut. 25 bps buys `$VAPURR`; a small refund goes to you in `$VAPURR`; the rest burns to mint `$PUSD`. Quote + simulation today. Does not execute.

That is the whole take. The $PUSD desk does not run a second treasury.
