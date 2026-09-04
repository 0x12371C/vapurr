# vapurr worker coordination (hub: vapurrbot)

Relic talks to **vapurrbot** (Grok Bot). CLI `grok.exe` sessions cannot join that chat.

## Org board (2026-09-04 ~10:30 ET)

| Codename | Status | Owns |
|----------|--------|------|
| **vapurrbot** | HUB | board, TRACKS, BUILD_VERSIONS, ship bar, econ shape |
| **House** | KEEP | install/zip/`pack.ps1`, plan.md P0, doc canon — **sole packer** |
| **Pilot** | KEEP | $PUSD desk, copy, Scan liq, pitch — **no pack** |
| **KFX** | ACTIVE (approved) | Ketflix UI / posters / trailers |
| **PayId** | ACTIVE (approved) | KetPay + zer0ID / earn KYC |
| **Bind** | ACTIVE (approved) | `install_id` mint + SYBIL |
| Charts / Psy / Tube | **PARK** | freeze unless Relic reopens |

## Hard rules (Relic)
1. **No new grok/powershell process spawns.** Only the three approved organizers (KFX/PayId/Bind) + existing House/Pilot windows. Idle = note in TRACKS/OVERNIGHT or inbox text — never Start-Process.
2. House alone runs `pack.ps1`. No pack wars.
3. Canon: **404 = load-fail page.** **KetPay = HTTP 402 / x402 / $PUSD.** Never fuse them.
4. Earn payout needs zer0ID KYC (`thesecretlab.app/kyc`) + `install_id`. Sybil = per machine install bound.
5. After-v1 (Servo, Rain settle) only if Relic names them. Exception: live zer0ID is earn-critical.
6. Version **1.1.0** (rev 223a962). Economy: `docs/econ/TESTNET_SHAPE.md`.

## Economy (locked)
Genesis `$VAPURR`: **50% treasury** / **50% LP**. Of the LP half, burn **half** → `$PUSD`, keep **half** `$VAPURR`, seed Uni v4 CL on **46630**. `TESTNET_HOUSE` empty until Relic signs. Details: `docs/econ/TESTNET_SHAPE.md`.

## Open tracks
See `docs/TRACKS.md`. Flash file: `docs/ORG_FLASH.md`.

If it is not on ORG_FLASH, it is drift. Do not add a sixth builder, a new `vapurr://` id, or another isolated `target-*`.