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

---

## Ordered deploy checklist

Execute in order. Record each address in `docs/STATUS.md` only after a successful, approved broadcast.

### 0. Preconditions

- [ ] Operator wallet funded on **46630**
- [ ] `CONFIRM_TESTNET_DEPLOY` **unset** for dry-runs
- [ ] Gen-4 market `0x47Aca529…3617` still treated as live until cutover flag
- [ ] Parallel tracks ready: `LaunchBootstrap`, `DevFundStream`, `ExogenousPairRegistry` (do not thrash those files mid-flight)

### 1. Fed V + gV + RebasePolicy (dynamic 1–9%)

- [ ] Deploy `VapurrToken` (Fed V)
- [ ] Deploy `RebasePolicy` (floor **1%/yr**, ceiling **9%/yr**, mid **3.5%** unbound — see `POLICY_RATE.md`)
- [ ] Deploy `gVAPURR`; `policy.bindGV(gV)`
- [ ] **Do not** `setMinter(gV)` until genesis mint + DevFund allocation complete

### 2. Lithe — `PusdMarketFed` behind upgradeable proxy (vanity)

- [ ] Deploy `PusdMarketFedUpgradeable` **implementation** (from a non-vanity key, or CREATE2)
- [ ] Deploy `ERC1967Proxy(impl, initialize(vapurr, rate, owner))`
  - **Preferred vanity land (path A):** STATUS deployer **nonce 0** CREATE of the proxy → exact vanity (no salt)
  - **CREATE2 (path B):** salt-hunt with fixed `initCodeHash` — `script/VanityCreate2Hunt.s.sol`
- [ ] Verify proxy `owner`, `vapurr`, `pusd`, `litheVersion()==1`
- [ ] Fund Lithe V inventory (bootstrap slice)

### 3. Oliver (`PusdLoop`)

- [ ] Deploy Oliver against **proxy** market address (not the impl)
- [ ] `setOwner` to rollout owner
- [ ] DevFund path later locks V as Oliver collateral only (`DEV_FUND.md`)

### 4. BondMarket (USDG bond-only)

- [ ] Deploy `BondMarket` wired to gV / policy utilization signal
- [ ] Confirm **USDG `BondAssetTag` only** — no PUSD/USDG pool
- [ ] Bind policy → bond book for dynamic 1–9% rate

### 5. Remittance + savings

- [ ] Deploy / wire `Remittance` sink + runway floor
- [ ] `market.setRemittance(sink, runway, autoRemit)`
- [ ] `SavingsRouter` / sPUSD / CD as per `EARNINGS_ENGINE.md` (post-floor split)
- [ ] Not auto-wired by factory — initiator step

### 6. DevFund (200k stream → Oliver collateral only)

- [ ] Genesis mint includes **200_000 V** DevFund allocation to initiator (factory path) **before** `setMinter(gV)`
- [ ] `LaunchBootstrap`: `DevFundStream` fund + `startStream`
- [ ] Unlock settles **only** into Oliver collateral; recipient draws **$PUSD** only
- [ ] Distinct from BrowserStream (50k / 3y treasury float)

### 7. Exogenous pair registry — V/ETH + V/NVDA + V/AMD

- [ ] `ExogenousPairRegistry` via `LaunchBootstrap`
- [ ] Register genesis books: **V/ETH**, **V/NVDA**, **V/AMD** (trading / POL — not bond purchase)
- [ ] Ban USDG / PUSD as exogenous pair legs
- [ ] Optional minimal POL seed (`seedPol_`)

### 8. Minter handoff

- [ ] `canonicalV.setMinter(gV)` — gV sole V inflation path
- [ ] Policy owner = rollout owner
- [ ] Snapshot desk ABI (`snapshot(address)` 12 words) against proxy

### 9. House / wgV follow-up (not in factory)

- [ ] House pairs are **wgV / $PUSD** (`HOUSE_PAIR.md`, `WGV_HOUSE.md`) — not raw gV
- [ ] Deploy / wire `HousePairConfig` + House Uni path **after** core cutover
- [ ] Remittance skim / fee attribution as separate PR
- [ ] Clear gen-4 house/pair_config from local cutover so books do not mix

### 10. Cutover / UI honesty

- [ ] Approved CutoverDeploy only — no silent prod
- [ ] UI / desk address book updated to gen-5 proxy + Fed V
- [ ] Gen-4 addresses marked retired in STATUS after cutover
- [ ] Migrator / `LegacyVConverter` inventory route verified on fork first

---

## Dry-run commands

From `contracts/`:

```powershell
# Plan only (default) — no broadcast
forge script script/TestnetRollout.s.sol:TestnetRollout -vv

# Optional CREATE2 notes / hunt (still no broadcast)
forge script script/VanityCreate2Hunt.s.sol:VanityCreate2Hunt -vv

# LIVE broadcast — explicit gate required
$env:CONFIRM_TESTNET_DEPLOY = "1"
# forge script script/TestnetRollout.s.sol:TestnetRollout --rpc-url $TESTNET_RPC --broadcast
```

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

## Related

- TESTNET_PROXY_46630.md — CREATE2 factory / salt miner companion (parallel)
- scripts/mine-lithe-vanity.ps1 — cast-assisted salt search

- `STATUS.md` — live gen-4 addresses + vanity line
- `POLICY_RATE.md` — 1–9% bond-utilization policy
- `DEV_FUND.md` — 200k → Oliver collateral
- `BONDS.md` / `ROUTING.md` — USDG bond-only lock
- `TESTNET_SHAPE.md` — historical LP shape (gen-4 context)
- Contracts: `PusdMarketFedUpgradeable`, `proxy/ERC1967Proxy`, `LaunchBootstrap`, `CanonicalLitheFactory`
