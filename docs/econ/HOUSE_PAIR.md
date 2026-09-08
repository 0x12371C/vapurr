# House pair (canon)



Relic lock 2026-09-05.



## Pair



**House AMM leg = `wgVAPURR` / `$PUSD`**, not raw rebasing `gVAPURR`.



| Leg | Token | Why |

|-----|-------|-----|

| Equity | **wgVAPURR** | Non-rebasing shares (wstETH pattern). Rebase accrues in exchange rate, not balance Ã¢â‚¬â€ LP / CL math stays honest. |

| Cash | **$PUSD** | Product dollar (Lithe index separate). |



## Hard invariant (enforce)



1. **Raw `gVAPURR` is never a House pool currency.** Uni v4 / CPMM `PoolKey` currencies must be exactly `{wgV, $PUSD}` (either order).

2. Deploy / factory path MUST call `HousePairConfig.requireHousePair` (or `HousePairFactory.validateAndMark`) before `initializePool` / seed. Revert: `RawGvNotHouseEquity` if either currency is gV.

3. Equity leg check: `requireHouseEquity(token)` Ã¢â‚¬â€ only `wgV` passes; raw gV and raw `$VAPURR` fail.

4. Wrapping gV Ã¢â€ â€™ wgV fixes **only the equity leg**. It does **not** fix cash-leg rebase accounting (see PUSD note below).



Code SoT: `contracts/HousePairConfig.sol` (+ thin `HousePairFactory`). Proofs: `contracts/test/HousePairGuard.t.sol`.



## Do not pair



- Raw **gVAPURR** in Uni v4 / CPMM Ã¢â‚¬â€ index rebase soft-taxes the pool (balance drift without swaps).

- Raw **$VAPURR** as the house equity leg once staking is live Ã¢â‚¬â€ stakers receive the dynamic 1-9%/yr policy rebase (mid ~3.5% unbound); house should quote claim-wrapped gV.



## Wrap path



1. Stake `$VAPURR` Ã¢â€ â€™ `gVAPURR` (index rebase, Fed policy only).

2. Wrap `gVAPURR` Ã¢â€ â€™ `wgVAPURR` for AMM / LP.

3. Unwrap `wgVAPURR` Ã¢â€ â€™ `gVAPURR` (more gV after rebase) Ã¢â€ â€™ unstake to `$VAPURR`.



## $PUSD rebase note (P1)



Naked `$PUSD` **is** rebasing in this stack (`PusdToken`: shares Ãƒâ€” Lithe index; `drip` lifts index). **sPUSD** is the savings vault (non-rebasing shares claiming rebasing PUSD) Ã¢â‚¬â€ not the House cash leg.



Pool-held `$PUSD` therefore accrues Lithe drip to whoever holds the pool balance. Ordinary Uni v4 reserve math does **not** allocate that gain to LPs correctly without an explicit hook / accounting path. Treat **pool-held PUSD rebase accounting** as **P1** (same class as the CODEX attack note: wgV fixes one leg only).



## Live wiring gap



`HouseLp` / `HouseSwap` constructors now take `HousePairConfig` and set equity/cash immutables from `config.wgV()` / `config.pusd()` (not `market.vapurr()`). `seed` / `unlockCallback` call `requireHousePair` before PositionManager / PoolManager work.



Still open for **live** Uni v4:

- Post-cutover: `script/TestnetHouseFollowup.s.sol` dry-runs / (gated) deploys `wgVAPURR` + `HousePairConfig` against gen-5 Lithe + gV — see `TESTNET_ROLLOUT.md` section 9. Core factory not blocked.
- Pass `HousePairConfig` into HouseLp/HouseSwap (Rust `house_deploy` / `swap_deploy` ABI must encode `pairConfig` first).

- Bootstrap must seed **wgV** inventory (wrap gV first), not raw `$VAPURR` / market.vapurr balances.

- PositionManager + Permit2 approvals for wgV on the live book; PoolManager unlock/settle e2e fork proofs.

- Pool-held `$PUSD` Lithe-index rebase settlement / LP allocation remains **P1** (settle hair is interim only).



## Hard wall (reminder)



BrowserStream / browse earn **never** call gV rebase mint. Stream pays from **already-minted treasury** earmark only. See `ROUTING.md`.

## House fee carve -> remittance (2026-09-05 sketch)

Uni v4 LP fees stay with LPs. Protocol carve (ops / hook / swapper skim) lands as **realized $PUSD inventory** in contracts/HouseFeeRemit.sol, then 
emitSurplus -> RemittanceSink (sink-level runway floor). No second local floor on House.

- creditFees pulls inventory only (never mints).
- 
emitSurplus requires a wired sink; empty reserve reverts TINY.
- Proofs: contracts/test/HouseFeeRemit.t.sol (credit+remit, empty, partial, unset sink).
- Skim adapter sketch: `contracts/HouseUniSkim.sol` â€” authorized hook/owner `skimToCredit` pulls realized $PUSD inventory into `HouseFeeRemit.creditFees` (never mints). Proofs: `HouseUniSkimTest` 4/4 (skimâ†’remit sink, stranger AUTH, zero TINY, owner path).
- Still open: full Uni v4 `IHooks` / swapper integration + deploy + Rust bootstrap (adapter is the inventory bridge only).

## Address book / IPC (2026-09-06)

Client remittance book slots are empty until Relic deploys: `TESTNET_HOUSE_FEE_REMIT`, `TESTNET_HOUSE_UNI_SKIM`, `TESTNET_FEE_ATTRIBUTION`, `TESTNET_REMITTANCE_SINK` + MarketCfg fields; snap under `remittance`. `econ-house-fee-remit` returns **NeedRemittance** until those CAs are filled. Contract sketches (`HouseFeeRemit` / `HouseUniSkim` / `FeeAttribution` / RemittanceSink) exist; P1 live enable + Uni v4 e2e still open.

**UI (2026-09-07):** House card paints remittance destination chips + **Remit fees** live CTA (`#h-remit-cta` → `econ-house-fee-remit`). Empty book stays NeedRemittance-honest; CTA is not gray-gated. Prove: `scripts/verify-remittance-book.py`.
## Operator notes

Short ops checklist: [WGV_HOUSE.md](WGV_HOUSE.md) (wrap path, green/open, do-nots).

## UI stub
### FeeAttribution who-paid UI stub (2026-09-06)

`frontend/pusd.html` House tab paints `#h-attrib` House/Lithe/Oliver chips (em-dash until `FeeAttribution.breakdown()` reads land) and updates `#h-fee-note` from `snap.remittance.configured` (empty book → NeedRemittance honesty). Live enable still Relic-gated. Prove extended 2026-09-07: scripts/verify-remittance-book.py covers who-paid chips.


`vapurr://house` (`pusd.html?tab=house`) copy locks **wgV / $PUSD** + wrap-first gate. Deploy/seed still open until live Uni + pairConfig.

### HouseLpUpgradeable ABI stub (2026-09-08 ~07:01)

Client stub on disk (House Uni v4 LP proxy impl — adopt/seed/snapshot/upgrade walls; ops; no user IPC encode yet): `crates/vapurr-econ/src/house_lp_upgradeable_impl.abi.json` + `HOUSE_LP_UPGRADEABLE_IMPL_ABI` in lib.rs. Prove stub: `scripts/verify-house-lp-upgradeable-abi.py`.

