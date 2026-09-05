# Passcode lock UI

`frontend/lock.html` — Apple-style 4-digit fullscreen chrome (void/lime).

## Shell contract

| Direction | Hook |
|-----------|------|
| UI → shell | `vapurr.send({ cmd: "passcode-submit", code: "####", mode })` when 4 digits entered |
| Shell → UI | `window.__passcodeFail(msg)` shake + clear |
| Shell → UI | `window.__passcodeOk()` clear (then navigate away) |
| Shell → UI | `window.__passcodePaint({ mode, title, hint, kicker })` |
| Modes | `unlock` (default), `set`, `confirm` — also `?mode=` |

Idle auto-lock default **15 min** is shell-owned (storage + timer + gate). UI does not start the timer.

No Luna/UST/Olympus names.
