# Tracks



Living owner board. Snapshot: `docs/SNAPSHOT.md`. Flash: `docs/ORG_FLASH.md`.



| Track | Owner | Status | Next |

|-------|-------|--------|------|

| Org board | vapurrbot | live ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â Bot is primary (CLI credits dry) | KEEP House+Pilot; ACTIVE KFX/PayId/Bind only; **no new grok procs** |

| Code signing | Relic + House | **P0 SHIP BLOCKER** - unsigned Defender hit. Wallet IPC hardening **in-tree** (Codex); Relic hostile-page retest open Ã¢â‚¬â€ `docs/RELEASE_REVIEW_2026-09-04.md`. | OV/Artifact Signing; `pack.ps1` must sign; MSFT FP; Relic security retest before public ship |

| Build / pack | House | **1.1.9** Programs rev `a825573` @ 11:02 ET (sha fa404567…); local TSL channel manifest **missing** this hour; Defender still hostile to unsigned dist. Hold pack until signed + Relic security bar. | Signed 1.1.10 pack to TSL after Relic retest; then Programs hot-patch/reinstall |

| Media hosting | vapurrbot + TSL | Ketflix `TRAILER_BASE` ÃƒÂ¢Ã¢â‚¬Â Ã¢â‚¬â„¢ thesecretlab.app; CDN 404 until upload | sync `frontend/ketflix/trailers/*.mp4`; see HOSTING.md |

| Ketflix UI | KFX | posters+hero 1080p landscape; **idle** since ~13:07 | review only; no new proc |

| Ketflix trailers | KFX | **12/12** local; last ~13:07 | host on TSL; do not pack |

| SuperApp commercial | vapurrbot | AAA pass ~25.6s brand-locked mascot | Relic feedback; pad to 30s |

| Mascot slop lore | vapurrbot | director rewritten (T2V inventer); not sustained-running | start `run_vapurr_slop_machine.py` when wanted |

| KetPay / zer0ID | PayId | NeedKyc + KYC URL; **idle** since ~10:26 | freeze unless Relic names hole |

| install_id | Bind+House | code-verified; Bind PID **gone**; **idle** | no respawn; don't reopen setup.rs |

| Token economy | vapurrbot+Pilot | gen-4 / HouseLp claimed live 46630; Pilot touched pay/wallet/pusd PM | keep STATUS CAs honest; vault live |

| Token economy / Fed gV | Relic+House | **slice green** — gV walls + fee/skim/CD/wgV stubs + **SavingsRouter** (`60f3744`, forge 20/20) | Live wire / Uni IHooks / House AMM wgV deploy when Relic opens; SignPath remains ship P0 |

| Charts / Psy / Tube | PARK | | |



## Hard media rule (Relic)

Never pack `.mp4` / `.webm`. Host at `https://thesecretlab.app/vapurr/ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦`. Embed excludes in `crates/vapurr-shell/src/host/assets.rs`.

