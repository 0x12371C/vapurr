# Tracks

Living owner board. Snapshot: `docs/SNAPSHOT.md`. Flash: `docs/ORG_FLASH.md`.

| Track | Owner | Status | Next |
|-------|-------|--------|------|
| Org board | vapurrbot | live — Bot is primary (CLI credits dry) | KEEP House+Pilot; ACTIVE KFX/PayId/Bind only; **no new grok procs** |
| Build / pack | House | **1.1.3** zip+Install @16:12 (HOUSE_REFUND_BPS=3 / swap rebate) | quit+reinstall to leave old binary; **no mp4/webm in embed** |
| Media hosting | vapurrbot + TSL | Ketflix `TRAILER_BASE` → thesecretlab.app; CDN 404 until upload | sync `frontend/ketflix/trailers/*.mp4`; see HOSTING.md |
| Ketflix UI | KFX | posters+hero 1080p landscape; **idle** since ~13:07 | review only; no new proc |
| Ketflix trailers | KFX | **12/12** local; last ~13:07 | host on TSL; do not pack |
| SuperApp commercial | vapurrbot | AAA pass ~25.6s brand-locked mascot | Relic feedback; pad to 30s |
| Mascot slop lore | vapurrbot | director rewritten (T2V inventer); not sustained-running | start `run_vapurr_slop_machine.py` when wanted |
| KetPay / zer0ID | PayId | NeedKyc + KYC URL; **idle** since ~10:26 | freeze unless Relic names hole |
| install_id | Bind+House | code-verified; Bind PID **gone**; **idle** | no respawn; don't reopen setup.rs |
| Token economy | vapurrbot+Pilot | gen-4 / HouseLp claimed live 46630; Pilot touched pay/wallet/pusd PM | keep STATUS CAs honest; vault live |
| Charts / Psy / Tube | PARK | | |

## Hard media rule (Relic)
Never pack `.mp4` / `.webm`. Host at `https://thesecretlab.app/vapurr/…`. Embed excludes in `crates/vapurr-shell/src/host/assets.rs`.

