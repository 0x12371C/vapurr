# Testnet CutoverDeploy READY (prep only)

**When:** 2026-09-05 ~17:55 ET  
**Branch:** `fix/gv-spusd-guards` @ `41df999a65bd9eb5ce9638506ce93b2aded99116` (worktree: `vapurr-lockpad`)  
**Chain:** Robinhood testnet **46630**  
**Broadcast:** **NO** — do not set `CONFIRM_TESTNET_DEPLOY=1` until Relic explicitly confirms.

## Checklist GO/NO-GO

| # | Item | Verdict | Notes |
|---|------|---------|-------|
| 1 | Docs + scripts read | **GO** | `TESTNET_ROLLOUT`, `CODEX_BRAIN_PASS`, `ROUTING`, `GENESIS_ALLOCATION`, `TestnetRollout.s.sol`, House follow-up script |
| 2 | Tip SHA + focused forge | **GO** | Tip `41df999`. Focused forge **43/43 PASS** (CanonicalLitheFactory, MintAuthority, GenesisTreasury, LitheMintP0, PusdMarketFedProxy, BondMarket, RunwayRfv, DevFundStream) |
| 3 | Dry-run / script path | **GO** | `forge script …:TestnetRollout -vv` exit 0; `CONFIRM_TESTNET_DEPLOY 0`; gas ~**52.4M**; savings enabled **0**; cutover mock 288k; vanity **MISS** (expected off Path A) |
| 4 | Bobby / deployer balance | **GO (gas) / NO-GO (key)** | Bobby `0x875078…D1a` ≈ **0.002 ETH**, nonce 0. GasPrice ≈ 0.01 gwei → maxGas ~200M > 52.4M. **Bobby PRIVATE_KEY not in env/keystore.** Path A sk **present** (`%LOCALAPPDATA%\vapurr\mainnet-deploy.sk` → `0x48043E…AeA5`) but **0 ETH** (nonce still 0). |
| 5 | Remaining blockers | see below | |
| 6 | This READY note | **GO** | Prep complete; broadcast still Relic-gated |

**Overall:** **PREP GO / BROADCAST NO-GO**

## Deploy command (live — Relic gate only)

```powershell
cd C:\Users\jfren\vapurr-lockpad\contracts
$env:Path = "C:\Users\jfren\.foundry\bin;" + $env:Path
$env:CONFIRM_TESTNET_DEPLOY = "1"   # ONLY after Relic explicit confirm
$env:PRIVATE_KEY = "<deployer pk>"  # bobby OR Path A after funding
$env:LEGACY_MARKET = "0x47Aca5292423e2133A3eE983aB38291de3983617"
$env:LEGACY_V = "0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585"
$env:LEGACY_V_SUPPLY = "288137000000000000000000"  # live gen-4 totalSupply ~288137
forge script script/TestnetRollout.s.sol:TestnetRollout --rpc-url https://rpc.testnet.chain.robinhood.com --broadcast -vv
```

**Dry-run (safe default):**

```powershell
# CONFIRM unset
forge script script/TestnetRollout.s.sol:TestnetRollout -vv
```

## Confirm gate

- Script: `vm.envOr("CONFIRM_TESTNET_DEPLOY", 0) == 1` else local simulate only (`TestnetRollout.s.sol`).
- Prior source brain pass: **CutoverDeploy GO** (`docs/econ/_codex_cutover_verdict.txt`, tip then `685a793`; tip now `41df999` UI-only delta; forge still green).
- House / wgV: separate `TestnetHouseFollowup` + `CONFIRM_HOUSE_FOLLOWUP` — **after** core.

## Balances (2026-09-05 ~17:55 ET)

| Addr | Role | ETH | Nonce |
|------|------|-----|-------|
| `0x875078Dba143Cf729A6b2327003BB425FD613D1a` | bobby | ~0.002 | 0 |
| `0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5` | Path A vanity deployer | **0** | 0 |
| Vanity target `0xC47f…EBD2` | CREATE(deployer,0) | n/a | — |

## Remaining blockers

1. **Deployer key wiring (P0 for live):** bobby funded but key not in process env / foundry keystore; Path A key on disk but unfunded. Relic must export bobby PK **or** fund Path A then use `mainnet-deploy.sk` (preferred for vanity MATCH).
2. **Vanity Path A land:** only if broadcast from `0x48043E` at nonce 0. Bobby broadcast → vanity MISS (acceptable for testnet).
3. **LEGACY_* required for live cutover inventory** — else cutover skipped. Live supply ≈ **288137** V (not dry-run mock address).
4. **SavingsRouter enabled-default (P1):** constructor `enabled=true`; **mitigated** — `TestnetRollout` calls `setAllocation(false,0)`. Operator must enable + seed before first `forwardSurplus`.
5. **Gen-4 still live** on 46630 until approved cutover (market `0x47Aca529…3617`).
6. **Migrator fork verify [MANUAL]** before/at cutover.
7. House Uni v4 / pairConfig / wgV bootstrap — follow-up, not core blocker.
8. Exogenous USDG dollar solvency — honesty gap, not factory shape.

## Exact next Relic action

1. Pick deployer: **fund Path A `0x48043E` ≥0.003 ETH** (vanity) **or** put **bobby** `PRIVATE_KEY` in env.  
2. Explicitly say **confirm CutoverDeploy** (then and only then set `CONFIRM_TESTNET_DEPLOY=1`).  
3. Set `LEGACY_MARKET` / `LEGACY_V` / `LEGACY_V_SUPPLY=288137e18` and run the live command above.  
4. Record addresses in `docs/STATUS.md`; then House follow-up dry-run.

Locks unchanged: Lithe seigniorage; genesis 1.2M; USDG bond-only; no silent broadcast.
