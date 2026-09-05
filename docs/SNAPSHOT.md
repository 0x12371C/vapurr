# vapurr snapshot — 2026-09-04 late afternoon (vapurrbot)

Living board for agents. Prefer this + `TRACKS.md` over old overnight kick lore.

## Ship
| Item | Truth |
|------|-------|
| Version | **1.1.4** (Cargo / channel rev `ce99dac`) |
| Last full zip | `dist/vapurr-1.1.3-windows-x64.zip` **~43.3 MB @ 16:12 ET** sha12 `CE20D957068F` |
| Channel | `%LOCALAPPDATA%\vapurr\channel\vapurr.exe` — **1.1.3** (sha `298edc7cf9ff…`) |
| Install | `dist/vapurr/Install vapurr.exe` — open **channel** `vapurr.exe` for day-to-day (Install path is setup) |
| Pack rule | **never embed mp4/webm**; House/`pack.ps1` sole packer |

## Hot fixes
- **Oliver** (not Euler) on wallet desk + `$PUSD` vault card.
## Hot fixes in 1.1.2 → 1.1.3
- **1.1.2:** Swap hard-crash — `score_of() -> i128` into `serde_json::json!` panicked `number out of range`. Scores stringified.
- **1.1.3:** House `$VAPURR`/`$PUSD` kickback **0.03%** (`HOUSE_REFUND_BPS=3`) with real `refund.display` amount in Swap UI. LiFi/non-house stays **0.05%** (`ROUTE_REFUND_BPS=5`).


## Security / release hold (2026-09-04 evening)
- Codex wallet IPC hardening is **in-tree** (security.rs, DPAPI keystore, receipt-gated pay, API capability token).
- Finish pass: `cargo +stable-x86_64-pc-windows-gnu test -p vapurr-shell -p vapurr-wallet` green (98 pass / 1 ignored) on isolated TEMP profile.
- **Hold public ship** until signed pack + Relic hostile-page retest. Details: `docs/RELEASE_REVIEW_2026-09-04.md`.
## Org
| Role | Status |
|------|--------|
| vapurrbot | HUB (this chat) — Relic out of grok CLI credits; Bot is primary |
| House | KEEP — sole `pack.ps1` |
| Pilot | KEEP — $PUSD desk / chrome; no pack |
| KFX / PayId / Bind | ACTIVE approved organizers only |
| Charts / Psy / Tube | PARK |
| Hard rule | **No new grok/powershell spawns.** Idle = TRACKS/OVERNIGHT/inbox text |

## Canon locks
- **404 = load-fail chrome only.** **KetPay = HTTP 402 / x402 / $PUSD.** Never fuse.
- Earn claim needs **zer0ID KYC** (`thesecretlab.app/kyc`) + **`install_id`** (per-machine sybil).
- Product dollar **$PUSD**. Home chain **4663**; econ bootstrap / settle **46630** testnet.
- House book fee **0.30%** + **0.03%** `$VAPURR` user rebate (display + quote plan).
- Mascot: flat lime geometric cat `#c0f800` from `frontend/mascot.png`.

## Media
- Ketflix trailers **12/12** local under `frontend/ketflix/trailers/` — **do not pack**.
- Play URL: `https://thesecretlab.app/vapurr/ketflix/trailers/{slug}.mp4`. CDN **not uploaded yet**.
- Docs: `docs/ketflix/HOSTING.md`. Embed excludes in `assets.rs`.
- SuperApp commercial: `frontend/commercial/vapurr-superapp-30s.mp4` (~25.6s).

## Product gates (v1 / v1.2)
See `STATUS.md`. Headline: native Windows WebView2 ships; gen-4 market + HouseLp live on **46630**; vault live; KetPay testnet settle.

## Graphify
`graphify-out/vapurr.html`. Refresh: `python -m graphify update .` then `python scripts/brand_graph.py`.

## Open next (Relic)
1. Sync 12 trailers to TSL `/vapurr/ketflix/trailers/`
2. Git remote + push (repo currently has **no remote**)
3. Commercial pad to clean 30s / beat feedback
4. Start mascot slop loop on 3090 if wanted
