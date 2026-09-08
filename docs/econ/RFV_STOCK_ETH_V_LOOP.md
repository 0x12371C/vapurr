# RFV stock → ETH → wgV → $VAPURR loop (keeper playbook)

Relic lock 2026-09-06 ~18:15 ET. Chain **46630**. Path A untouched (nonce 0).

## Canon (live)

1. Bonded stock sits in **ExoRfvSink** (spendable) — bonders claim **gV after 7d**, not stock.
2. Ops sells treasury STOCK → WETH on deepest STOCK/WETH fee=3000 pools (pool oracle).
3. Buy **wgV** with that WETH on Uni v3 **wgV/WETH** `0x21e99a94…5f69`, then **`wgV.unwrap` → `gV.unstake` → V** (spot V for Lithe/$PUSD).
4. Caps: ≤25% of each stock bal per pass; day notional cap; Path A nonce must stay 0.

## Live addresses

| Item | Value |
|------|-------|
| ExoRfvSink | `0xdbf2736cd318489d0cc6844445f2a6c019d6a36f` |
| BondMarket STOCKS treasury | ExoRfvSink (retargeted) |
| wgV/WETH pool fee=3000 | `0x21e99a94AD6dBa1D734BF0e1E877167EDd155f69` |
| SwapRouter02 | `0x3ce954107b1a675826b33bf23060dd655e3758fe` |
| NPM | `0x54a0ef7da351cb8fd1998e7945cc51e5825fb233` |
| Synthra factory | `0x911b4000D3422F482F4062a913885f7b035382Df` |
| wgV | `0x0599C4C4d24Bc4bbbaAdeC5CfD783Fc1cB09964a` |
| gV | `0x81840c5edfffec4e1ec2ea3c21b93d00e3daafbb` |
| WETH | `0x33e4191705c386532ba27cBF171Db86919200B94` |
| GenesisTreasury (V carve; exo **stranded**) | `0x79e3089D517E56dA30dad901eAd1BFe27ac88520` |

## Honest notes

- **No pre-existing gV/ETH or wgV/ETH book** was findable on Synthra v3 getPool or recent Uni v4 Initialize scans. Relic product path is wgV/ETH — **stood up real Uni v3 wgV/WETH** and proved swaps.
- GT still holds 15e18×5 stocks with no rescue — documented sunk; new bonds → ExoRfvSink.
- Bootstrap depth ~0.007 WETH / ~20 wgV after proves — deepen when RFV ETH grows (don't dust).

## Keeper

`CONFIRM_RFV_LOOP=1 BOBBY_PK=… python scripts/safe-stock-eth-v-loop.py`

Snap/run: `docs/econ/_rfv_unblock_run.json`

## Client ABI stub

`crates/vapurr-econ/src/exo_rfv_sink.abi.json` — keeper `spend` / `approveSpender` shapes only. Prove: `scripts/verify-exo-rfv-abi.py`. No vapurr-econ IPC cmd yet.
