# vapurr chrome surfaces

Chrome never uses a separate "site process" in the product model. Today every chrome URL is HTML inside the page WebView at `http://vapurr.localhost/…`.

Resolver: `resolve_nav` / `pane_url` in `crates/vapurr-shell/src/main.rs`.
Aliases: `chrome_url` in `crates/vapurr-shell/src/desk.rs`.

If you add a `vapurr://` id, add a row here in the same change.

## Window chrome (not `vapurr://` pages)

| WebView | File | Role |
|---|---|---|
| sidebar | `frontend/sidebar.html` | 64px rail |
| toolbar | `frontend/toolbar.html` | 84px: 36px tab strip + 48px omnibox |
| radio | `frontend/radio.html` | Maestro Play strip / float |
| page | (routed) | everything else |

## `vapurr://` → file

| id | aliases | file | notes |
|---|---|---|---|
| `home` | (empty) | `home.html?v=wordmark` | thinking-orb home |
| `wallet` | `portfolio` | `wallet.html` | Apple-glass portfolio for this device on Robinhood Chain. Live Trenches is a separate tab. |
| `pay` | `404` | `pay.html` | 404 payments sheet |
| `card` | | `card.html` | parked — not on the rail or Wallet for now |
| `zzzmail` | `zmail`, `mail` | `zzzmail.html` | glass inbox. `.hood` names (ENS-shaped). Seal → CID pin → 0.25¢ gasless postage. `zmail.html` is a redirect stub |
| `id` | | `id.html` | parked zer0ID sheet. No issuer, no KYC form. Not Shield |
| `defi` | `finance` | `defi.html` | House DeFi hub — Swap, Bridge, PUSD, vapurrbid, PNS, Liquidity. Rail button. |
| `swap` | | `swap.html` | LI.FI quote chrome. CTA copies the route and opens Wallet — does not settle |
| `stake` | `pusd`, `vapurr`, `mint`, `lithe` | `pusd.html` | $VAPURR / $PUSD desk. **Lithe** is 9% on $PUSD |
| `vapurrbid` | `outbid`, `bid`, `board` | `vapurrbid.html` | $PUSD pay-to-rank. `outbid.html` redirects here |
| `pns` | `hood`, `names` | `pns.html` | Purr Name Service. TLD `.hood`. On-chain registry |
| `bridge` | | `bridge.html` | LI.FI quote chrome, same honesty as swap |
| `dapps` | | `dapps.html` | |
| `scan` | `explorer`, `xray`, `blocks`, `gas`, `gwei` | `explorer.html` | query string kept; `gas`/`gwei` open `?tab=gas` |
| `floor` | `list`, `projects` | `floor.html` | |
| `fomo` | `family` | https://fomo.family | Live Trenches — opens fomo.family in this window |
| `ketflix` | | `ketflix.html` | floor / dApps tile |
| `ketcharts` | `charts`, `chart` | `ketcharts.html` | Ket charts. Vela™ engine (Apache-2.0). Live Binance OHLCV |
| `ketbook` | `docs`, `honkit`, `book` | `ketbook.html` | Public product docs (Ketbook). Source `ketbook/`. Not the internal `docs/` folder |
| `earn` | | `earn.html` | |
| `history` | | `history.html` | |
| `bookmarks` | | `bookmarks.html` | |
| `cookies` | `cookie`, `jar` | `cookies.html` | |
| `settings` | | `settings.html` | |
| `shield` | `adblock` | `shield.html` | Shield UI (adblock-rust). Rail button. Distinct from `id` |
| `boost` | `memory`, `blobs` | `memory.html` | local blob quota |
| *(other)* | | `pane.html?id=…` | fallback copy-only sheet |

Scan queries typed in the omnibox (`vapurr-rhc::scan::is_scan_query`) also open `explorer.html`.

## Rust endpoints on the chrome host

Served by `crates/vapurr-shell/src/host.rs` (not static files):

| path | crate |
|---|---|
| `/fomo/api/desk` | `vapurr-fomo` (crate still serves JSON; v1 chrome opens https://fomo.family) |
| `/scan/api/*` | `vapurr-rhc::scan` |
| `/scan/api/liq` | `vapurr-rhc::liq` — live RHC RPC market map. Stats are full; graph/lists are a capped view. |
| `/liq/api` | same snapshot, for swap/fomo to pull |
| `/route/api/quote` | `vapurr-rhc::route` — LI.FI quote + 25 bps scoop |
| `/route/api/tokens` | `vapurr-rhc::route` |
| `/zzzmail/api/quote` | `vapurr-zmail` — 0.25¢ $PUSD/$VAPURR, gasless |
| `/zzzmail/api/me` | mailcard |
| `/zzzmail/api/inbox` | opened letters + pinset |
| `/zzzmail/api/send` | POST `{to, body, asset}` — seal, pin, voucher. Optional `subject` |
| `/zzzmail/api/hood` | PNS snapshot (primary + owned). Alias: `/zzzmail/api/pns`, `/hood/me` |
| `/zzzmail/api/hood/register` | POST `{name}` — claim `name.hood` on testnet 46630. Alias: `/pns/register` |
| `/zzzmail/api/pns/deploy` | POST — deploy `PnsRegistry` if missing |
| `/zzzmail/api/pns/set-addr` | POST `{name, addr}` — ENS `setAddr` |
| `/zzzmail/api/pns/set-name` | POST `{name}` — set reverse / primary |
| `/zzzmail/api/hood/resolve/{name}` | PNS resolve (addr + x25519). Alias: `/pns/resolve/{name}` |
| `/zzzmail/api/hood/reverse/{addr}` | PNS reverse `0x` → `alice.hood`. Alias: `/pns/reverse/{addr}` |
| `/zzzmail/api/letter/{cid}` | open a pinned letter by CID |
| `/patch/api/status` | running build vs channel (`vapurr.next.exe`, `%LOCALAPPDATA%\vapurr\channel`, `VAPURR_CHANNEL`, repo `dist`). Apply is IPC `patch-apply`, not this GET. |

CLI: `vapurr.exe --publish` stamps this exe into the channel. `--patch-apply` swaps and relaunches. `--patch-swap` is the helper. `pack.ps1` publishes the same channel.

## Other frontend files

Not every file is a `vapurr://` id. Named so they do not rot:

- `pane.html` — unknown-id fallback
- `fomo.html` — redirect stub to https://fomo.family (`pane_url("fomo")` opens the live site, not this file)
- `zmail.html` — redirect stub to `zzzmail.html`
- `outbid.html` — redirect stub to `vapurrbid.html`
- `explorer.js`, `floor.js`, `ipc.js`, `shader.js`, `route.js`, `globe.js`, `radio.js`, `zzzmail.js` — scripts
- `chrome.css`, `tokens.css`, `radio.css`
- `cat.svg`, `ketflix-logo.svg`, `ketflix.png`, `ketcharts-logo.svg`, `logo.png`, `maestro-logo.png`, `mascot.png`, `404.png`, `zzzmail-icon.png`
- `vendor/vela.global.min.js` — Vela™ `@luxalgo/vela` 0.6.17 (Apache-2.0). NOTICE/LICENSE beside it. Attribution on the Ketcharts screen.
- `robinhood-chain-logo-black.svg`, `robinhood-chain-logo-white.svg`
- `cursors/arrow.svg`, `cursors/text.svg`
- `fonts/Sora-*.ttf`
- `vendor/three.webgpu.min.js`, `vendor/three.tsl.min.js`, `vendor/three.core.min.js` — PUSD globe (`pusd.html` / `globe.js`)
- `vendor/BloomNode.js`, `vendor/RoomEnvironment.js` — Three addons on disk; `globe.js` does not import them yet
- Scan Liquidity is SVG in `explorer.js`
- `frontend/ketbook/` — generated HonKit output. `npm run docs:app`. Served at `/ketbook/`. Not a `pane_url` file; `ketbook.html` frames it.
