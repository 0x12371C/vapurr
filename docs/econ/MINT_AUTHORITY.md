# Mint authority (V + PUSD) ? design

Relic lock 2026-09-05. Interim design until Fed + market share one V token.
Cross-refs: `ROUTING.md` wall 1, `PUSD_V_REDEEM.md`, `GvFed.sol`, `PusdMarket.sol`.

## Problem

Today there are **two** `VapurrToken` deployments in source:

| Surface | Token | Minter | Prints V? |
|---------|-------|--------|-----------|
| `PusdMarket` | embedded, `minter` **immutable** = market | market only | **No** on redeem (inventory unwrap). Market never calls `vapurr.mint` after genesis. |
| `GvFed` | standalone, `minter` **mutable** via `setMinter` | Fed policy / gV | **Yes** ? sole inflation path (`gVAPURR.accrue` ? `vapurr.mint` to stakers). |

Pointing Fed staking at the market's embedded token is impossible (immutable minter = market). Deploying a second Fed V creates **two incompatible V assets** (silent dual-print risk if both are treated as "the" V).

`$PUSD` is market-minted only (`PusdToken.minter` = market). Lithe drip expands PUSD via index; fees are single-counted (inventory burn on drip / remit) ? see Lithe P0.

## Target (one-token unify)

1. **One** canonical `$VAPURR` with a **role / setMinter** interface (Fed's `setMinter` pattern).
2. Roles (not dual silent printers):
   - **Fed / gV policy** ? sole `mint` for the 3.5%/yr staker rebase.
   - **Market** ? `take` / `give` / burn-unwrap inventory only; **no** inflationary `mint` on redeem.
   - Optional **treasury seeder** ? one-shot genesis / inventory fund under governance.
3. Market deploy takes an existing V address (or receives minter role after factory deploy) instead of `new VapurrToken()` with immutable self-minter.
4. PUSD stays market-minter; do not give Fed a PUSD print path.

## Hard fences (now and after unify)

- `swapPusdToV` = inventory unwrap only (`INV` if thin). Never `vapurr.mint`.
- `swapVToPusd` locks V into market inventory (no burn) so redeem can pay out.
- Browse / BrowserStream transfers already-minted float only ? never triggers rebase mint.
- Lithe fee surplus is single-counted: mint fee into inventory **xor** consume via drip burn **xor** remit ? same unit never pays twice.

## Interim (this PR)

- **No silent dual-print in product paths:** market redeem fence + Fed-only rebase remain the operational rule.
- Tests keep using Fed's standalone `VapurrToken` for gV; market tests use market-embedded V. Do not wire them as one asset until unify lands.
- Shared interface already exists in Fed as `IVapurrMint` (`mint` / `transfer` / `balanceOf` / ?). Unify PR should extract that (plus `setMinter` / roles) to a shared file and make `PusdMarket` consume it.
- Deployment checklist: never label two V addresses as fungible; never assign Fed rebase minter and market inventory ops without an explicit role split.

## Out of scope here

Full factory rewrite, migration of live gen-4 book, and House/Oliver collateral retargeting onto a shared V. Prefer complete Lithe / THIN / ABI fixes first.
