# PROXY DEPLOY GATE (hard lock)

**Relic:** a failure of that magnitude can never happen again.
**When locked:** 2026-09-06 (post GT stranding)
**Scope:** every vapurr contract address written to `DEPLOYED` / `TESTNET_*` / BondMarket treasury / desk market.json

## Rule

**All live CAs deploy behind EIP-1967 proxies. Never launch implementation-as-live.**

Before any contract address is recorded as the user-facing / treasury / market CA:

1. Query EIP-1967 **implementation** slot on-chain.
2. Slot must be **non-zero** (points at a real impl).
3. Only then write the address into DEPLOYED / TESTNET_* / BondMarket treasury / FE book.

If the impl slot is zero → **STOP**. That address is a bare implementation (or wrong target). Do not wire it.

## Slots (EIP-1967)

| Slot | keccak name | Storage key |
|------|-------------|-------------|
| Implementation | `eip1967.proxy.implementation` | `0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc` |
| Admin | `eip1967.proxy.admin` | `0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103` |
| Beacon | `eip1967.proxy.beacon` | `0xa3f0ad74e5423aebfd80d3ef4346578335a9a72aeaee59ff6cb3582b35133d50` |

### UUPS (preferred for vapurr Lithe path)

- **Impl slot non-zero** → PASS (gate satisfied).
- Admin slot usually **zero** on stock OZ UUPS (owner lives in impl storage; upgrades via `upgradeTo` / `upgradeToAndCall`).
- **vapurr ERC1967Proxy** also sets `admin=msg.sender` at construct — Lithe on 46630 is UUPS (`proxiableUUID` = impl slot) *and* shows deployer in the admin slot. Gate still keys only on **impl ≠ 0**.
- Optional sanity: `proxiableUUID()` returns the ERC1967 impl slot.

### Transparent Proxy

- **Impl slot non-zero** → PASS.
- **Admin slot also non-zero** → note it (ProxyAdmin). Do not treat admin as the user-facing CA; users call the proxy, admin calls upgrade.

Beacon-only without impl slot is **not** enough for this gate — require the standard impl slot.

## How to prove

```powershell
# Pass expected (Lithe proxy on 46630):
powershell -File scripts/verify-proxy.ps1 -Address 0x3E2011aEc730d5b5ebc06dad38b7aecF4d43B7f1

# Fail expected (GenesisTreasury bare impl — anti-pattern):
powershell -File scripts/verify-proxy.ps1 -Address 0x79e3089D517E56dA30dad901eAd1BFe27ac88520
# exit code != 0
```

Script exits **0** only when impl slot ≠ 0. Bare impl → non-zero exit.

## GT case study (anti-pattern)

| Item | Value |
|------|-------|
| GT | `0x79e3089D517E56dA30dad901eAd1BFe27ac88520` |
| Chain | Robinhood testnet **46630** |
| EIP-1967 impl / admin / beacon | all **0x0** |
| Runtime | full Solidity dispatcher (~12KB), not thin proxy |
| Result | **75e18 stocks stranded** (15e18 × TSLA/AMD/PLTR/AMZN/NFLX) — no `rescueERC20`, no upgrade path, `NoMarketSell` on V exits |
| Detail | `docs/econ/GT_PROXY_PROBE.md` |

Bobby owns GT as a concrete locker, not an ERC1967 admin/UUPS key. Stocks cannot be swept without out-of-band chain/token admin. New STOCKS treasury → ExoRfvSink (spendable). **Path A / Oliver untouched by this gate work.**

**Contrast:** Lithe proxy `0x3E2011aEc730d5b5ebc06dad38b7aecF4d43B7f1` has EIP-1967 impl = `0xF531C5445F7142e1515d44C854dD5Cfe1Ddef311` (UUPS). That is the deploy shape.

## Checklist (every new CA)

- [ ] Deploy **implementation** first (not user-facing).
- [ ] Deploy **ERC1967Proxy** (or UUPS proxy) pointing at impl; initialize via proxy.
- [ ] `scripts/verify-proxy.ps1 -Address <proxy>` exits 0.
- [ ] Record **proxy** address only in DEPLOYED / TESTNET_* / treasury / market.json.
- [ ] Never put bare impl into those books.

## Live gen-4 proxy board (2026-09-07)

Honesty snapshot for Robinhood testnet **46630** after Oliver + House/Swap/PNS UUPS wave. Prove: `scripts/verify-proxy-book.py` (offline) and `scripts/verify-proxy-book.py --live` (RPC).

### UUPS (impl slot non-zero) — PASS

| Const | Proxy CA | Notes |
|-------|----------|-------|
| TESTNET_LOOP | `0x07d1085b545d5e1f55668a6a2EA9332233AaeC69` | Oliver `PusdLoopUpgradeable` |
| TESTNET_HOUSE | `0x603AaDFCD483aC196E2bcB158989dD5d38B24336` | `HouseLpUpgradeable` (raw `$VAPURR` shape; wgV still open) |
| TESTNET_SWAP | `0x4a00651238EAf8F849b8d8cbb7FD051a4D0f5383` | `HouseSwapUpgradeable` |
| TESTNET_PNS | `0xC0E6f3217525afc80FE89f077504D9E5377a4bB5` | `PnsRegistryUpgradeable` (fresh registry) |

Also: historical Lithe vanity proxy `0x3E2011aEc730d5b5ebc06dad38b7aecF4d43B7f1` (contrast in GT section). Live gen-4 Lithe book address remains `TESTNET_MARKET` below until a proxy cutover.

### Bare (impl slot zero) — known gate debt

| Const | CA | Risk |
|-------|-----|------|
| TESTNET_MARKET | `0x47Aca5292423e2133A3eE983aB38291de3983617` | bare Lithe impl — no upgrade path |
| TESTNET_VAPURR | `0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585` | bare token impl |
| TESTNET_PUSD | `0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e` | bare token impl |
| GT (retired treasury) | `0x79e3089D517E56dA30dad901eAd1BFe27ac88520` | stranded stocks anti-pattern |

Owner handoff for LOOP/HOUSE/PNS admin still pending (deploy key to operator) — see `docs/STATUS.md`. Do not treat bare Market/V/PUSD as gate-clean; next Relic deploy must land proxies before rewiring the book.

## Related

- `GT_PROXY_PROBE.md` — live probe of stranded GT
- `TESTNET_PROXY_46630.md` — Lithe vanity UUPS + CREATE2 prep
- `contracts/proxy/ERC1967Proxy.sol`, `UUPSUpgradeable.sol`

### ERC1967Proxy ABI stub (2026-09-08 ~04:01)

Client stub on disk (minimal proxy construct + fallback/receive for cutover of bare Lithe/V/PUSD CAs; no user IPC encode yet): `crates/vapurr-econ/src/erc1967_proxy.abi.json` + `ERC1967_PROXY_ABI` in lib.rs. Gate still: never record bare impl as live CA — prove with `scripts/verify-proxy.ps1` / `scripts/verify-proxy-book.py`. Prove stub: `scripts/verify-erc1967-proxy-abi.py`.


