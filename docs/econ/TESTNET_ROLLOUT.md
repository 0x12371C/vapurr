# Testnet rollout — gen-5 full stack (prep)

**Chain:** Robinhood testnet **46630**.  
**Mode:** preparation + dry-run only until Relic approves CutoverDeploy.  
**Honesty gate:** **gen-4 is live** on 46630 until cutover. Do not treat gen-4 addresses as the one-token book. Do not silently broadcast.

Vanity Lithe / market proxy target (STATUS `MAINNET_MARKET_VANITY`, reused for staged rollout):

| | |
|--|--|
| Target | `0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2` |
| STATUS deployer | `0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5` |
| Verified | `VANITY == CREATE(deployer, nonce=0)` |

Proxy pattern: **UUPS + ERC1967Proxy** (`contracts/proxy/*`, `PusdMarketFedUpgradeable.sol`).

USDG is **bond / treasury intake only** (`BondAssetTag`). No USDG AMM / peg-pool.

**Script coverage legend:** `[IN SCRIPT]` = composed by `script/TestnetRollout.s.sol` dry-run / gated live path. `[MANUAL]` / `[FOLLOW-UP]` = still operator or later PR.

---

## Ordered deploy checklist

Execute in order. Record each address in `docs/STATUS.md` only after a successful, approved broadcast.

### 0. Preconditions

- [ ] Operator wallet funded on **46630**
- [x] `CONFIRM_TESTNET_DEPLOY` **unset** for dry-runs (script default; dry-run simulates stack locally with no broadcast)
- [ ] Gen-4 market `0x47Aca529…3617` still treated as live until cutover flag
- [x] Parallel tracks composed via existing `LaunchBootstrap` / `DevFundStream` / `ExogenousPairRegistry` (do not thrash those files mid-flight)

### 1. Fed V + gV + RebasePolicy (dynamic 1–9%) — [IN SCRIPT]

- [x] Deploy `VapurrToken` (Fed V) — in `TestnetRolloutDeploy`
- [x] Deploy `RebasePolicy` (floor **1%/yr**, ceiling **9%/yr**, mid **3.5%** unbound — see `POLICY_RATE.md`)
- [x] Deploy `gVAPURR`; `policy.bindGV(gV)`
- [x] **Do not** `setMinter(gV)` until genesis mint + DevFund allocation complete (enforced by step order)

### 2. Lithe — `PusdMarketFed` behind upgradeable proxy (vanity) — [IN SCRIPT]

- [x] Deploy `PusdMarketFedUpgradeable` **implementation**
- [x] Deploy `ERC1967Proxy(impl, initialize(vapurr, rate, owner))`
  - **Preferred vanity land (path A):** STATUS deployer **nonce 0** CREATE of the proxy → exact vanity (no salt) — still operator land; dry-run logs MATCH/MISS
  - **CREATE2 (path B):** salt-hunt with fixed `initCodeHash` — `script/VanityCreate2Hunt.s.sol`
- [ ] Verify proxy `owner`, `vapurr`, `pusd`, `litheVersion()==1` on live chain after approved broadcast
- [x] No Lithe redeem V inventory fund (seigniorage: `swapPusdToV` mints via `marketMinter`; see §8 handoff)

### 3. Oliver (`PusdLoop`) — [IN SCRIPT]

- [x] Deploy Oliver against **proxy** market address (not the impl)
- [x] `setOwner` to rollout owner (after wiring)
- [x] DevFund path locks V as Oliver collateral only (`DEV_FUND.md`) via LaunchBootstrap

### 4. BondMarket (USDG bond-only) — [IN SCRIPT]

- [x] Deploy `BondMarket` wired to gV payout + Fed V supply assertion
- [x] Confirm **USDG `BondAssetTag` only** — ETH/STOCKS tabs unset; no PUSD/USDG pool
- [x] `policy.bindBondMarket(bonds)` for dynamic 1–9% rate
- [ ] Live valuation oracle / capacity ops after cutover (params env-tunable: `BOND_USDG_CAPACITY`, `BOND_TREASURY`, `USDG`)

### 5. Remittance + savings — [IN SCRIPT]

- [x] Deploy / wire `RemittanceSink` + `RunwayFloor`
- [x] `market.setRemittance(sink, runway, autoRemit)` + Oliver same sink/floor
- [x] Deploy `SPUSD` + `SpusdCd` + `SavingsRouter(sink, liquid, cd)`
- [x] `sink.setForward(savingsRouter)` — post-floor split per `EARNINGS_ENGINE.md` / `SPUSD.md`
- [x] **Starts DISABLED** (`setAllocation(false, 0)`) — safe default until liquid shares are seeded / operator enables
- [ ] Operator enable + seed liquid vault (or all-CD allocation) before first `forwardSurplus` — **[MANUAL]** post-deploy

### 6. DevFund (200k stream → Oliver collateral only) — [IN SCRIPT]

- [x] Genesis mint includes **200_000 V** DevFund allocation **before** `setMinter(gV)`
- [x] `LaunchBootstrap`: `DevFundStream` fund + `startStream`
- [x] Unlock settles **only** into Oliver collateral; recipient draws **$PUSD** only
- [ ] Distinct from BrowserStream (50k / 3y treasury float) — not deployed here

### 7. Exogenous pair registry — V/ETH + V/NVDA + V/AMD — [IN SCRIPT]

- [x] `ExogenousPairRegistry` via `LaunchBootstrap`
- [x] Register genesis books: **V/ETH**, **V/NVDA**, **V/AMD** (trading / POL — not bond purchase)
- [x] Ban USDG / PUSD as exogenous pair legs (registry constructor)
- [ ] Optional minimal POL seed (`SEED_POL=1` / `seedPol_`) — off by default in dry-run

### 8. Minter handoff + cutover inventory — [IN SCRIPT]

- [x] Dual-minter handoff (match `CanonicalLitheFactory` / `MINT_AUTHORITY.md`):
  - Genesis mint complete (bootstrap float + DevFund 200k + optional cutover inventory) **before** policy handoff
  - `canonicalV.setMarketMinter(Lithe)` — Lithe seigniorage printer (while deployer still holds policy minter)
  - `canonicalV.setMinter(gV)` — gV policy inflate 1–9%
  - No Lithe redeem inventory fund — redeem mints V via `marketMinter`
- [x] Policy owner = rollout owner
- [x] `LegacyVConverter` + `LitheCutoverMigrator` composed like factory:
  - **Dry-run (CONFIRM unset):** local mock legacy market/V if `LEGACY_*` unset — **not** live gen-4 addresses
  - **Live (`CONFIRM_TESTNET_DEPLOY=1`):** requires `LEGACY_MARKET` + `LEGACY_V` + `LEGACY_V_SUPPLY` (verified vs `vapurr()` / `totalSupply()`); otherwise cutover skipped (no invented addresses)
  - Converter funded 1:1 with canonical V inventory; migrator constructed against Lithe proxy
- [ ] Snapshot desk ABI (`snapshot(address)` 12 words) against proxy on live chain
- [ ] Migrator fork verify against real gen-4 before CutoverDeploy — **[MANUAL]**

### 9. House / wgV follow-up (not in factory) — [FOLLOW-UP] / manual next

- [ ] House pairs are **wgV / $PUSD** (`HOUSE_PAIR.md`, `WGV_HOUSE.md`) — not raw gV
- [ ] Deploy / wire `HousePairConfig` + House Uni path **after** core cutover
- [ ] Remittance skim / fee attribution as separate PR
- [ ] Clear gen-4 house/pair_config from local cutover so books do not mix

### 10. Cutover / UI honesty — [MANUAL]

- [ ] Approved CutoverDeploy only — no silent prod
- [ ] UI / desk address book updated to gen-5 proxy + Fed V
- [ ] Gen-4 addresses marked retired in STATUS after cutover
- [ ] Migrator / `LegacyVConverter` inventory route verified on fork first

---

## Dry-run commands

From `contracts/`:

```powershell
# Plan + local full-stack simulation (default) — no broadcast
forge script script/TestnetRollout.s.sol:TestnetRollout -vv

# Optional CREATE2 notes / hunt (still no broadcast)
forge script script/VanityCreate2Hunt.s.sol:VanityCreate2Hunt -vv

# LIVE broadcast — explicit gate required
$env:CONFIRM_TESTNET_DEPLOY = "1"
# Also set LEGACY_MARKET / LEGACY_V / LEGACY_V_SUPPLY for cutover inventory (else skipped)
# forge script script/TestnetRollout.s.sol:TestnetRollout --rpc-url $TESTNET_RPC --broadcast
```

Optional env (dry-run deploys MockUsdg / mock exo legs when unset): `USDG`, `EXO_ETH`, `EXO_NVDA`, `EXO_AMD`, `BOOTSTRAP_V`, `RUNWAY_FLOOR`, `BOND_USDG_CAPACITY`, `BOND_TREASURY`, `DEVFUND_RECIPIENT`, `SEED_POL`, `AUTO_REMIT`, `ROLLOUT_OWNER`, `LITHE_RATE_WAD`, `CD_COUPON_BPS`, `CD_BREAK_FEE_BPS`, `CD_TERM`, `LEGACY_MARKET`, `LEGACY_V`, `LEGACY_V_SUPPLY`.

Proxy upgrade proofs:

```powershell
forge test --match-contract PusdMarketFedProxyTest -vv
```

---

## CREATE2 / vanity achievability

| Question | Answer |
|----------|--------|
| Is STATUS vanity the nonce-0 CREATE of STATUS deployer? | **Yes** (verified) |
| Can UUPS proxy land there via CREATE at nonce 0? | **Yes**, if deployer nonce is still **0** and impl already exists on another key |
| Can CREATE2 from STATUS deployer hit the same vanity? | **Only with a salt hunt** for the exact `proxy initCodeHash` (impl + init calldata fixed first). Not guaranteed inside a small iteration budget |
| If STATUS deployer nonce already > 0? | Path A dead; use path B salt hunt or accept a non-vanity proxy on testnet |

**Recommendation for staged 46630 rollout:** use path A on a fresh vanity-capable key if mainnet deployer nonce is reserved/spent; keep CREATE2 hunt script for mainnet land when impl bytecode is frozen.

---

## Dry-run notes (prep — no live addresses)

Captured from `forge script script/TestnetRollout.s.sol:TestnetRollout -vv` (CONFIRM unset).

**Now in script (local simulate, no broadcast):**

1. Fed V + RebasePolicy + gV (dynamic 1–9%)
2. Lithe impl + ERC1967Proxy (UUPS) — prefer vanity `0xC47f…EBD2`
3. Oliver (`PusdLoop`) behind market proxy
4. BondMarket (USDG BondAssetTag only) + `policy.bindBondMarket`
5. RemittanceSink + RunwayFloor + `setRemittance` on Lithe + Oliver
6. **SavingsRouter + SPUSD + SpusdCd** + `sink.setForward` — **starts DISABLED**
7. Genesis mint DevFund 200k (+ optional `BOOTSTRAP_V`) + cutover converter inventory **before** `setMinter(gV)`
8. **LegacyVConverter + LitheCutoverMigrator** (factory-shaped; dry-run mocks if `LEGACY_*` unset)
9. LaunchBootstrap: DevFundStream start + V/ETH+V/NVDA+V/AMD
10. Dual-minter: `setMarketMinter(Lithe)` then `setMinter(gV)` (no Lithe redeem inventory)

**Still manual / follow-up:**

- Enable SavingsRouter + seed liquid vault (or all-CD) before first `forwardSurplus`
- House / wgV (**manual next** — not factory)
- Live `LEGACY_*` env for real gen-4 cutover inventory (no invented addresses)
- Vanity land via STATUS deployer nonce-0 (or CREATE2 hunt)
- Relic-approved CutoverDeploy + UI address book + migrator fork verify

HONEST: gen-4 remains live on 46630 until Relic-approved CutoverDeploy. Do not invent live gen-5 addresses here.

**Last dry-run (2026-09-05 ~2:05pm ET):** `forge script script/TestnetRollout.s.sol:TestnetRollout -vv` from `contracts/` — **exit 0**, `CONFIRM_TESTNET_DEPLOY 0`, no broadcast. Local simulate composed prior stack + SPUSD/SpusdCd/SavingsRouter (`sink.setForward`, **savings enabled 0 / cdBps 0**), + LegacyVConverter+LitheCutoverMigrator (dry-run mock legacy, inventory 1e6 V). Gas used ~44.7M. Vanity MISS expected off STATUS deployer nonce-0 path. No live gen-5 addresses — dry-run only.

## Related

- TESTNET_PROXY_46630.md — CREATE2 factory / salt miner companion (parallel)
- scripts/mine-lithe-vanity.ps1 — cast-assisted salt search

- `STATUS.md` — live gen-4 addresses + vanity line
- `POLICY_RATE.md` — 1–9% bond-utilization policy
- `DEV_FUND.md` — 200k → Oliver collateral
- `BONDS.md` / `ROUTING.md` — USDG bond-only lock
- `SPUSD.md` / `EARNINGS_ENGINE.md` — savings forward + post-floor split
- `TESTNET_SHAPE.md` — historical LP shape (gen-4 context)
- Contracts: `PusdMarketFedUpgradeable`, `proxy/ERC1967Proxy`, `LaunchBootstrap`, `CanonicalLitheFactory`, `BondMarket`, `Remittance`, `SavingsRouter`, `SPUSD`, `SpusdCd`, `LegacyVConverter`, `LitheCutoverMigrator`
