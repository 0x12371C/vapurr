# Mint authority (V + PUSD) — seigniorage Lithe

Relic lock 2026-09-05 (seigniorage rewrite). Lithe is Terra-style seigniorage:
**burn $VAPURR → mint $PUSD**; **burn $PUSD → mint $VAPURR**.
Dynamic gV rebase (1–9% bond dial) remains an **additional** V printer to stakers — not the sole printer.
Cross-refs: `ROUTING.md`, `PUSD_V_REDEEM.md`, `GvFed.sol`, `IVapurrMinter.sol`, `PusdMarketFed.sol`.

No Luna / UST / Olympus product names in UI or docs. No silent 46630 broadcast.

## Mint authority diagram

```
                    +------------------+
                    |  Fed VapurrToken |
                    |  (canonical V)   |
                    +--------+---------+
                             |
           +-----------------+------------------+
           |                                    |
           v                                    v
  +------------------+               +----------------------+
  | policy minter    |               | marketMinter         |
  | (`minter` = gV)  |               | (Lithe / PusdMarketFed)|
  +--------+---------+               +----------+-----------+
           |                                    |
           | gV.accrue / rebase                 | swapVToPusd:  burn V, mint PUSD
           | inflate to stakers                 | swapPusdToV:  burn PUSD, mint V
           | (bond dial 1-9%/yr)                |
           v                                    v
     staker balances                      traders / peg rail

  BrowserStream: transfer-only float (never minter)
  LegacyVConverter: pre-funded cutover inventory (never minter)
  PUSD: always market-minted (`PusdToken.minter` = Lithe)
```

| Role | Who | Prints V? | Burns V? |
|------|-----|-----------|----------|
| Policy minter | gV after handoff | Yes — staker rebase | No (unused) |
| Market minter | Lithe (`PusdMarketFed`) | Yes — on PUSD redeem | Yes — on PUSD expand |
| BrowserStream | distributor | No | No |
| LegacyVConverter | — | No | No |

`setMinter` / `setMarketMinter` are callable only by the current **policy** minter.
Factory order: genesis mint → fund converter → `setMarketMinter(Lithe)` → `setMinter(gV)`.

## Embedded gen-4 book (`PusdMarket`)

Source seigniorage: market is immutable self-minter of embedded V — burn on expand, mint on redeem.
Live gen-4 bytecode may still be inventory until Relic-approved cutover. Do not treat embedded V and Fed V as fungible.

## Hard fences (still Relic locks)

- USDG = bond treasury intake only (no V/USDG cash books).
- DevFund 200k → Oliver collateral → PUSD draw only; NoMarketSell.
- BrowserStream = already-minted float drip only.
- Launch pairs: V/ETH, V/NVDA, V/AMD.
- Lithe fee surplus single-counted (drip burn xor remit).
- UI honesty: only real on-chain addresses.
- No silent mainnet/testnet cutover from factory alone.

## Cutover deployment (still open)

Gen-5 source is seigniorage Lithe + dual printers. Live 46630 stays gen-4 until Relic-approved CutoverDeploy.

1. Factory deploy of canonical V + gV/policy + Lithe + converter + migrator + Oliver.
2. Roles after handoff: **gV** = policy minter; **Lithe** = marketMinter (seigniorage).
3. Follow-ups: wgV + HousePairConfig + House; setRemittance; retarget address book only after verify.
