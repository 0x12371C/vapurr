# PusdLoop / Oliver vs ROUTING - gaps (audit)

> **Superseded (2026-09-05 seigniorage rewrite):** Lithe redeem is Terra-style `marketMinter` mint (not inventory/`INV`). Dual printers = Lithe seigniorage + gV policy 1â€“9%. See `MINT_AUTHORITY.md`, `PUSD_V_REDEEM.md`, `ROUTING.md` wall #1. Historical inventory-fence notes below are gen-4 / pre-cutover audit context only.


First pass by vapurrbothelper, 2026-09-05. Updated 2026-09-05 after trust fences; scrubbed again 2026-09-05 14:02 ET for HouseFeeRemit + SpusdCd landings (inventory V redeem notes below are pre-seigniorage audit context).

Scope: `contracts/PusdLoop.sol` (Oliver), adjacent Lithe/mint paths in `PusdMarket.sol`, House fee surface. Walls checked: browse never taps V mint; remittance can hit sPUSD.

## Fixed

- **V mint authority fenced on market redeem.** `PusdMarket.swapPusdToV` no longer calls `vapurr.mint`. Semantics: **burn-unwrap / inventory** â€” `swapVToPusd` locks V into market inventory (no burn); redeem returns V via `give` and reverts `INV` if inventory thin. `fundVInventory` seeds already-minted float. Sole V inflation remains Fed/gV rebase (`GvFed.sol`). Tests: `RoutingFences.t.sol` (`test_market_redeem_does_not_mint_v`, `test_market_cannot_unbounded_mint_for_browse_earn`).

- **Remittance pipe.** `IRemittance` / `RemittanceSink` + `RunwayFloor` in `contracts/Remittance.sol`. Oliver (`PusdLoop`) wires `setRemittance` / `remitReserve`; `remitOnAccrue` best-effort pushes RESERVE_BPS owner-share cash to sink on `_accrue`. Test: `test_accrue_path_can_call_remittance`.

- **Runway floor stub.** `RunwayFloor` records floor (may start at 0), `surplus(balance)` view; sink `surplus()` and vault remit gate only above floor. Test: `test_runway_floor_gates_surplus`, `test_remit_respects_runway_floor_on_vault`.

- **sPUSD liquid skeleton.** `contracts/SPUSD.sol` â€” ERC-4626-style deposit/withdraw/redeem over $PUSD; `receiveRemittance` / `creditYield` raises NAV without new shares. Time-lock CD is **landed** as `SpusdCd.sol` + `SavingsRouter.sol` (see `SPUSD.md`); address-book/IPC still open. Test: `test_spusd_deposit_and_yield_credit`.

- **BrowserStream / gV walls.** Already in `GvFed.sol` + `GvBoundaries.t.sol` (drip does not mint; browse cannot rebase). Still green alongside new fences.

- **Lithe remittance.** `PusdMarket.setRemittance` / `remitSurplus`; surplus of `yieldReserve` above runway floor goes to `IRemittance` sink (same RemittanceSink pattern as Oliver). Accrue still drips holder yield (9% cap); `remitOnAccrue` best-effort remits remaining surplus. Tests: `test_lithe_remit_surplus_to_sink`, `test_lithe_remit_respects_runway_floor`, `test_lithe_accrue_path_can_call_remittance`.

## Landed since first pass (keep audit honest)

- **House fee carve to remit.** `HouseFeeRemit.sol` + `HouseUniSkim.sol` skim authorized fees to RemittanceSink / creditFees. Live Uni v4 hook/swapper e2e + pairConfig deploy still open (`WGV_HOUSE.md`, `HOUSE_PAIR.md`).
- **Time-locked sPUSD CD.** `SpusdCd.sol` + `SavingsRouter.sol` (surplus split after runway; proportional underfunding). Proofs in `SpusdCd.t.sol` / `SavingsRouter.t.sol`. UI stub `#spusd-cd` on `bonds.html` stays live CTA; address-book/IPC wire still open.

## Still open (P1+)

- **Oliver collateral != canon gV/V.** `PusdLoop.depositV` takes raw V only — no gV/wgV collateral type yet.
- **Oliver bad-debt / Fed LOLR.** No protocol loss socialization or Fed backstop path.
- **Live savings deploy.** SavingsRouter / SpusdCd not in Rust address book or wallet IPC yet; Bonds/CD surface stays honest-empty until reviewed wiring.
- **House live Uni v4.** pairConfig into HouseLp/HouseSwap, Rust bootstrap wgV, Permit2/PM e2e (`CODEX_BRAIN_PASS.md`).


## What already aligns

- Oliver does **not** mint V â€” credit is PUSD supply/borrow/loop only; V is collateral in/out.
- Loop cannot invent cash depth past vault PUSD balance (cash-capped virtual shares).
- Lithe mint-spread funds `yieldReserve` from burn-to-mint PUSD, not from V inflation into sPUSD.
- Product docs: earn must not unbounded-print V â€” enforced by inventory fence on market redeem + BrowserStream transfer-only.
