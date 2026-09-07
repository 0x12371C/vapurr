# vapurr snapshot - 2026-09-07 ~00:02 ET (vapurrbot)

Living board for agents. Prefer this + `TRACKS.md` + `STATUS.md` over old overnight kick lore.

## Ship
| Item | Truth |
|------|-------|
| Programs | **1.1.19** @ `%LOCALAPPDATA%\Programs\vapurr\vapurr.exe` (rev `e590ef3`, sha `7731576ce92c...`, **NotSigned**; Uninstall DisplayVersion repaired to match) |
| AppData channel | **1.1.19** (same rev/sha as Programs; profile VERSION.txt synced) |
| TSL public channel | **1.1.13** at `thesecretlab.app/vapurr/channel/manifest.json` (rev `e590ef3`, different sha/size) |
| Cargo / dist zip | Cargo still **1.1.10**; latest `dist/vapurr-1.1.10-windows-x64.zip` + `dist/vapurr-setup.exe` (~11:02 ET). Local repo `channel/manifest.json` missing. |
| Pack rule | **never embed mp4/webm**; House/`pack.ps1` sole packer |
| Signing | **P0 SHIP BLOCKER** — unsigned Defender hit. SignPath / OV still open. Relic: unsigned OK for build/test; hold stranger-ship until signed + security retest (`docs/RELEASE_REVIEW_2026-09-04.md`). |

## Routing / House honesty (2026-09-06)
- Synthetic VAPURR rebates + rebate rankings + unexecuted 25-bps conversion claim are **gone** (STATUS swap/bridge repair). Do not revive SNAPSHOT 1.1.3 kickback copy.
- House book fee remains **0.30%** (3000 ppm) on-chain when configured. No user rebate until an executed payout exists.
- Public `#route-catalog` seeded on swap/bridge (stock icons, network label) without unlocking private API (`acd6c72`).
- Shared `#routing-visual` Cash/Equity/Bonds/House map on Cash + Bonds + Swap/Bridge/Overview.

## Fed / econ UI stubs (landed; Relic-gated live)
- BONDS ETH/USDG/STOCKS tabs + session/early-close/Fed-ops banners; NeedBondMarket honesty.
- sPUSD CD live CTA stub + NeedSavings honesty.
- House fee→remittance note + FeeAttribution who-paid chips (em-dash until breakdown reads).
- wgV / $PUSD House visual stub (wrap-first).
- Still Relic-gated: Oliver gV/wgV collateral wire, live CD/BondMarket ABI after deploy, House Uni v4 e2e / remittance enable, SignPath, promote TSL to 1.1.19 after signed pack, live relay.

## Org
| Role | Status |
|------|--------|
| vapurrbot | HUB — Bot primary |
| House | KEEP — sole `pack.ps1` |
| Pilot | KEEP — product truth |
| KFX / PayId / Bind | ACTIVE approved organizers only (idle OK; **no respawn**) |
| Charts / Psy / Tube | PARK |
| Hard rule | **No new grok/powershell organizer spawns.** Idle = TRACKS/OVERNIGHT/inbox text |

## Canon locks
- **404 = load-fail chrome only.** **KetPay = HTTP 402 / x402 / $PUSD.** Never fuse.
- Earn claim needs **zer0ID KYC** (`thesecretlab.app/kyc`) + **`install_id`** (per-machine sybil).
- Product dollar **$PUSD**. Home chain **4663**; econ bootstrap / settle **46630** testnet.
- USDG = BondAssetTag only (no `$PUSD`/USDG pool).
- Mascot: flat lime geometric cat `#c0f800` from `frontend/mascot.png`.

## Media
- Ketflix trailers **12/12** local under `frontend/ketflix/trailers/` — **do not pack**.
- Play URL: `https://thesecretlab.app/vapurr/ketflix/trailers/{slug}.mp4`. CDN upload still open.
- Docs: `docs/ketflix/HOSTING.md`.

## Open next (Relic)
1. SignPath / signed pack, then promote TSL channel to match Programs 1.1.19
2. Hostile-page security retest before public ship
3. Live address-book fills (savings / bond_market / remittance) after reviewed deploys
4. Trailer sync to TSL when ready
