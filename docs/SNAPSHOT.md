# vapurr snapshot - 2026-09-07 ~16:40 ET (vapurrbot)

Living board for agents. Prefer this + TRACKS.md + STATUS.md over old overnight kick lore.

## Ship
| Item | Truth |
|------|-------|
| Programs | **1.1.10** @ %LOCALAPPDATA%\\Programs\\vapurr\\vapurr.exe (rev 57b32f, sha cfd7435a5984..., **NotSigned**; Uninstall DisplayVersion **1.1.10** matches) |
| AppData channel | **1.1.10** (same sha/rev) - Programs **matches** channel; profile VERSION.txt matches |
| TSL public channel | **1.1.13** at 	hesecretlab.app/vapurr/channel/manifest.json (rev e590ef3, sha 324286bf6f68...); **www.thesecretlab.app TLS expired** |
| Cargo / dist zip | Cargo **1.1.10**; local Programs/channel also **1.1.10**. TSL still lagging signed promote. |
| Pack rule | **never embed mp4/webm**; House/pack.ps1 sole packer |
| Version board | scripts/verify-version-board.py --watch PASS live Programs==channel==DisplayVersion=1.1.10; NOTE TSL SKEW 1.1.13 |
| Signing | **P0 SHIP BLOCKER** - unsigned Defender hit. SignPath / OV still open. Relic: unsigned OK for build/test; hold stranger-ship until signed + security retest (docs/RELEASE_REVIEW_2026-09-04.md). |

## Routing / House honesty (2026-09-06)
- Synthetic VAPURR rebates + rebate rankings + unexecuted 25-bps conversion claim are **gone** (STATUS swap/bridge repair). Do not revive SNAPSHOT 1.1.3 kickback copy.
- House book fee remains **0.30%** (3000 ppm) on-chain when configured. No user rebate until an executed payout exists.
- Public `#route-catalog` seeded on swap/bridge (stock icons, network label) without unlocking private API (`acd6c72`).
- Shared `#routing-visual` Cash/Equity/Bonds/House map on Cash + Bonds + Swap/Bridge/Overview.

## Fed / econ UI stubs (landed; Relic-gated live)
- BONDS ETH/USDG/STOCKS tabs + session/early-close/Fed-ops banners; NeedBondMarket honesty.
- sPUSD CD live CTA stub + NeedSavings honesty.
- House fee→remittance note + FeeAttribution who-paid chips (em-dash until breakdown reads). Prove: scripts/verify-remittance-book.py (who-paid chips).
- wgV / $PUSD House visual stub (wrap-first).
- ExogenousPairRegistry ABI stub on disk (exogenous_pair_registry + exogenous_seed_market.abi.json); prove erify-exo-pair-abi.py.
- BrowserStream ABI stub on disk (`browser_stream.abi.json`); prove `verify-browser-stream-abi.py` (2026-09-07 ~20:16 ET).
- USDG pins: TESTNET catalog vs desk `rhc::USDG` proved (`verify-usdg-pins.py`). Still Relic-gated: Oliver gV/wgV collateral wire, live CD/BondMarket ABI after deploy, House Uni v4 e2e / remittance enable, SignPath, promote TSL to match local 1.1.10 after signed pack, live relay.

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
1. SignPath / signed pack, then promote TSL channel to match Programs 1.1.10 (after signed pack); renew www.thesecretlab.app TLS (bare host still 200)
2. Hostile-page security retest before public ship
3. Live address-book fills (savings / bond_market / remittance) after reviewed deploys
4. Trailer sync to TSL when ready
