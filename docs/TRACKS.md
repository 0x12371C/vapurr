# Tracks

Overnight log: `docs/TRACKS/OVERNIGHT.md` (canonical). Root `docs/OVERNIGHT.md` is a stub pointer only.

Living owner board. Snapshot: `docs/SNAPSHOT.md`. Flash: `docs/ORG_FLASH.md`.

| Track | Owner | Status | Next |
|-------|-------|--------|------|
| Org board | vapurrbot | live — Bot is primary (CLI credits dry) | KEEP House+Pilot; ACTIVE KFX/PayId/Bind only; **no new grok procs** |
| Code signing | Relic + House | **P0 SHIP BLOCKER** - unsigned Defender hit (`NotSigned` on Programs exe). Wallet IPC hardening in-tree; Relic hostile-page retest open — `docs/RELEASE_REVIEW_2026-09-04.md`. | OV/Artifact Signing; `pack.ps1` must sign; MSFT FP; Relic security retest before public ship |
| Build / pack | House | **Programs 1.1.17** (sha 93A4AAE04C7C, rev e590ef3, **NotSigned**); AppData channel **1.1.18** (sha 97156FC2CA73, rev e590ef3, **NotSigned**) — Programs lags channel; AppData root VERSION.txt lag **1.1.11**; **TSL channel 200** at **1.1.13** (rev e590ef3, sha 324286bf6f68...); dist/Cargo still **1.1.10**; local repo channel/manifest.json missing. Overnight 23:13: verify-spusd-cd.py PASS (Open CD live CTA + NeedSavings + SPUSD.md). Hold stranger-ship until signed + Relic security bar. | Signed pack to TSL after Relic retest; promote TSL after signed pack; PatchApply Programs to match channel 1.1.18; SignPath remains ship P0 |erify-wgv-house.py PASS (wrap-first House UI + WGV_HOUSE/HOUSE_PAIR). Hold stranger-ship until signed + Relic security bar. | Signed pack to TSL after Relic retest; promote TSL after signed pack + renew TSL TLS; PatchApply Programs to match channel 1.1.18; SignPath remains ship P0 |
| Media hosting | vapurrbot + TSL | Ketflix `TRAILER_BASE` → thesecretlab.app; CDN 404 until upload | sync `frontend/ketflix/trailers/*.mp4`; see HOSTING.md |
| Ketflix UI | KFX | posters+hero 1080p landscape; **idle** since ~13:07 | review only; no new proc |
| Ketflix trailers | KFX | **12/12** local; last ~13:07 | host on TSL; do not pack |
| SuperApp commercial | vapurrbot | AAA pass ~25.6s brand-locked mascot | Relic feedback; pad to 30s |
| Mascot slop lore | vapurrbot | director rewritten (T2V inventer); not sustained-running | start `run_vapurr_slop_machine.py` when wanted |
| KetPay / zer0ID | PayId | NeedKyc + KYC URL; **idle** since ~10:26 | freeze unless Relic names hole |
| install_id | Bind+House | code-verified; Bind PID **gone**; **idle** | no respawn; don't reopen setup.rs |
| Token economy | vapurrbot+Pilot | gen-4 / HouseLp claimed live 46630; Pilot touched pay/wallet/pusd PM | keep STATUS CAs honest; vault live |
| Token economy / Fed gV | Relic+House | **slice green** — gV walls + fee/skim/CD/wgV stubs + SavingsRouter/BondMarket live-by-default + gen-5 Lithe cutover source; **USDG = BondAssetTag only** (no `$PUSD`/USDG pool / peg-depth) | Live wire / Uni IHooks / House AMM wgV deploy when Relic opens; savings CAs empty (NeedSavings); remittance CAs empty (NeedRemittance); bond_market empty (NeedBondMarket); SignPath remains ship P0; bonds/CD book prove `verify-bonds-book.py` PASS |
| Charts / Psy / Tube | PARK | | |

## Hard media rule (Relic)

Never pack `.mp4` / `.webm`. Host at `https://thesecretlab.app/vapurr/…`. Embed excludes in `crates/vapurr-shell/src/host/assets.rs`.
