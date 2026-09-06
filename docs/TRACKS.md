# Tracks

Overnight log: `docs/TRACKS/OVERNIGHT.md`.



Living owner board. Snapshot: `docs/SNAPSHOT.md`. Flash: `docs/ORG_FLASH.md`.



| Track | Owner | Status | Next |

|-------|-------|--------|------|

| Org board | vapurrbot | live ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Bot is primary (CLI credits dry) | KEEP House+Pilot; ACTIVE KFX/PayId/Bind only; **no new grok procs** |

| Code signing | Relic + House | **P0 SHIP BLOCKER** - unsigned Defender hit (`NotSigned` on Programs exe). Wallet IPC hardening in-tree; Relic hostile-page retest open â€” `docs/RELEASE_REVIEW_2026-09-04.md`. | OV/Artifact Signing; `pack.ps1` must sign; MSFT FP; Relic security retest before public ship |

| Build / pack | House | **Programs 1.1.11** (sha `26EC8DE3AB1F`, **NotSigned**); AppData channel **1.1.12** rev `8770b23` (sha `cde147a41437`); Programs!=channel; TSL channel manifest **404**. Route-safety `0c9d575` on `pack/d59912d`. Hold public until signed + Relic security bar. | Signed pack to TSL after Relic retest; restore public channel manifest |

| Media hosting | vapurrbot + TSL | Ketflix `TRAILER_BASE` ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ thesecretlab.app; CDN 404 until upload | sync `frontend/ketflix/trailers/*.mp4`; see HOSTING.md |

| Ketflix UI | KFX | posters+hero 1080p landscape; **idle** since ~13:07 | review only; no new proc |

| Ketflix trailers | KFX | **12/12** local; last ~13:07 | host on TSL; do not pack |

| SuperApp commercial | vapurrbot | AAA pass ~25.6s brand-locked mascot | Relic feedback; pad to 30s |

| Mascot slop lore | vapurrbot | director rewritten (T2V inventer); not sustained-running | start `run_vapurr_slop_machine.py` when wanted |

| KetPay / zer0ID | PayId | NeedKyc + KYC URL; **idle** since ~10:26 | freeze unless Relic names hole |

| install_id | Bind+House | code-verified; Bind PID **gone**; **idle** | no respawn; don't reopen setup.rs |

| Token economy | vapurrbot+Pilot | gen-4 / HouseLp claimed live 46630; Pilot touched pay/wallet/pusd PM | keep STATUS CAs honest; vault live |

| Token economy / Fed gV | Relic+House | **slice green** â€” gV walls + fee/skim/CD/wgV stubs + SavingsRouter/BondMarket live-by-default + gen-5 Lithe cutover source; **USDG = BondAssetTag only** (no `$PUSD`/USDG pool / peg-depth) | Live wire / Uni IHooks / House AMM wgV deploy when Relic opens; savings CAs empty (NeedSavings); remittance CAs empty (NeedRemittance); bond_market empty (NeedBondMarket); SignPath remains ship P0 |

| Charts / Psy / Tube | PARK | | |



## Hard media rule (Relic)

Never pack `.mp4` / `.webm`. Host at `https://thesecretlab.app/vapurr/ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â¦`. Embed excludes in `crates/vapurr-shell/src/host/assets.rs`.

