# Ketflix / vapurr media hosting

**Rule (Relic 2026-09-04):** never pack `.mp4` / `.webm` into the Windows install or zip.
`RustEmbed` excludes them (`crates/vapurr-shell/src/host/assets.rs`).

## Public base
`https://thesecretlab.app/vapurr/ketflix/trailers/{slug}.mp4`

Wired in `frontend/ketflix.html` as `TRAILER_BASE`.

Also reserved:
- `https://thesecretlab.app/vapurr/commercial/` — SuperApp spot masters
- Local working copies stay on disk under `frontend/ketflix/trailers/` for upload only

## Upload checklist
1. Cook trailers into `frontend/ketflix/trailers/*.mp4` (local).
2. Sync that folder to TSL static path `/vapurr/ketflix/trailers/` (same host as Maestro audio).
3. House pack without media — exe should stay lean; confirm zip shrinks vs 10:28 bloat pack.
4. Play on Ketflix should hit TSL; toast "Trailer soon" only if CDN 404.

## Why the 10:28 zip was fat
Earlier packs effectively carried trailer weight (STATUS noted rust-embedded trailers). Exclude is now explicit; next House pack re-logs sha/size.
