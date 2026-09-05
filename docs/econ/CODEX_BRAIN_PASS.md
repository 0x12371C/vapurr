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

- [partial] Mint authority: `docs/econ/MINT_AUTHORITY.md` design + fences; full one-token role unify still open (P0 fix #3).
- [done] `gVAPURR.stake` settles via `accrue()` before share mint (see Progress — P0 fix #2).
- [done] Lithe single-count: burn fee inventory on drip before index expand (P0 fix #3). Remit spends remaining inventory only.
- [done] `computeSwap` clamps inverted CP spread; redeem/arb no longer false-THIN when inventory present (P0 fix #3).
- [attack] Wrapping gV fixes only one House leg: PUSD itself rebases. Both wgV/PUSD and PUSD/USDG require explicit handling of pool-held PUSD rebases; ordinary swap accounting cannot be assumed to allocate those gains correctly.
- [done] `SPUSD` / `wgVAPURR` virtual+dead shares + min deposit/wrap (see Progress — P0 fix #2). Dust remittance skim bounded; CD time-locks still TODO for full skim resistance.
- [P0-bug] Self-issued PUSD reserves and V inventory do not establish exogenous dollar backing. The minimum 2% V swap spread is not ~par USDG redemption; a nominal PUSD runway floor cannot establish dollar solvency, and Lithe drip can consume the supposedly retained reserve.
- [done] Oliver oracle freshness + conservative credit rate + feed jump clamp (P0 fix #4). Stale rate blocks borrow/withdrawV/liq sizing. See Progress - P0 fix #4.
- [done] Oliver bad-debt absorb + stub Fed backstop (P0 fix #4). gV/wgV collateral still unimplemented / needs redemption-aware valuation (out of slice).
- [partial] Oliver auto-remit is try/catch isolated (`remitReserveFromAccrue`); sink revert no longer freezes repay/withdraw/liq. `remitReserve` now `_requireLtv(owner)` after exit. Lithe auto-remit isolation still open.
- [merge-blocker] Executable bonds must remain gated until payout inventory, vesting/rebase ownership, per-asset capacity, haircuts and reliable valuation are defined; stocks additionally need market-closure and corporate-action handling. Discounts against manipulable equity prices can overissue gV claims for inadequate RFV.
- [confusion] Keep ETH/USDG/stock bonds visible and printer mechanics hidden; never hide execution availability, vesting, capacity or loss exposure. gV already earns rebases, contradicting "claim gV, then stake"; TVL must exclude recursive double counting, and YOTC must disclose costs and conditional yield assumptions.
- [done] `market_abi.rs` dual-probe + cache only on conclusive detect; RPC miss fails open / re-probes (P0 fix #3). Inventory fences + mint unify still need deploy-specific verification.
- [done] `market_abi.rs` tracked + wired through mint/redeem/rate paths (P0 fix #3). Frontend TVL/YOTC bundle validation still separate.


## Progress ? Lithe single-count + redeem/THIN + market_abi (P0 fix #3)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards` (slice also ok as `fix/lithe-mint-p0`).

- Lithe: `accrue` burns market fee inventory before `pusd.drip`, so holder yield and remittance share one surplus pool (no mint-and-keep + drip double claim).
- `computeSwap`: removed `baseOffer >= askBaseAmount` THIN gate; inverted CP spread clamps to 0 then MIN_STABILITY_SPREAD. Inventory/`INV` still gates redeem.
- `market_abi.rs`: dual-probe scrubbed then live hex; cache only conclusive (market, abi) pairs; RPC/dual-miss fails open to Scrubbed and re-probes (no sticky legacy).
- Mint unify: design in `docs/econ/MINT_AUTHORITY.md` (interim: keep fences, no silent dual-print; full one-token role split deferred).
- Tests: `LitheMintP0Test` (forge); `market_abi` unit tests (rust).

- [done] Lithe fee surplus single-counted (burn inventory on drip). Proofs: `LitheMintP0Test`.
- [done] `computeSwap` allows inventory redeem after one-sided flow (no false THIN). Proofs: `LitheMintP0Test`.
- [done] `market_abi` non-sticky fail-open detect + committed bridge module.
- [partial] Mint authority: design doc landed; full Fed+market one-token unify still open.

## Progress - Oliver oracle freshness + bad-debt path (P0 fix #4)

Date: 2026-09-05 (America/New_York). Branch: fix/gv-spusd-guards.

- PusdMarket: rateUpdatedAt heartbeat on feed + first-spot _spot; creditVapurrRate(maxAge) requires freshness and prefers lower of live vs pending; MAX_FEED_JUMP_WAD (50%) rejects spiked feeds.
- PusdLoop (Oliver): _px() uses creditVapurrRate(MAX_RATE_AGE=1h) so borrow / withdrawV / liq sizing revert STALE on stale oracle; snapshot falls back to raw rate if stale.
- Bad debt: absorbBadDebt sweeps dust collat, try/catches optional IFedBackstop.coverBadDebt, then socializes residual (badDebtSocialized); does not freeze repay for others.
- Tests: OliverOracleBadDebtTest (test_stale_rate_blocks_borrow, test_stale_rate_blocks_withdraw_v, test_stale_blocks_liquidate_sizing, test_pending_devaluation_tightens_before_swap, test_feed_jump_clamp_rejects_spike, test_absorb_bad_debt_does_not_freeze_repay, test_absorb_with_reverting_backstop_still_clears, test_absorb_prefers_backstop_cover, test_fresh_feed_restores_borrow_room_check).

- [done] Oliver oracle freshness + conservative pending devaluation + feed jump clamp. Proofs: OliverOracleBadDebtTest.
- [done] Bounded LOLR hook (IFedBackstop) + socialized write-down via absorbBadDebt. Proofs: OliverOracleBadDebtTest.

MERGE_REC: hold — solvency bugs, split mint authority and incomplete ABI integration block merge.
