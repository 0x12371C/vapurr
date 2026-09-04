# Testnet token economy shape (Relic · 2026-09-04)

**Home:** Robinhood Chain testnet / econ bootstrap **46630**.  
**Assets (both ours):** `$VAPURR` (equity) + `$PUSD` (product dollar). Chain USDG is not our product dollar.

## Genesis split

On initial token creation / bootstrap mint of `$VAPURR`:

| Slice | Share | Destination |
|-------|-------|-------------|
| **LP** | **50%** | Burn `$VAPURR` → mint `$PUSD`, seed **Uniswap v4 concentrated liquidity** pool `$VAPURR` / `$PUSD` |
| **Treasury** | **50%** | Protocol treasury (held; not LP) |

Relic: *"initial 50% of token creation goes to LP creation, so half of the $VAPURR gets burned to create $PUSD… this becomes our uniswap v4 concentrated liquidity pool… since we have a stable coin we can create a good stable market here. The other 50% is treasury."*

## Why this shape

- We control both legs → can bootstrap a real `$VAPURR`/`$PUSD` market without depending on external USDG depth.
- `$PUSD` as the stable leg → concentrated liquidity around a stable price band (Uni v4 hooks/CL) instead of a wide chaotic AMM.
- Treasury 50% stays dry powder (ops, BrowsePool seeding, later Lithe/earn — separate from this bootstrap doc).

## LP bootstrap (intended flow)

1. Mint genesis `$VAPURR` supply `S`.
2. Send `0.5·S` to **treasury**.
3. Take `0.5·S` as **LP slice**:
   - Burn LP-slice `$VAPURR` (per Relic: this half creates `$PUSD`).
   - Mint `$PUSD` against that burn (mint policy: document rate in STATUS when coded — start 1:1 testnet unless Lithe/spread says otherwise).
   - Initialize Uni v4 **CL** pool `$VAPURR`/`$PUSD` with the resulting `$PUSD` and the paired `$VAPURR` side required by the pool (see Open).
4. Record pool id / hooks / tick range in `docs/STATUS.md` when live on 46630.

## Locked (Relic 2026-09-04 internal deploy)

1. **Paired VAPURR:** **A.** Of the LP 50%, burn **half** → `$PUSD` (market mint, ≥2% spread) and keep **half** as `$VAPURR`. Both legs seed the Uni v4 CL. Existing `$PUSD` on the signing device also goes in. Treasury 50% never enters the pool.
2. **Tick range:** ±20% around the oracle (`tick ±1860`, spacing 60). Fee **0.30%** (`UNI_V4_FEE_VOL`). No hook.
3. **Lithe:** stays on mint-spread / `$PUSD` index. The pool does not double-charge.
4. **46630 only.** `HouseLp.sol` + `TESTNET_HOUSE` (empty until this device deploys). NFT to the signing device.
5. **Stocks** (AMZN/TSLA/AMD/NFLX/PLTR on testnet) are **ops**. Sell them for ETH/USDG gas. They are not house LP. No `$PUSD/USDG` pool. Testnet has Uni v4 PM/POSM/Permit2; no Universal Router and no WETH on 46630 — stock dumps are not wired until a v4 swap helper exists.

## Open (still)

- **Fresh gen-4 is live on 46630.** Market `0x47Aca529…3617`, house NFT #2273 at `0x667bFcAF…1bf7`, vault `0xC4d4BC75…39Bb`, swapper `0xb699c0CD…4FE2`. Retired CAs do not count.

## Non-goals (for this shape)

- Not branding pay as 404.
- Not unbounded `$VAPURR` print for earn (earn pays `$PUSD` from Lithe/mint-spread BrowsePool).
- Not mainnet `4663` settlement until Relic opens it.

## Owners

- **Pilot / Econ:** encode mint → burn → `$PUSD` → Uni v4 CL bootstrap; desk truth.
- **House:** pack/docs when contracts land.
- **vapurrbot:** keep this file + TRACKS honest.

## Related

- `docs/ketpay/DESIGN.md` — KetPay / `$PUSD` / Lithe / BrowsePool
- `docs/ketpay/SYBIL.md` — install_id
- Chain: `eip155:4663` home; econ bootstrap `46630`