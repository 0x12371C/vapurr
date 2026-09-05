# Tracks

Living owner board. Snapshot: `docs/SNAPSHOT.md`. Flash: `docs/ORG_FLASH.md`.

| Track | Owner | Status | Next |
|-------|-------|--------|------|
| Org board | vapurrbot | live â€” Bot is primary (CLI credits dry) | KEEP House+Pilot; ACTIVE KFX/PayId/Bind only; **no new grok procs** |
| Code signing | Relic + House | **P0 SHIP BLOCKER** — public `vapurr-setup.exe` Defender `Wacatac.C!ml` (unsigned). Independent verify clean. | Buy OV or Microsoft Artifact Signing; `pack.ps1` must sign; MSFT FP submit; `docs/SIGNING.md` |
| Build / pack | House | **1.1.9** on TSL (sha 50075F06); Programs still **1.1.7**; Defender quarantines local dist copies | Signed 1.1.10 pack → TSL; then Programs hot-patch/reinstall |
| Media hosting | vapurrbot + TSL | Ketflix `TRAILER_BASE` â†’ thesecretlab.app; CDN 404 until upload | sync `frontend/ketflix/trailers/*.mp4`; see HOSTING.md |
| Ketflix UI | KFX | posters+hero 1080p landscape; **idle** since ~13:07 | review only; no new proc |
| Ketflix trailers | KFX | **12/12** local; last ~13:07 | host on TSL; do not pack |
| SuperApp commercial | vapurrbot | AAA pass ~25.6s brand-locked mascot | Relic feedback; pad to 30s |
| Mascot slop lore | vapurrbot | director rewritten (T2V inventer); not sustained-running | start `run_vapurr_slop_machine.py` when wanted |
| KetPay / zer0ID | PayId | NeedKyc + KYC URL; **idle** since ~10:26 | freeze unless Relic names hole |
| install_id | Bind+House | code-verified; Bind PID **gone**; **idle** | no respawn; don't reopen setup.rs |
| Token economy | vapurrbot+Pilot | gen-4 / HouseLp claimed live 46630; Pilot touched pay/wallet/pusd PM | keep STATUS CAs honest; vault live |
| Charts / Psy / Tube | PARK | | |

## Hard media rule (Relic)
Never pack `.mp4` / `.webm`. Host at `https://thesecretlab.app/vapurr/â€¦`. Embed excludes in `crates/vapurr-shell/src/host/assets.rs`.
