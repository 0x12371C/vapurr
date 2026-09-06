# sPUSD and term savings

Liquid sPUSD is a share claim on PUSD held in SPUSD. Its NAV rises when it
receives remittance or when its underlying PUSD rebases. It does not receive
the gV equity rebase. A nominal PUSD claim is not a guaranteed USDG redemption.

## Implemented savings path (2026-09-05)

```text
House / Lithe / Oliver
    -> FeeAttribution
    -> RemittanceSink [one RunwayFloor]
    -> SavingsRouter [enabled by default; owner killswitch]
         -> SPUSD.receiveRemittance       [liquid share NAV]
         -> SpusdCd.receiveRemittance     [CD coupon budget]
```

The sink still supports forwarding directly to one receiver. SavingsRouter
adds the missing allocation between the two products. It only accepts calls
from its immutable sink and verifies that both receivers use the same asset.

- `setAllocation(enabled, cdBps)` sets the CD share of FUTURE surplus receipts.
  `cdBps` is not an interest rate. Constructor starts **enabled** with `cdBps = 2500` (25% of post-floor surplus to CD); owner `setAllocation(false, ...)` is the killswitch.
- `sink.forwardSurplus(0)` forwards only cash above the shared floor.
  No new floor, minting, borrowing, or depositor-principal transfer is introduced.
- A nonzero liquid allocation requires live liquid shares beyond the dead shares.
  An empty liquid vault cannot receive a first-depositor windfall through this router.
- If either receiver fails, the entire transaction rolls back, including the first
  allocation. The cash stays at the sink. Receiver allowances are cleared after use.
- `totalReceived`, `totalLiquid`, and `totalCd` record received token balances.
  Rebasing-token transfer rounding can leave small residual inventory in the router.
- The split is an operator policy, not a demonstrated sustainable rate. Defaults are live-by-default for ship posture; change them with `setAllocation`.

## CD terms and settlement

SpusdCd holds nominal PUSD principal in cash. It does not lend that principal
to Oliver or accept a CD/bond claim as collateral.

| Field | Behavior |
|---|---|
| Principal | Actual received PUSD, measured from the balance change at deposit |
| Term | Unlock timestamp fixed at entry; timestamp overflow rejected |
| Coupon | Fixed target per term, recorded in `couponDue(id)`; funding contingent |
| Break fee | Entry-time rate in `positionBreakFeeBps(id)` |
| Policy changes | Affect only new positions; existing targets, fees, and unlocks remain fixed |
| Early close | Principal minus entry-time break fee; no coupon; fee remains surplus |
| Mature close | Principal plus the funded portion of the coupon target |
| Later entitlement | Close is final; unpaid coupon targets do not survive as arrears |

`totalPrincipal` tracks all open principal claims. `totalCouponDue` tracks all
open coupon targets, including positions that have not matured.

```text
availableSurplus = min(credited surplus, max(actual PUSD balance - totalPrincipal, 0))
couponOut       = due                             if availableSurplus >= totalCouponDue
                  floor(availableSurplus * due / totalCouponDue) otherwise
```

Example: two open positions each target 500 PUSD, but only 100 PUSD is funded.
Each receives 50 PUSD if both mature and close with no intervening changes.
The first closer cannot sweep the whole 100 PUSD. Integer rounding favors
remaining positions by at most payout-rounding dust for a fixed book.

New deposits, early exits, and later surplus credits change the coverage ratio.
This is a pooled, contingent coupon design, not a guaranteed fixed-rate deposit.
A future guaranteed-coupon product would need admission capacity and reserved funding.
The UI must show the target per term and current funded preview separately.

Direct donations and passive PUSD rebases are not automatically added to the
CD coupon ledger. They remain unallocated inventory. No sweep path is introduced.
Coupons never consume another depositor's recorded principal or mint PUSD/V.

## Local wiring recipe

The sequence below describes contract calls for a reviewed future deployment;
it is not a deployment performed by this change.

1. Use one PUSD asset, RunwayFloor, and RemittanceSink.
2. Deploy SPUSD and SpusdCd using that same PUSD address.
3. Deploy SavingsRouter(sink, liquid, cd); check all immutable addresses.
4. Register branch addresses in FeeAttribution and wire branch remittance to it.
5. Set the sink's forward receiver to the router.
6. Seed genuine liquid savings before any allocation with a nonzero liquid leg,
   or use an explicitly selected all-CD allocation to pre-fund coupons.
7. Confirm or tune `setAllocation` (default enabled @ 25% CD). Call forwardSurplus from the sink owner.
8. Read position terms and previewClose before requesting any CD close transaction.

## UI sketch (2026-09-05)

`frontend/bonds.html` `#spusd-cd` is a **live CTA stub** (Open CD + amount), same posture as Open Bond:
actionable controls, clear on-tx / not-configured error when address-book / IPC is missing.
Do not gray-gate or hide the CD surface. Term / coupon / break-fee rows stay placeholder until a savings market is connected (`econ-cd-open`).

**Address book / IPC (2026-09-06):** empty slots landed in apurr-rhc (TESTNET_SPUSD, TESTNET_SPUSD_CD, TESTNET_SAVINGS_ROUTER) + MarketCfg (spusd / spusd_cd / savings_router) and snap under savings. econ-cd-open parses and returns **NeedSavings** until Relic fills CAs after a reviewed deploy. Live open/ABI still open; existing deployments do not pick up source-only changes automatically.

## Validation

`SavingsRouter.t.sol`: default gate, source authorization, asset matching, empty-vault
protection, atomic failure, floor preservation, and fuzzed conservation.

`SpusdCd.t.sol`: frozen terms, proportional underfunding across mature/unmatured
positions, early-exit accounting, principal isolation, ownership, and fuzzed payouts.

`EarningsEngine.t.sol`: real Lithe mint fees, House funded carve, Oliver accrued then
repaid interest -> attribution -> sink -> both savings legs at a flat V oracle price.
Also tests actual rebasing PUSD deposits and credits. The House fee is funded inventory
in this test; automated Uni swap fee collection remains a separate integration.

### NeedSavings honesty UI (2026-09-06)

frontend/bonds.html #spusd-cd paints #cd-book-note from snap.savings.configured (empty book -> NeedSavings). Open CD CTA stays live; on-tx miss surfaces clearly. Live open ABI still Relic-gated after deploy.

