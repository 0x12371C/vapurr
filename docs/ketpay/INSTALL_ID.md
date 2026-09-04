# install_id

Per-machine UUID for earn sybil. Minted once on successful Windows Install. Idempotent.

## File

`%LOCALAPPDATA%\vapurr\install_id`

Profile dir (next to `desk.json` / `vapurr.log`). Not the exe install dir (`%LOCALAPPDATA%\Programs\vapurr`).

Plain text, lowercase UUID, one line. Not a wallet key. Not in git.

## When

`do_install` in `crates/vapurr-shell/src/setup.rs` after copy + shortcuts + uninstall key.

Trigger: `setup/api/install` (and IPC `setup-install`). Mint failure is logged; Install still succeeds.

Reinstall keeps the same UUID if the file is already a valid UUID. Uninstall does **not** delete it (reinstall-farming would mint a new machine).

Portable (`setup/api/portable`) does not mint.

## Earn

`Desk::snapshot` and `desk_json` include `install_id` when the file exists. `frontend/earn.html` shows the first 8 chars. Enrollment is `(install_id, device_key)` — see `docs/ketpay/SYBIL.md`.

## Files

- `crates/vapurr-shell/src/setup.rs` — mint / read
- `crates/vapurr-shell/src/desk.rs` — earn snapshot field
- `crates/vapurr-shell/src/main.rs` — desk payload
- `frontend/earn.html` — display (already)

## Checklist (House verify) — code 2026-09-04

- [x] Mint path: `do_install` calls `ensure_install_id()` (profile dir, not Programs)
- [x] Desk/earn: `desk.rs` snapshot + `earn.html` first 8 chars
- [x] SYBIL: enrollment is `(install_id, device_key)` in `docs/ketpay/SYBIL.md`
- [x] Uninstall: `uninstall_silent` rmdirs Programs only — does **not** delete the profile file
- [x] Portable: `Msg::Portable` does not call `ensure_install_id`
- [x] Idempotent: `install_id_idempotent` unit test

Live click-Install was not re-run this pass. Do not reopen `setup.rs`.
