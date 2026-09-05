# Codex brain pass (econ hard knots)

Date: 2026-09-05 (America/New_York). Model: gpt-6-astra, high effort. Mode: brain-only, no patches.
Branches in scope: `feat/bonds-ux-map`, `fix/pusdloop-routing-gaps`.


## Progress — Oliver LTV (P0 fix #1)

Date: 2026-09-05 (America/New_York). Branch: `fix/oliver-ltv`.

- Post-action health: `borrow` / `withdraw` transfer cash first, then `_requireLtv` (reverts unwind the transfer).
- Cash/util policy: sole-supply book cannot borrow past LTV once cash has left; util stays within LTV_BPS in proofs.
- `remitReserve`: remaining-LTV on owner after share burn + sink pull.
- Auto-remit: best-effort via `try this.remitReserveFromAccrue` so sink failure cannot brick accrue paths.
- Tests (FAIL before / PASS after): `test_sole_supplier_cannot_borrow_past_ltv_cash`, `test_sole_supplier_max_borrow_respects_85_ltv`, `test_withdraw_ltv_uses_post_cash_state`, `test_remit_reserve_respects_owner_ltv`, `test_auto_remit_revert_does_not_freeze_repay`.

- [done] `PusdLoop.borrow`/`withdraw` LTV now checked on post-transfer cash (fix/oliver-ltv). Mid-transfer collateral inflation closed; sole-supplier cannot drain past 85% LTV. Proofs: `OliverLtvTest`.

## Progress — gV rebase settle + sPUSD donation guards (P0 fix #2)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards` (from `fix/oliver-ltv`).

- `gVAPURR.stake` / `unstake` call permissionless `accrue()` before share mint/burn so late stake cannot capture unpaid intervals; empty-pool stake clocks `lastRebase` (no multi-year backlog mint).
- `rebase()` remains policy-gated and delegates to `accrue()`.
- `SPUSD`: virtual shares (1e6/1) + dead shares on first deposit + `MIN_DEPOSIT`; withdraw ceils shares / redeem floors assets.
- `wgVAPURR`: same virtual + dead + `MIN_WRAP` pattern on wrap/unwrap.
- Tests (FAIL before / PASS after): `GvRebaseSettleTest` (`test_late_stake_does_not_capture_prior_interval`, `test_empty_pool_stake_does_not_award_stale_years`); `SpusdDonationGuardsTest` (`test_donation_cannot_steal_next_depositor`, `test_dust_deposit_cannot_skim_remittance`, `test_wgV_donation_cannot_steal_next_wrapper`).

- [done] `gVAPURR.stake` settles accrued rebase before share mint (fix/gv-spusd-guards). Late-stake / empty-pool emission theft closed. Proofs: `GvRebaseSettleTest`.
- [done] `SPUSD` + `wgVAPURR` donation-resistant pricing (virtual + dead shares + min deposit/wrap). Proofs: `SpusdDonationGuardsTest`.

- [P0-bug] Mint authority remains split: market V permanently assigns its minter to `PusdMarket`, while Fed staking requires mint access. That token cannot support Fed rebases; deploying the separate Fed token creates two incompatible V assets.
- [done] `gVAPURR.stake` settles via `accrue()` before share mint (see Progress — P0 fix #2).
- [P0-bug] Lithe mints fee PUSD into market inventory, then `accrue` increases PUSD supply through `drip` without burning that inventory. Decreasing `yieldReserve` alone leaves duplicate outstanding claims against unchanged V backing.
- [P0-bug] `computeSwap` rejects negative calculated spreads through `baseOffer >= askBaseAmount`. After one-sided flow, favorable counterflow can therefore revert `THIN` even with sufficient V inventory, obstructing redemption and arbitrage.
- [attack] Wrapping gV fixes only one House leg: PUSD itself rebases. Both wgV/PUSD and PUSD/USDG require explicit handling of pool-held PUSD rebases; ordinary swap accounting cannot be assumed to allocate those gains correctly.
- [done] `SPUSD` / `wgVAPURR` virtual+dead shares + min deposit/wrap (see Progress — P0 fix #2). Dust remittance skim bounded; CD time-locks still TODO for full skim resistance.
- [P0-bug] Self-issued PUSD reserves and V inventory do not establish exogenous dollar backing. The minimum 2% V swap spread is not ~par USDG redemption; a nominal PUSD runway floor cannot establish dollar solvency, and Lithe drip can consume the supposedly retained reserve.
- [attack] Oliver uses owner-fed `vapurrRate`, refreshed only on market swaps, without freshness or collateral-price safeguards. Stale or inflated valuations permit excessive borrowing; wrapping collateral would not repair this oracle dependency.
- [P0-bug] Oliver has no bad-debt write-down: exhausted collateral can leave irrecoverable debt inside supplier NAV while reserve fees accrue. Define loss allocation and bounded LOLR funding before expansion; gV/wgV collateral also remains unimplemented and needs redemption-aware valuation.
- [partial] Oliver auto-remit is try/catch isolated (`remitReserveFromAccrue`); sink revert no longer freezes repay/withdraw/liq. `remitReserve` now `_requireLtv(owner)` after exit. Lithe auto-remit isolation still open.
- [merge-blocker] Executable bonds must remain gated until payout inventory, vesting/rebase ownership, per-asset capacity, haircuts and reliable valuation are defined; stocks additionally need market-closure and corporate-action handling. Discounts against manipulable equity prices can overissue gV claims for inadequate RFV.
- [confusion] Keep ETH/USDG/stock bonds visible and printer mechanics hidden; never hide execution availability, vesting, capacity or loss exposure. gV already earns rebases, contradicting "claim gV, then stake"; TVL must exclude recursive double counting, and YOTC must disclose costs and conditional yield assumptions.
- [merge-blocker] `market_abi.rs` caches one ABI globally across clients and deployments, permanently interpreting RPC failures as legacy ABI. Selector compatibility also proves neither deployed inventory fences nor unified mint authority; those require deployment-specific verification.
- [merge-blocker] The ABI bridge is untracked and its integration remains dirty; merging branch commits alone omits it. The empty branch comparison and differing master diffstats do not establish ancestry or conflict safety, and the supplied bundle cannot validate the TVL/YOTC frontend changes.

MERGE_REC: hold — solvency bugs, split mint authority and incomplete ABI integration block merge.
