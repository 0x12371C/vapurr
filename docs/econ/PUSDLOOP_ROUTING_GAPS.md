# PusdLoop / Oliver vs ROUTING - gaps (audit)

First pass by vapurrbothelper, 2026-09-05. Updated 2026-09-05 after trust fences (inventory V redeem, remittance/runway/sPUSD stubs).

Scope: `contracts/PusdLoop.sol` (Oliver), adjacent Lithe/mint paths in `PusdMarket.sol`, House fee surface. Walls checked: browse never taps V mint; remittance can hit sPUSD.

## Fixed

- **V mint authority fenced on market redeem.** `PusdMarket.swapPusdToV` no longer calls `vapurr.mint`. Semantics: **burn-unwrap / inventory** — `swapVToPusd` locks V into market inventory (no burn); redeem returns V via `give` and reverts `INV` if inventory thin. `fundVInventory` seeds already-minted float. Sole V inflation remains Fed/gV rebase (`GvFed.sol`). Tests: `RoutingFences.t.sol` (`test_market_redeem_does_not_mint_v`, `test_market_cannot_unbounded_mint_for_browse_earn`).

- **Remittance pipe.** `IRemittance` / `RemittanceSink` + `RunwayFloor` in `contracts/Remittance.sol`. Oliver (`PusdLoop`) wires `setRemittance` / `remitReserve`; `remitOnAccrue` best-effort pushes RESERVE_BPS owner-share cash to sink on `_accrue`. Test: `test_accrue_path_can_call_remittance`.

- **Runway floor stub.** `RunwayFloor` records floor (may start at 0), `surplus(balance)` view; sink `surplus()` and vault remit gate only above floor. Test: `test_runway_floor_gates_surplus`, `test_remit_respects_runway_floor_on_vault`.

- **sPUSD liquid skeleton.** `contracts/SPUSD.sol` — ERC-4626-style deposit/withdraw/redeem over $PUSD; `receiveRemittance` / `creditYield` raises NAV without new shares. Time-lock CD still TODO in `SPUSD.md`. Test: `test_spusd_deposit_and_yield_credit`.

- **BrowserStream / gV walls.** Already in `GvFed.sol` + `GvBoundaries.t.sol` (drip does not mint; browse cannot rebase). Still green alongside new fences.

- **Lithe remittance.** `PusdMarket.setRemittance` / `remitSurplus`; surplus of `yieldReserve` above runway floor goes to `IRemittance` sink (same RemittanceSink pattern as Oliver). Accrue still drips holder yield (9% cap); `remitOnAccrue` best-effort remits remaining surplus. Tests: `test_lithe_remit_surplus_to_sink`, `test_lithe_remit_respects_runway_floor`, `test_lithe_accrue_path_can_call_remittance`.

## Still open (P1+)

- **House fee carve to remit.** `HouseSwap` / Uni v4 pool fee stays in LP path; no surplus skim to runway/sPUSD. (Do not expand unless trivial.)

- **Oliver collateral != canon gV/V.** `PusdLoop.depositV` takes raw V only — no gV/wgV collateral type yet.

- **Oliver bad-debt / Fed LOLR.** No protocol loss socialization or Fed backstop path.

- **Time-locked sPUSD CD tranches.** Doc-only TODO.

## What already aligns

- Oliver does **not** mint V — credit is PUSD supply/borrow/loop only; V is collateral in/out.
- Loop cannot invent cash depth past vault PUSD balance (cash-capped virtual shares).
- Lithe mint-spread funds `yieldReserve` from burn-to-mint PUSD, not from V inflation into sPUSD.
- Product docs: earn must not unbounded-print V — enforced by inventory fence on market redeem + BrowserStream transfer-only.
