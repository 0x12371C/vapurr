# vapurr worker coordination (hub: vapurrbot)

Relic talks to **vapurrbot** (Grok Bot). CLI `grok.exe` credits may be dry — Bot is the live hub.

## Org board (2026-09-04 ~15:00 ET)

| Codename | Status | Owns |
|----------|--------|------|
| **vapurrbot** | HUB | board, TRACKS, SNAPSHOT, BUILD_VERSIONS, ship bar, media hosting, commercial/slop |
| **House** | KEEP | install/zip/`pack.ps1`, plan.md P0, doc canon — **sole packer** |
| **Pilot** | KEEP | $PUSD desk, copy, Scan liq, pitch — **no pack** |
| **KFX** | ACTIVE (approved) | Ketflix UI / posters / trailers (local cook only) |
| **PayId** | ACTIVE (approved) | KetPay + zer0ID / earn KYC |
| **Bind** | ACTIVE (approved) | `install_id` mint + SYBIL |
| Charts / Psy / Tube | **PARK** | freeze unless Relic reopens |

## Hard rules (Relic)
1. **No new grok/powershell process spawns.** Only KFX/PayId/Bind + existing House/Pilot. Idle = TRACKS/OVERNIGHT/inbox — never Start-Process.
2. House alone runs `pack.ps1`. No pack wars.
3. Canon: **404 = load-fail.** **KetPay = HTTP 402 / x402 / $PUSD.** Never fuse.
4. Earn payout needs zer0ID KYC + `install_id`.
5. **Never embed mp4/webm.** Host on thesecretlab.app (`docs/ketflix/HOSTING.md`).
6. Version **1.1.0**. Economy: `docs/econ/TESTNET_SHAPE.md`. Living truth: `docs/SNAPSHOT.md` + `STATUS.md`.

## Open tracks
See `docs/TRACKS.md`. Flash: `docs/ORG_FLASH.md`. Graph: `docs/GRAPHIFY.md`.

If it is not on ORG_FLASH / TRACKS, it is drift.
