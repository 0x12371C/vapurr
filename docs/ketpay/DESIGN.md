# KetPay

Product name: **KetPay**. Wire: HTTP **402** / **x402 v2**. Unit: **$PUSD** on Robinhood testnet `46630` (USDG only if accept-list has no PUSD on that net). v1.2 does not settle on mainnet `4663`. Not called 404 â€” 404 is the load-fail page.

## What it is
Class/module in `vapurr-pay` (+ thin chrome) so the shell can pay *anywhere* a resource speaks x402: browse paywalls, APIs, agents. Prefer $PUSD. Card passthrough stays separate (after live Rain).

## Exists today
- `vapurr-pay`: PaymentRequired, PayRouter, already prefers PUSD via `pick_rhc_dollar` / `is_pusd`
- `frontend/pay.html` KetPay sheet â€” `wallet-send` `$PUSD` on 46630; refuses mainnet
- Earn chrome `frontend/earn.html`: opt-in host sharing, pending in **USDG** Ã¢â‚¬â€ needs retarget to $PUSD + real faucet from supply

## Build (ordered)
1. Rename product surface to **KetPay** (`vapurr://ketpay` alias `pay`; drop alias `404`)
2. Settlement path for X402+$PUSD (device key sign Ã¢â€ â€™ pay_to) Ã¢â‚¬â€ start testnet 46630; keep honest if mainnet gas missing
3. Chrome: KetPay sheet shows amount in $PUSD, not USDG theatre
4. Tests around Accept picking PUSD over USDG

## Browse-earn Ã¢â‚¬â€ where the money comes from
Candidates (pick one primary, document in STATUS):
- **Lithe slice**: Lithe is 9% on $PUSD (index drip). Carve a fixed bps of Lithe (or of mint-spread that funds Lithe) into a **BrowsePool** paid in $PUSD
- **Seigniorage bps**: on each VÃ¢â€ â€™P mint, skim N bps to BrowsePool before Lithe
- **Not** unbounded $VAPURR print Ã¢â‚¬â€ dilutes equity; prefer $PUSD from real spread

Earn UX already: opt-in, host+HTTPS+time only. Change payout asset Ã¢â€ â€™ $PUSD. Rate stays tiny; claim via Submit window Ã¢â€ â€™ BrowsePool.

## Sybil (stronger than IP)
IP lock = weak / VPN theatre. Because we ship a Windows install:
1. **Device key is the identity** for earn (already local). One pending balance per device key.
2. **Install attestation**: channel stamp / `vapurr.next` patcher identity + install path under `%LOCALAPPDATA%\Programs\vapurr` Ã¢â‚¬â€ bind earn enrollment to first-run machine id (hash of machineguid + install id, not raw HWID in logs)
3. **Rate limits**: per-device visit caps, diminishing returns, cooldown on Submit
4. **Stake-to-earn (optional later)**: small $VAPURR or $PUSD lock to raise caps Ã¢â‚¬â€ farmers pay
5. **zer0ID unique-human** after v1 Ã¢â‚¬â€ don't block KetPay design on it
6. Reject headless / automation WV flags (already partly in Shield/Boost work)

Never store raw hardware serials in plaintext receipts.

## Owners
- Pilot: KetPay product + earn/$PUSD + sybil design in code/docs
- House: SURFACES/AGENTS rename payÃ¢â€°Â 404 when touching docs
- vapurrbot: track board

## Out of scope tonight
Live Rain. Fake settled payments. Servo.

## Decision (Relic)
- Sybil root: **per machine install bound** (install_id + device key). See SYBIL.md.

## Decision (Relic) â€” earn payout
- Browse-earn **payment** requires zer0ID KYC at https://www.thesecretlab.app/kyc.
- Enrollment stays per machine install; KYC gates **claim/payout** only.

## Genesis economy (Relic 2026-09-04)
See `docs/econ/TESTNET_SHAPE.md`: genesis `` split **50% LP** (burn → `` → Uni v4 CL ``/``) / **50% treasury**. Both assets ours on testnet.
