# sPUSD and term savings

Liquid sPUSD is a share claim on PUSD held in SPUSD. Its NAV rises when it
receives remittance or when its underlying PUSD rebases. It does not receive
the gV equity rebase. A nominal PUSD claim is not a guaranteed USDG redemption.

## Implemented savings path (2026-09-05)

```text
House / Lithe / Oliver
    -> FeeAttribution
    -> RemittanceSink [one RunwayFloor]
    -> SavingsRouter [disabled by default]
         -> SPUSD.receiveRemittance       [liquid share NAV]
         -> SpusdCd.receiveRemittance     [CD coupon budget]
```

The sink still supports forwarding directly to one receiver. SavingsRouter
adds the missing allocation between the two products. It only accepts calls
from its immutable sink and verifies that both receivers use the same asset.

- `setAllocation(enabled, cdBps)` sets the CD share of FUTURE surplus receipts.
  `cdBps` is not an interest rate. Deployment starts disabled with a zero CD share.
- `sink.forwardSurplus(0)` forwards only cash above the shared floor.
  No new floor, minting, borrowing, or depositor-principal transfer is introduced.
- A nonzero liquid allocation requires live liquid shares beyond the dead shares.
  An empty liquid vault cannot receive a first-depositor windfall through this router.
- If either receiver fails, the entire transaction rolls back, including the first
  allocation. The cash stays at the sink. Receiver allowances are cleared after use.
- `totalReceived`, `totalLiquid`, and `totalCd` record received token balances.
  Rebasing-token transfer rounding can leave small residual inventory in the router.
- The split is an operator policy, not a demonstrated sustainable rate. No particular
  split is enabled by this change.

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
7. Select a split and enable the router. Call forwardSurplus from the sink owner.
8. Read position terms and previewClose before requesting any CD close transaction.

SavingsRouter is not in the Rust address book or wallet IPC yet. The Bonds/CD
surface stays disabled. Existing deployments cannot acquire these source changes;
new contracts and reviewed wiring are required.

## Validation

`SavingsRouter.t.sol`: default gate, source authorization, asset matching, empty-vault
protection, atomic failure, floor preservation, and fuzzed conservation.

`SpusdCd.t.sol`: frozen terms, proportional underfunding across mature/unmatured
positions, early-exit accounting, principal isolation, ownership, and fuzzed payouts.

`EarningsEngine.t.sol`: real Lithe mint fees, House funded carve, Oliver accrued then
repaid interest -> attribution -> sink -> both savings legs at a flat V oracle price.
Also tests actual rebasing PUSD deposits and credits. The House fee is funded inventory
in this test; automated Uni swap fee collection remains a separate integration.
