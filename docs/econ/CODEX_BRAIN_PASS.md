# Codex brain pass (econ hard knots)

## Re-review timestamp
Date: 2026-09-05 ~02:55 America/New_York. Model: gpt-6-astra (high). Mode: brain-only, no patches.
Branch: `fix/gv-spusd-guards` @ `ef87976`+ (bonds skeleton pending commit).
Note: Codex read-only sandbox blocked independent file probes; verdict based on progress contracts below + prior closed proofs as reported.

### Prior P0 status (closed vs open/partial)
- CLOSED -- Oliver post-action LTV and reserve-remit owner health; reported regression proofs cover both.
- CLOSED -- gV pre-mint/burn accrual and empty-pool clock prevent prior-interval capture.
- CLOSED -- sPUSD/wgV donation guards address cited theft paths; full timed-remittance skim resistance remains outside this closure.
- CLOSED -- Lithe burns fee inventory before drip, removing the identified double claim.
- CLOSED -- Redeem false-THIN gate removed; inventory constraints remain.
- CLOSED -- market_abi wired through mint/redeem/rate with conclusive-only caching; live compatibility remains deployment-dependent.
- CLOSED -- Oliver freshness, conservative pending valuation and feed-jump clamp implemented.
- CLOSED -- Oliver bad-debt write-down and failure-isolated optional backstop implemented; funded coverage is not established.
- PARTIAL -- Auto-remit isolation closes Oliver failure propagation; Lithe isolation remains unconfirmed.
- CLOSED -- Realized-only Oliver remittance + sink-level RunwayFloor on consolidated RemittanceSink RFV (cross-branch floor). Exogenous USDG backing still open.
- PARTIAL -- Fed single-minter enforcement implemented; live market still has distinct embedded V.
- PARTIAL -- House raw-gV gate + HouseLp/HouseSwap wgV wiring landed; live Uni v4 deploy/bootstrap and PUSD rebase accounting remain open.
- OPEN -- Self-issued PUSD/V inventory does not establish exogenous dollar backing or par redemption.
- PARTIAL -- BondMarket gated skeleton: inventory/capacity/haircut/enabled=false; live enable still blocked on valuation + stock handling.

### Remaining P0/P1 merge blockers (max 12)
- [P0] Live one-token migration: replace/migrate gen-4 `0x47Ac...` embedded V and retarget consumers; distinct V addresses cannot be treated as fungible.
- [P1] HouseLp/HouseSwap Solidity equity wiring to wgV + PairConfig gate landed; still need live pairConfig deploy, Rust deploy ABI/bootstrap wgV, Permit2/PM e2e, PUSD rebase settlement.
- [P1] Pool-held PUSD: define and prove rebase settlement/allocation for House and `$PUSD`/USDG pools.
- [P0] Bond execution: `BondMarket` gate landed (disabled/capacity0 default; inventory-only payout). Keep live tabs disabled until reliable RFV valuation + stock closure handling; vesting claim path is inventory transfer only.
- [P0] External backing / solvency: sink-level cross-branch floor accounting landed; exogenous USDG (or equivalent) backing and true dollar solvency still not established by nominal PUSD RFV.

### MERGE_REC
**hold** -- Local fixes substantially close prior defects, but live token unification and consolidated backing remain unresolved, and bond execution must stay gated.

---
Date: 2026-09-05 (America/New_York). Model: gpt-6-astra, high effort. Mode: brain-only, no patches.
Branches in scope: `feat/bonds-ux-map`, `fix/pusdloop-routing-gaps`.



## Progress - BondMarket gated skeleton (executable partial)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards`.

- `contracts/BondMarket.sol`: `BondAssetTag` ETH/USDG/STOCKS; quote discount+vesting; `bond()` pulls exogenous asset to treasury/RFV sink; pays gV/wgV **from pre-funded inventory only** (no mint); reverts on disabled / capacity 0 / CAP / INV; haircut + Fed `priceWad`; `enabled` default false.
- Vesting: reserved payout until `claim` after unlock — transfer from bond book, not Fed mint.
- Proofs: `BondMarketTest` (`test_disabled_market_reverts`, `test_enabled_with_inventory_succeeds_without_minting_fed_supply`, `test_capacity_respected`, `test_haircut_reduces_credited_and_payout`, inventory + ETH/STOCKS gated defaults).
- Docs: `BONDS.md` aligned to gated reality (not a fake open market).

- [partial] Bonds executable framework gated; do **not** enable live tabs until valuation oracles + stock handling land.
- [remaining] Reliable RFV valuation; stock closure/corporate-action; optional rebase-ownership polish if payout is live rebasing gV.

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

- [done-interim] Mint authority: Fed `IVapurrMinter` / zero-or-one `setMinter`; gV sole inflation minter; stream cannot mint; market redeem does not touch Fed supply. Proofs: `MintAuthorityTest`. Full one-token live unify still open (0x47Ac…).
- [done] `gVAPURR.stake` settles via `accrue()` before share mint (see Progress — P0 fix #2).
- [done] Lithe single-count: burn fee inventory on drip before index expand (P0 fix #3). Remit spends remaining inventory only.
- [done] `computeSwap` clamps inverted CP spread; redeem/arb no longer false-THIN when inventory present (P0 fix #3).
- [partial] Wrapping gV fixes only the House **equity** leg (PairConfig now rejects raw gV). Naked `$PUSD` still rebases (Lithe index); pool-held PUSD rebase accounting remains **P1** for wgV/PUSD and PUSD/USDG.
- [done] `SPUSD` / `wgVAPURR` virtual+dead shares + min deposit/wrap (see Progress — P0 fix #2). Dust remittance skim bounded; CD time-locks still TODO for full skim resistance.
- [P0-bug] Self-issued PUSD reserves and V inventory do not establish exogenous dollar backing. The minimum 2% V swap spread is not ~par USDG redemption; a nominal PUSD runway floor cannot establish dollar solvency, and Lithe drip can consume the supposedly retained reserve.
- [done] Oliver oracle freshness + conservative credit rate + feed jump clamp (P0 fix #4). Stale rate blocks borrow/withdrawV/liq sizing. See Progress - P0 fix #4.
- [done] Oliver bad-debt absorb + stub Fed backstop (P0 fix #4). gV/wgV collateral still unimplemented / needs redemption-aware valuation (out of slice).
- [partial] Oliver auto-remit is try/catch isolated (`remitReserveFromAccrue`); sink revert no longer freezes repay/withdraw/liq. `remitReserve` now `_requireLtv(owner)` after exit. Lithe auto-remit isolation still open.
- [partial] BondMarket skeleton enforces inventory/capacity/haircut/`enabled` gate (proofs: BondMarketTest). Live enable still blocked: reliable valuation + stock market-closure/corporate-action; discounts against manipulable prices can overissue if Fed opens early.
- [confusion] Keep ETH/USDG/stock bonds visible and printer mechanics hidden; never hide execution availability, vesting, capacity or loss exposure. gV already earns rebases, contradicting "claim gV, then stake"; TVL must exclude recursive double counting, and YOTC must disclose costs and conditional yield assumptions.
- [done] `market_abi.rs` dual-probe + cache only on conclusive detect; RPC miss fails open / re-probes (P0 fix #3). Inventory fences + mint unify still need deploy-specific verification.
- [done] `market_abi.rs` tracked + wired through mint/redeem/rate paths (P0 fix #3). Frontend TVL/YOTC bundle validation still separate.


## Progress ? Lithe single-count + redeem/THIN + market_abi (P0 fix #3)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards` (slice also ok as `fix/lithe-mint-p0`).

- Lithe: `accrue` burns market fee inventory before `pusd.drip`, so holder yield and remittance share one surplus pool (no mint-and-keep + drip double claim).
- `computeSwap`: removed `baseOffer >= askBaseAmount` THIN gate; inverted CP spread clamps to 0 then MIN_STABILITY_SPREAD. Inventory/`INV` still gates redeem.
- `market_abi.rs`: dual-probe scrubbed then live hex; cache only conclusive (market, abi) pairs; RPC/dual-miss fails open to Scrubbed and re-probes (no sticky legacy).
- Mint unify: Fed single-minter enforceable (`IVapurrMinter` + `MintAuthorityTest`); market still embedded-V until live migration; full one-token role split deferred.
- Tests: `LitheMintP0Test` (forge); `market_abi` unit tests (rust).

- [done] Lithe fee surplus single-counted (burn inventory on drip). Proofs: `LitheMintP0Test`.
- [done] `computeSwap` allows inventory redeem after one-sided flow (no false THIN). Proofs: `LitheMintP0Test`.
- [done] `market_abi` non-sticky fail-open detect + committed bridge module.
- [done-interim] Fed single-minter pattern + docs; [remaining] live gen-4 market `0x47Ac…` still embedded-V — migration/redeploy required for one-token unify.

## Progress - Oliver oracle freshness + bad-debt path (P0 fix #4)

Date: 2026-09-05 (America/New_York). Branch: fix/gv-spusd-guards.

- PusdMarket: rateUpdatedAt heartbeat on feed + first-spot _spot; creditVapurrRate(maxAge) requires freshness and prefers lower of live vs pending; MAX_FEED_JUMP_WAD (50%) rejects spiked feeds.
- PusdLoop (Oliver): _px() uses creditVapurrRate(MAX_RATE_AGE=1h) so borrow / withdrawV / liq sizing revert STALE on stale oracle; snapshot falls back to raw rate if stale.
- Bad debt: absorbBadDebt sweeps dust collat, try/catches optional IFedBackstop.coverBadDebt, then socializes residual (badDebtSocialized); does not freeze repay for others.
- Tests: OliverOracleBadDebtTest (test_stale_rate_blocks_borrow, test_stale_rate_blocks_withdraw_v, test_stale_blocks_liquidate_sizing, test_pending_devaluation_tightens_before_swap, test_feed_jump_clamp_rejects_spike, test_absorb_bad_debt_does_not_freeze_repay, test_absorb_with_reverting_backstop_still_clears, test_absorb_prefers_backstop_cover, test_fresh_feed_restores_borrow_room_check).

- [done] Oliver oracle freshness + conservative pending devaluation + feed jump clamp. Proofs: OliverOracleBadDebtTest.
- [done] Bounded LOLR hook (IFedBackstop) + socialized write-down via absorbBadDebt. Proofs: OliverOracleBadDebtTest.


## Progress - shared RunwayFloor + realized-only remit (P0 fix #5)

Date: 2026-09-05 (America/New_York). Branch: fix/gv-spusd-guards.

- One shared `RunwayFloor` SoT: Oliver + Lithe wire the same instance (`IRunwayView` / `remittable` alias). Dual per-branch floors rejected.
- Oliver `remitReserve`: `pendingReserve` on accrue, `realizedReserve` on repay/liq/unwind (interest-first via `_realizeFromRepay`), then shared `runway.surplus`. Unpaid pending cannot remit; sole-owner cash still remittable (no third-party depositor claim).
- Lithe `remitSurplus`: still inventory-backed `yieldReserve` only, gated by the same floor.
- INVARIANT documented in `Remittance.sol` + `ROUTING.md`: remittance RFV is realized surplus above floor; never unpaid claims / depositor principal (circular RFV).
- Tests: `RunwayRfvTest` (`test_oliver_and_lithe_share_same_runway_floor`, `test_cannot_remit_unpaid_interest_from_depositor_cash`, `test_realized_remit_respects_floor_and_user_claims`, `test_lithe_cannot_remit_below_shared_floor`, `test_liq_with_accrued_interest_realizes_reserve`); RoutingFences accrue path realizes via repay before remit.

- [done] Shared runway floor + realized-only Oliver remit. Proofs: `RunwayRfvTest`.
- [done] Liq / unwind / `_burnDebt` (incl. backstop cover) call `_realizeFromRepay` same as `repay`; interest collected on liq becomes remittable surplus. Proof: `test_liq_with_accrued_interest_realizes_reserve`.

## Progress - sink-level RFV / cross-branch floor (P0)

Date: 2026-09-05 (America/New_York). Branch: fix/gv-spusd-guards.

- `RemittanceSink` is consolidated RFV SoT (`ITreasuryRfv`: `accountedRfv` / `retainedFloor` / `surplus`). Floor enforced on sink balance via `forwardSurplus` (cannot drain below floor).
- Oliver + Lithe remit **all realized** surplus into one sink; no second local `runway.surplus` gate on branch pools.
- Realized-only Oliver invariant kept (`pendingReserve` not remittable; user claims stay backed).
- Proofs: `RunwayRfvTest` (`test_two_branches_remit_one_sink_floor`, `test_realized_remit_respects_floor_and_user_claims`, `test_lithe_cannot_remit_below_shared_floor`, unpaid-interest + liq realize); RoutingFences floor tests retargeted to sink forward.
- Docs: `ROUTING.md` sink-level retain; hard wall #4 updated.

- [done] Sink-level cross-branch floor on consolidated remittance cash. Proofs: `RunwayRfvTest` + RoutingFences.
- [remaining / honest gap] Exogenous USDG (or equivalent) external backing still not in this slice — nominal $PUSD in the sink is not dollar solvency.




## Progress - House rebase-safe pairing gate (practical slice)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards`.

- Hardened `docs/econ/HOUSE_PAIR.md`: invariant that raw gV is never a House pool currency; factory must call PairConfig before init; wrap fixes equity leg only.
- `contracts/HousePairConfig.sol` + thin `HousePairFactory`: `requireHouseEquity` / `requireHousePair` revert `RawGvNotHouseEquity` on raw gV; accept only `{wgV, $PUSD}`.
- Proofs: `HousePairGuardTest` (`test_raw_gV_not_accepted_as_house_equity`, `test_raw_gV_not_accepted_in_house_pair`, `test_factory_rejects_raw_gV_pool`, `test_wgV_pusd_accepted_as_house_pair`).
- Verified: naked `$PUSD` **is** rebasing (`PusdToken` shares x Lithe index). **sPUSD** is the vault. Pool-held PUSD rebase accounting = **P1** (not closed by wgV wrap).
- `HouseLp` / `HouseSwap` headers document LIVE GAP (still `market.vapurr()`/`pusd()`).

- [done] House PairConfig/factory gate + docs invariant + raw-gV rejection proofs.
- [P1] Pool-held `$PUSD` Lithe-index rebase accounting (House + `$PUSD`/USDG books). Ordinary Uni v4 reserves do not allocate drip gains to LPs.
- [done-wiring] HouseLp/HouseSwap equity = wgV + PairConfig before init/seed (see Progress below). [blocked-live-v4] Permit2/PM e2e + Rust pairConfig deploy + bootstrap wgV + PUSD rebase hook still open.


## Progress - HouseLp/HouseSwap equity = wgV (P1 wiring)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards`.

- `HouseLp` / `HouseSwap`: constructor takes `pairConfig`; immutables `wgV` + `pusd` from config; `requireHousePair` in constructor and again before `initializePool`/seed (`HouseLp`) or in `unlockCallback` (`HouseSwap`). No longer reads `market.vapurr()` for the equity leg. Cash leg remains `$PUSD`.
- Compile scripts (`compile-house.mjs` / `compile-swap.mjs`) include `HousePairConfig.sol` for hex rebuild.
- Proofs: `HouseLpWiringTest` + `HouseSwapWiringTest` (config wiring, snapshot wgV token, seed gate before mock Posm, reject zero config). Guard suite still green.
- Docs: `HOUSE_PAIR.md` live gap narrowed to deploy/Rust/bootstrap + PUSD rebase P1.

- [done] House Solidity equity currency = wgV via PairConfig; PairConfig called before init/seed path.
- [P1 remaining] Pool-held `$PUSD` rebase settlement/allocation (interim settle hair only).
- [LIVE GAP] Uni v4 PM/Permit2 e2e fork proofs; Rust deploy must pass `pairConfig`; bootstrap must wrap to wgV before seed; cfg book needs pairConfig/wgV addresses.




## Progress - mint authority unify interim (P0)

Date: 2026-09-05 (America/New_York). Branch: `fix/gv-spusd-guards`.

- Shared `IVapurrMinter` + Fed `VapurrToken` zero-or-one `setMinter` (revoke via `address(0)`).
- Intended sole inflation path: hand minter to `gVAPURR`; RebasePolicy triggers rebase; BrowserStream transfers only.
- Market comments: embedded V distinct until migration; `swapPusdToV` inventory unwrap never mints.
- Docs: `MINT_AUTHORITY.md` done-vs-gap; live gen-4 `0x47Ac…` still old market book.
- Proofs: `MintAuthorityTest` (only gV mints; stream/browse cannot; market redeem leaves Fed supply unchanged; revoke works).

- [done-interim] Fed-side single-minter enforceable in new code.
- [remaining-live] One-token migrate live market + retarget House/Oliver/frontend; do not treat dual V addresses as fungible until cutover.

MERGE_REC: hold — solvency bugs, split mint authority and incomplete ABI integration block merge.
