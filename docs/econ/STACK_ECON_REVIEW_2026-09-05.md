# Current stack and economic review - 2026-09-05

Reviewed local branch `fix/gv-spusd-guards`, starting at `a825573`.
Cargo version is 1.1.9. The newest local release zip is
`dist/vapurr-1.1.9-windows-x64.zip`, dated September 4 at 19:26 local time.
The September 5 source commits are newer than that zip. A version label alone
does not establish which financial contracts or security changes are packaged.

This is a source and integration review, not a comprehensive security audit or
fresh RPC attestation of deployed bytecode. Canonical addresses are configured
in vapurr-rhc; deployment claims in older docs were not treated as source parity.

## Stack map

| Layer | Current implementation | Economic implication |
|---|---|---|
| Browser | vapurr-shell, tao, four wry/WebView2 views, embedded frontend | Distribution/product layer; not a shipped Rust replacement for the rendering engine |
| Engine protocol | vapurr-engine FetcherEngine + vapurr-net; Servo slot unavailable | Native protocol groundwork; Servo is not on the browsing hot path |
| Profile/cache/shield | vapurr-core, vapurr-blob, vapurr-shield | Product utility; memory claims require actual browser measurements |
| Wallet | vapurr-wallet + shell security.rs, DPAPI vault, native confirmations, document-bound IPC | Device signing authority; not equivalent to hardware isolation |
| Chain/markets | vapurr-rhc RPC, Scan liquidity cache, route quote/simulation; vapurr-econ client | Mainnet 4663 data and testnet 46630 money must stay distinct |
| Identity/earn | vapurr-id VerifiedAccount gate, install_id, queued browsing receipts | Stream entitlement/distribution not wired by this change |
| Payments/mail | vapurr-pay protocol; shell KetPay testnet sends; vapurr-zmail/PNS/CID vouchers | Payment utility exists; postage vouchers are not settled fee revenue |
| Content/feed | frontend Ketflix/radio, vapurr-fomo; public content surfaces | Distribution/retention possibilities; no proven revenue budget in this review |
| Treasury optimizer | vapurr-econ treasury.rs + kelly.rs, recent Scan price windows | Suggested weights, not executed holdings, external reserves, or a solvency ledger |
| Lithe | PusdMarket/PusdToken, V↔PUSD mint/redeem rail, fee-funded PUSD rebase cap | Native PUSD fees; no contractual USDG redemption or 10M-market-cap peg switch |
| Oliver | PusdLoop supply, V collateral, borrowing/loop/unwind, 85% LTV / 90% liquidation threshold | Leverage belongs to user borrowing positions; protocol takes reserve fees |
| House | wgV pair configuration and fee adapters in source; older canonical testnet book | Wrapped equity book and outside-dollar peg book are distinct |
| Fed | GvFed V/gV/wgV, policy wrapper, BrowserStream | Separate Fed V and market V until migration; cannot treat them as interchangeable |
| Bonds | Gated BondMarket, discounted pre-funded inventory, per-asset capacity and valuation | Financing, not operating earnings; no bond-yield collateral connection |
| Savings | SPUSD, SpusdCd, new SavingsRouter, shared sink/floor | Tested cash allocation, not a deployed banking balance sheet |

## What funds the system

The source supports a clear separation between financing, issuance, and earnings:

- BrowserStream transfers at most 50,000 already-minted V globally over three years.
  Full funding implies about 45.66 V/day shared across eligible recipients.
  This is an inventory-funded acquisition budget, not per-user issuance.
- gV accrue mints equity backing into gV. It does not fund the savings router.
- Bonds exchange external assets for discounted vested equity inventory.
  Their inflow is financing; it is not recurring income available to promise forever.
- House's protocol carve, Lithe's remaining mint fee inventory, and Oliver's
  remittable reserve are the native PUSD sources for savings.
- Sink PUSD and recursively supplied PUSD are not exogenous USDG backing.
  Accounting in PUSD units must remain distinct from dollar exit capacity.

A fixed token budget scales down per-person rewards as usage grows. Whether
browser rewards acquire valuable users still depends on measured retention,
attributable revenue, operating costs, and fraud losses. The stack does not
yet establish those unit economics.

## Where leverage actually sits

Oliver allows PUSD supply, V collateral, and PUSD borrowing. Its loop adds
supply shares and debt shares without transferring additional PUSD into the vault.
The maximum repeated-loan leverage tends toward 1 / (1 - 0.85) = 6.67,
subject to steps, cash limits, interest, and collateral constraints.

This is gross user position leverage. It is not a 6x multiplier on the vault's
actual cash, exogenous reserves, or the PUSD earning Lithe rebases.

For an isolated self-looped book with starting equity E, gross supplied claims
L*E, debt (L-1)*E, reserve fraction r=10%, and opening borrow rate b:

```text
borrow interest paid       = b * (L-1) * E
supplier interest received = (1-r) * b * (L-1) * E
net loan carry              = -r * b * (L-1) * E
```

Only actual cash in the vault earns the underlying PUSD rebase. Recursive claims
do not each hold another copy of that cash.

Illustration using the current steady-state curve: cash >= 100k PUSD,
L=6, utilization=5/6. The opening borrow rate is about 5.56%; the isolated
self-loop's loan carry is approximately -2.78% of starting equity annually,
before compounding, changes in utilization, rebase income, and costs.
With only 10k PUSD cash, the boot slope makes the analogous opening borrow rate
about 125.56%, and loan carry approximately -62.78%. These are static formula
illustrations, not return forecasts or measured deployed results.

The banking thesis therefore needs borrowers willing to pay for useful liquidity.
Self-loop interest is partly a transfer back to the same supplier, with a reserve
fee cost. It cannot by itself create system-wide external income.

## What this change builds

`SavingsRouter.sol` divides one post-floor payment between liquid sPUSD and
CD surplus. It deploys disabled, accepts only its configured sink, matches
underlying assets, and rolls back both legs if either receiver fails.
It blocks liquid allocations when there are no redeemable saver shares.

`SpusdCd.sol` now fixes coupon targets, break fees, and maturity at deposit.
It tracks aggregate principal and open coupon targets. Underfunded maturities
receive proportional coupon cash, including a reserve share for unmatured
positions. Closing extinguishes unpaid targets. Principal stays in cash.

The selected behavior is a contingent savings product. It introduces no
guaranteed APY, new mint authority, loan expansion, or CD collateral valuation.
See `SPUSD.md` for exact settlement rules and the wiring recipe.

The integration test uses real local market and lending contracts at a flat V
oracle price. It proves that collected branch fees can reach savers while the
single sink floor and depositor claims are preserved. It does not prove long-run
profitability, automated House fee generation, or USDG solvency.

## Material remaining gaps

1. **Economic demand and external exit:** no measured recurring revenue, borrower
   demand, realized USDG depth, or loss-adjusted treasury balance sheet was established.
   A 10M market cap alone cannot make PUSD redeemable for a dollar.
2. **Collateral and losses:** Oliver values V through an owner-fed oracle with
   freshness/jump checks; it has an optional backstop and socializes uncovered bad debt
   into supplier asset values. The stack does not yet implement the proposed CD/bond
   collateral product or a funded Fed first-loss policy.
3. **Coupon admission:** new positions can dilute coverage of open contingent targets.
   Fixed guaranteed coupons would require funded reservations and issuance capacity.
   CD principal is not currently productive lending capital; a lockup alone does not
   create extra yield.
4. **Yield provenance:** FeeAttribution.credit currently permits any funded caller
   to choose a named source. Even registered House fee credits may include voluntary
   inventory transfers. Lifetime source totals do not prove organic earnings or a
   particular saver's yield history. Tighten provenance before publishing those claims.
5. **PUSD holder drip:** the current underlying rebases, including balances at the
   sink and liquid vault. Those gains are separate from router remittance totals.
   CD passive gains/donations remain unallocated inventory in this slice.
6. **Rebase wording:** gV accrue is permissionless and applies elapsed-time interest
   to the current supply/index. Repeated settlement compounds; a single annual
   settlement produces 3.5%, while frequent settlements approach about 3.562%.
   Existing prose claiming an absolutely flat, policy-only clock is too strong.
7. **Deployment/identity:** current Rust commands/address book do not deploy or invoke
   the savings router or CDs. `PusdMarketFed` and `LegacyVConverter` now provide the
   one-token source path, but the live gen-4 address book still uses the embedded-V
   market. New source behavior must not be attributed to older deployed contracts or
   the September 4 zip.
8. **Release:** signing and hostile-page retest remain open on the existing release
   board. This work changes source and tests; it does not pack, install, publish, or
   authorize transactions.

## Validation

- Baseline: 85 Foundry tests passed.
- Savings change: 102 Foundry tests passed, including 17 new tests and two
  256-case fuzz tests for allocation/payout conservation.
- Real PUSD rebase rounding is covered for CD incoming principal and surplus.
- Rust workspace tests passed with offline dependencies and an isolated temporary app profile; four live tests ignored. Existing unused-code/import warnings remain.

Next banking work should start with a measured reserve/loan book and a priced
borrower use case. Source-tested savings allocation is now available as the
earnings distribution layer for that work.
