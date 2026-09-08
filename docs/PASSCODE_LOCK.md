# Passcode lock UI

rontend/lock.html — Apple-style 4-digit fullscreen chrome (void/lime).
**Never stores plaintext PIN** (no localStorage / sessionStorage). Shell hashes + gates.

## Hooks (shell assigns, UI calls)

| Hook | Role |
|------|------|
| VapurrLock.onSubmit(pin) | Unlock: fired with 4-digit pin, then UI wipes digits |
| VapurrLock.onUnlock() | Fired from VapurrLock.unlock() after shell verifies |
| VapurrLock.lock(opts?) | Show/re-arm lock (startup + idle). opts.mode: unlock\|set\|confirm |
| VapurrLock.onSetPin(a, b) | First-run: set then confirm; both wiped after call |
| VapurrLock.fail(msg) | Wrong pin / mismatch — shake + clear |
| VapurrLock.unlock() | Shell success path |
| VapurrLock.setMode(m) | Copy only |

Aliases: window.lock, __passcodeFail, __passcodeOk, __passcodePaint.

Idle auto-lock default **15 min** is shell-owned (storage + timer + IPC gate).

Fallback while wiring: apurr.send({ cmd: "passcode-submit", code, mode }) / passcode-set.
