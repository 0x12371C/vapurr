# Mint authority (V + PUSD) — single-minter interim

Relic lock 2026-09-05. Fed-side single-minter ships now; full Fed+market one-token unify still needs live migration.
Cross-refs: `ROUTING.md` wall 1, `PUSD_V_REDEEM.md`, `GvFed.sol`, `IVapurrMinter.sol`, `PusdMarket.sol`.

## Problem

Source still has **two** `VapurrToken` deployments:

| Surface | Token | Minter | Prints V? |
|---------|-------|--------|-----------|
| `PusdMarket` | embedded, `minter` **immutable** = market | market only | **No** on redeem (inventory unwrap / `give`). Market never calls `vapurr.mint` after genesis. |
| `GvFed` | standalone, implements `IVapurrMinter`, `setMinter` (zero or one) | Fed gV after handoff | **Yes** — sole inflation path (`gVAPURR.accrue` → `vapurr.mint` to stakers). |

Pointing Fed staking at the market's embedded token is impossible (immutable minter = market). Deploying a second Fed V creates **two incompatible V assets** if both are treated as "the" V.

`$PUSD` is market-minted only (`PusdToken.minter` = market). Lithe drip expands PUSD via index; fees are single-counted (inventory burn on drip / remit) — see Lithe P0.

## Done in this interim (code enforceable)

1. **`IVapurrMinter`** (`contracts/IVapurrMinter.sol`): `minter` / `setMinter` / `mint` — **zero or one** minter (revoke with `address(0)`).
2. **Fed `VapurrToken`** implements it; only the current minter may mint or reassign; intended sole inflation role = **gV** (RebasePolicy triggers `gV.rebase` → `accrue` → mint).
3. **BrowserStream** still transfers already-minted float only — cannot mint; cannot `setMinter`.
4. **Market redeem** remains inventory-only (`swapPusdToV` → `give`); comments document dual-token interim + intended end state.
5. **Proofs:** `MintAuthorityTest` — only gV minter mints; stream/browse cannot; market redeem does not increase Fed V supply; `setMinter(0)` revokes.

## Target (one-token unify — still open)

1. **One** canonical `$VAPURR` with `IVapurrMinter` (Fed's pattern).
2. Roles (not dual silent printers):
   - **Fed / gV policy** — sole `mint` for the 3.5%/yr staker rebase.
   - **Market** — `take` / `give` / burn-unwrap inventory only; **no** inflationary `mint` on redeem.
   - Optional **treasury seeder** — one-shot genesis / inventory fund under governance.
3. Market deploy takes an existing V address (or receives inventory role after factory deploy) instead of `new VapurrToken()` with immutable self-minter.
4. PUSD stays market-minter; do not give Fed a PUSD print path.

## Hard fences (now and after unify)

- `swapPusdToV` = inventory unwrap only (`INV` if thin). Never `vapurr.mint`.
- `swapVToPusd` locks V into market inventory (no burn) so redeem can pay out.
- Browse / BrowserStream transfers already-minted float only — never triggers rebase mint.
- Lithe fee surplus is single-counted: mint fee into inventory **xor** consume via drip burn **xor** remit — same unit never pays twice.

## Live migration gap (gen-4)

Live gen-4 market (e.g. **`0x47Ac…`**) still runs the **old embedded-V** book: immutable market minter, separate from any Fed `VapurrToken`.

**Still needed before labeling one V:**

- Redeploy / migrate market to accept external `IVapurrMinter` V (or role-split factory).
- Retarget House / Oliver / frontend / `market_abi` at the canonical V address.
- Explicit inventory seed + cutover checklist so redeem stays non-minting across the switch.
- Never treat live market V and Fed V as fungible until that cutover lands.

## Out of scope here

Full factory rewrite, House/Oliver collateral retargeting onto a shared V, and live book migration. Prefer complete Lithe / THIN / ABI fixes first; this PR only makes the Fed-side pattern enforceable and documents the gap.