# Mint authority (V + PUSD) — canonical cutover source

Relic lock 2026-09-05. Fed-side single-minter and the external-V market replacement are now source-tested; existing gen-4 deployments still need migration.
Cross-refs: `ROUTING.md` wall 1, `PUSD_V_REDEEM.md`, `GvFed.sol`, `IVapurrMinter.sol`, `PusdMarket.sol`.

## Problem

The live gen-4 book still has **two** incompatible `VapurrToken` deployments:

| Surface | Token | Minter | Prints V? |
|---------|-------|--------|-----------|
| `PusdMarket` | embedded, `minter` **immutable** = market | market only | **No** on redeem (inventory unwrap / `give`). Market never calls `vapurr.mint` after genesis. |
| `GvFed` | standalone, implements `IVapurrMinter`, `setMinter` (zero or one) | Fed gV after handoff | **Yes** — sole inflation path (`gVAPURR.accrue` → `vapurr.mint` to stakers). |
| `PusdMarketFed` | constructor-supplied Fed V | gV after handoff | **No** — market only moves funded V inventory. |

Pointing Fed staking at the market's embedded token is impossible (immutable minter = market). Deploying a second Fed V creates **two incompatible V assets** if both are treated as "the" V.

`$PUSD` is market-minted only (`PusdToken.minter` = market). **Lithe is the V↔PUSD mint/redeem rail**; its fee inventory can expand the PUSD index through drip. Fees are single-counted (inventory burn on drip / remit) — see Lithe P0.

## Done in this interim (code enforceable)

1. **`IVapurrMinter`** (`contracts/IVapurrMinter.sol`): `minter` / `setMinter` / `mint` — **zero or one** minter (revoke with `address(0)`).
2. **Fed `VapurrToken`** implements it; only the current minter may mint or reassign; intended sole inflation role = **gV** (RebasePolicy triggers `gV.rebase` → `accrue` → mint).
3. **BrowserStream** still transfers already-minted float only — cannot mint; cannot `setMinter`.
4. **Market redeem** remains inventory-only (`swapPusdToV` → `give`); comments document dual-token interim + intended end state.
5. **Proofs:** `MintAuthorityTest` — only gV minter mints; stream/browse cannot; market redeem does not increase Fed V supply; `setMinter(0)` revokes.
6. **`PusdMarketFed`** (`contracts/PusdMarketFed.sol`) is canonical Lithe: it accepts an existing canonical V address, never exposes V mint authority, and keeps the 12-word `snapshot(address)` ABI that the desk decodes.
7. **`LegacyVConverter`** (`contracts/LegacyVConverter.sol`) is immediate and permissionless: any holder can fund already-minted canonical V, and any legacy holder can exchange 1:1 while inventory exists. It has no pause, owner, recover, or sweep path; received legacy V remains locked.
8. **`LitheCutoverMigrator`** (`contracts/LitheCutoverMigrator.sol`) atomically redeems old PUSD through legacy Lithe, converts the released V from pre-funded inventory, then mints new PUSD through canonical Lithe. It holds no mint role or synthetic bridge balance.
9. **Cutover proofs:** `CanonicalVMarketTest` verifies canonical-V swaps without supply growth, gV's sole mint role, Oliver collateral retargeting, the existing desk snapshot ABI, direct legacy-V conversion, and the full Lithe-to-Lithe PUSD route.

## Cutover deployment work (still open)

Gen-5 cutover source is landed (CanonicalLitheFactory, PusdMarketFed, converter, migrator). **Live 46630 is still gen-4** until Relic-approved CutoverDeploy. Factory is not a silent mainnet/testnet cutover.

1. Run / approve **one** factory deploy of canonical $VAPURR + gV/policy + Lithe + converter + migrator + Oliver (genesis float for converter inventory + Lithe bootstrap, then sole minter = gV).
2. Roles after handoff:
   - **Fed / gV policy** — sole ongoing mint (dynamic 1-9%/yr staker rebase; mid ~3.5% unbound).
   - **Lithe (PusdMarketFed)** — inventory 	ake/give only; PUSD market-minted; **no** V inflation on redeem.
3. **Required follow-ups the factory does not do:**
   - Deploy **wgV**, **HousePairConfig**, and **House** (ROUTING: House pairs wgV/ — not raw gV or raw V). Clearing local house/pair_config on cutover is intentional honesty.
   - Initiator must **setRemittance** (and wire sPUSD / SavingsRouter if used); remittance is not auto-bound by the factory.
4. Retarget address book + frontend only after the successor book is funded and verified; never invent live addresses in UI.
5. Verify converter + Lithe inventory cover conversion and redeem obligations before pointing traffic at gen-5.

## Hard fences (now and after unify)

- `swapPusdToV` = inventory unwrap only (`INV` if thin). Never `vapurr.mint`.
- `swapVToPusd` locks V into market inventory (no burn) so redeem can pay out.
- Legacy conversion can pay only the canonical inventory already inside `LegacyVConverter`; it does not create either asset.
- Legacy PUSD migration is old Lithe redeem → V conversion → inventory-V swap that mints canonical PUSD in one transaction; it does not mint V or create a separate PUSD claim.
- Browse / BrowserStream transfers already-minted float only — never triggers rebase mint.
- Lithe fee surplus is single-counted: mint fee into inventory **xor** consume via drip burn **xor** remit — same unit never pays twice.

## Live migration gap (gen-4)

Live gen-4 market (e.g. **`0x47Ac…`**) still runs the **old embedded-V** book: immutable market minter, separate from any Fed `VapurrToken`. `PusdMarketFed`, `LegacyVConverter`, and `LitheCutoverMigrator` are the replacement source, not retroactive changes to that address.

**Still needed before labeling one V:**

- Relic-approved factory/CutoverDeploy on the target chain (no silent 46630 cutover).
- Post-deploy: wgV + HousePairConfig + House; setRemittance / savings wiring; inventory coverage check.
- Retarget address book, Oliver, House, frontend, and market_abi only after verification.
- Never treat live gen-4 market V and Fed V as fungible until that cutover lands.

## Out of scope here

Live contract deployment, address-book replacement, House migration, and release packaging. This source change makes the one-token market and converter available for that execution path.
