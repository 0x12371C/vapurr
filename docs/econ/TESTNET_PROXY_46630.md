# Testnet 46630 ? Lithe vanity proxy rollout PREP

Relic lock 2026-09-05. **Scripts + forge + docs only. Do NOT broadcast deploy without Relic approval.**

## Target

| Item | Value |
|------|-------|
| Chain | Robinhood testnet **46630** |
| Vanity Lithe / market proxy | `0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2` |
| Pattern | **UUPS** implementation + **ERC1967Proxy** at vanity address via **CREATE2** |

The vanity address is the **proxy** (user-facing Lithe). Implementation upgrades via `upgradeToAndCall` (owner).

## Source landed

| Piece | Path |
|-------|------|
| ERC1967 proxy + UUPS base | `contracts/proxy/ERC1967Proxy.sol` |
| CREATE2 factory | `contracts/proxy/Create2Factory.sol` |
| Upgradeable Lithe shell | `contracts/PusdMarketFedUpgradeable.sol` |
| Forge script (sim) | `contracts/script/LitheVanityDeploy.s.sol` |
| Vanity miner notes | `scripts/mine-lithe-vanity.ps1` |
| Proofs | `contracts/test/LitheProxy.t.sol` |

Non-upgradeable `PusdMarketFed` remains the forge cutover path used by `CanonicalLitheFactory` until Relic flips live gen-5 to the proxy address book.

## CREATE2 recipe

```
proxy = keccak256(0xff ++ create2Factory ++ salt ++ keccak256(proxyInitCode))[12:]

proxyInitCode = ERC1967Proxy.creationCode ++ abi.encode(implementation, initData)
initData      = abi.encodeCall(PusdMarketFedUpgradeable.initialize, (vapurr, rate, owner))
```

1. Deploy `Create2Factory` on 46630 (record address ? mining depends on it).
2. Deploy `PusdMarketFedUpgradeable` implementation.
3. Build `proxyInitCode` with that impl + init calldata (canonical V + rate + owner).
4. Mine `salt` until `computeAddress(salt, keccak256(proxyInitCode)) == 0xC47f?EBD2`.
5. `factory.deploy{value:0}(salt, proxyInitCode)` ? **only when Relic says go**.

## Simulation (no broadcast)

```
cd contracts
forge script script/LitheVanityDeploy.s.sol:LitheVanityDeploy -vvvv
# With env when ready (still dry-run unless --broadcast):
#   $env:VAPURR="0x..."; $env:LITHE_SALT="0x..."; forge script ... --rpc-url $RPC_46630
```

## Launch land (same cutover story)

Keep alongside vanity Lithe prep:

- **DevFundStream** ? 200k genesis; expansion-aware; unlocked V **Oliver collateral**; **$PUSD-only** draw (`NoMarketSell`)
- **ExogenousPairRegistry** ? V/ETH, V/NVDA, V/AMD POL books; **USDG bond-intake only** (no V/USDG or PUSD/USDG)
- **LaunchBootstrap.fundAndStart** ? after factory DevFund allocation

## Still open for live

- Relic-approved broadcast on 46630
- Salt mine hit for exact vanity (depends on factory + impl addresses)
- Wire desk/address book to proxy; migrate call sites from gen-4 `0x47Aca?`
- Full Lithe swap/remit parity on UUPS shell (shell is proxy-ready + inventory/snapshot; port remaining `PusdMarketFed` paths at cutover)
- UI honest-empty until addresses exist

## Related

- `TESTNET_SHAPE.md`, `STATUS.md`, `DEV_FUND.md`, `BONDS.md`, `ROUTING.md`
