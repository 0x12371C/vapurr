# vapurr chrome surfaces

Chrome never uses a separate "site process" in the product model. Today every chrome URL is HTML inside the page WebView at `http://vapurr.localhost/â€¦`.

Resolver: `resolve_nav` / `pane_url` in `crates/vapurr-shell/src/nav.rs`.
Aliases: `chrome_url` in `crates/vapurr-shell/src/desk.rs`.

If you add a `vapurr://` id, add a row here in the same change.

## Window chrome (not `vapurr://` pages)

| WebView | File | Role |
|---|---|---|
| sidebar | `frontend/sidebar.html` | 64px rail |
| toolbar | `frontend/toolbar.html` | 84px: 36px tab strip + 48px omnibox |
| radio | `frontend/radio.html` | Maestro Play strip / float |
| page | (routed) | everything else |

## `vapurr://` â†’ file

| id | aliases | file | notes |
|---|---|---|---|
| `home` | (empty) | `home.html?v=wordmark` | thinking-orb home |
| `wallet` | `portfolio` | `wallet.html` | Apple-glass portfolio for this device on Robinhood Chain. Live Trenches is a separate tab. |
| `pay` | `ketpay` | `pay.html` | **KetPay** — x402 / `$PUSD` on testnet 46630. Signs `wallet-send`. NOT 404. |
| `card` | | `card.html` | parked â€” not on the rail or Wallet for now |
| `zzzmail` | `zmail`, `mail` | `zzzmail.html` | glass inbox. `.hood` names (ENS-shaped). Seal â†’ CID pin â†’ 0.25Â¢ gasless postage. `zmail.html` is a redirect stub |
| `id` | | `id.html` | zer0ID. Start KYC opens https://www.thesecretlab.app/kyc. Does not fake Proven. Not Shield |
| `defi` | `finance` | `defi.html` | House DeFi hub â€” Swap, Bridge, PUSD, vapurrbid, PNS, Liquidity. Rail button. |
| `swap` | | `swap.html` | Best simulated net. Sign on this device broadcasts. MAX, balances, impact, `$VAPURR` refund |
| `stake` | `pusd`, `vapurr`, `mint`, `lithe`, `euler`, `loop`, `house`, `lp` | `pusd.html` | $VAPURR / $PUSD desk. **Lithe** is 9% on $PUSD. Euler vault `?tab=euler`. House Uni v4 CL `?tab=house`. Empty CAs until deploy. |
| `vapurrbid` | `outbid`, `bid`, `board` | `vapurrbid.html` | $PUSD pay-to-rank. `outbid.html` redirects here |
| `pns` | `hood`, `names` | `pns.html` | Purr Name Service. TLD `.hood`. On-chain registry |
| `bridge` | | `bridge.html` | Same as swap across chains. Sign on this device |
| `dapps` | | `dapps.html` | |
| `scan` | `explorer`, `xray`, `blocks`, `gas`, `gwei` | `explorer.html` | query string kept; `gas`/`gwei` open `?tab=gas` |
| `floor` | `list`, `projects` | `floor.html` | |
| `fomo` | `family` | https://fomo.family | Live Trenches â€” opens fomo.family in this window |
| `ketflix` | | `ketflix.html` | Netflix-style browse. Posters/catalog/trailers at `/ketflix/` |
| `ketcharts` | `charts`, `chart` | `ketcharts.html` | DexScreener-shaped RHC pair tape. Our `/liq/api/tape` + `/liq/api/trades/{pool}`. Vela™ candles. Gecko/Paprika fill history only. **Listed** is `$PUSD` pay-to-list (`KetList.sol`) — profile (web/X/tg/bio/logo) rides with the paid tx and paints from the snap |
| `ketbook` | `docs`, `honkit`, `book` | `ketbook.html` | Public product docs (Ketbook). Source `ketbook/`. Not the internal `docs/` folder |
| `earn` | | `earn.html` | browsing receipts — host + HTTPS + time, sealed window with hash |
| `history` | | `history.html` | |
| `bookmarks` | | `bookmarks.html` | |
| `cookies` | `cookie`, `jar` | `cookies.html` | |
| `settings` | | `settings.html` | |
| `shield` | `adblock` | `shield.html` | Shield UI (adblock-rust). Rail button. Distinct from `id` |
| `boost` | `memory`, `blobs` | `memory.html` | local blob quota |
| *(other)* | | `pane.html?id=â€¦` | fallback copy-only sheet |

Scan queries typed in the omnibox (`vapurr-rhc::scan::is_scan_query`) also open `explorer.html`.

## Rust endpoints on the chrome host

Served by `crates/vapurr-shell/src/host/` (not static files):

| path | crate |
|---|---|
| `/fomo/api/desk` | `vapurr-fomo` (crate still serves JSON; v1 chrome opens https://fomo.family) |
| `/scan/api/*` | `vapurr-rhc::scan` |
| `/scan/api/liq` | `vapurr-rhc::liq` â€” live RHC RPC market map. Stats are full; graph/lists are a capped view. |
| `/liq/api` | same snapshot, for swap/fomo to pull |
| `/liq/api/tape` | full RHC pair list for Ketcharts (CACHE, not Scan's capped view) |
| `/liq/api/trades/{pool}` | recent Swap logs for one pool. Cache + background RPC — not on the protocol thread |
| `/route/api/quote` | `vapurr-rhc::route` — scored routers, required sim, full route + `$VAPURR` refund, 25 bps buy/burn remainder → `$PUSD` |
| `/route/api/tokens` | `vapurr-rhc::route` |
| `/zzzmail/api/quote` | `vapurr-zmail` â€” 0.25Â¢ $PUSD/$VAPURR, gasless |
| `/zzzmail/api/me` | mailcard |
| `/zzzmail/api/inbox` | opened letters + pinset |
| `/zzzmail/api/send` | POST `{to, body, asset}` â€” seal, pin, voucher. Optional `subject` |
| `/zzzmail/api/hood` | PNS snapshot (primary + owned). Alias: `/zzzmail/api/pns`, `/hood/me` |
| `/zzzmail/api/hood/register` | POST `{name}` â€” claim `name.hood` on testnet 46630. Alias: `/pns/register` |
| `/zzzmail/api/pns/deploy` | POST â€” deploy `PnsRegistry` if missing |
| `/zzzmail/api/pns/set-addr` | POST `{name, addr}` â€” ENS `setAddr` |
| `/zzzmail/api/pns/set-name` | POST `{name}` â€” set reverse / primary |
| `/zzzmail/api/hood/resolve/{name}` | PNS resolve (addr + x25519). Alias: `/pns/resolve/{name}` |
| `/zzzmail/api/hood/reverse/{addr}` | PNS reverse `0x` â†’ `alice.hood`. Alias: `/pns/reverse/{addr}` |
| `/zzzmail/api/letter/{cid}` | open a pinned letter by CID |
| `/patch/api/status` | running build vs channel (`vapurr.next.exe`, `%LOCALAPPDATA%\vapurr\channel`, `VAPURR_CHANNEL`, repo `dist`). Apply is IPC `patch-apply`, not this GET. |

CLI: `vapurr.exe --publish` stamps this exe into the channel. `--patch-apply` swaps and relaunches. `--patch-swap` is the helper. `pack.ps1` publishes the same channel.

## Other frontend files

Not every file is a `vapurr://` id. Named so they do not rot:

- `pane.html` â€” unknown-id fallback
- `fomo.html` â€” redirect stub to https://fomo.family (`pane_url("fomo")` opens the live site, not this file)
- `zmail.html` â€” redirect stub to `zzzmail.html`
- `outbid.html` â€” redirect stub to `vapurrbid.html`
- `explorer.js`, `floor.js`, `ipc.js`, `shader.js`, `route.js`, `globe.js`, `radio.js`, `zzzmail.js`, `qr.js` â€” scripts
- `chrome.css`, `tokens.css`, `radio.css`
- `cat.svg`, `ketflix-logo.svg`, `ketflix.png`, `ketcharts-logo.svg`, `logo.png`, `maestro-logo.png`, `mascot.png`, `setup-mascot.png`, `404.png`, `zzzmail-icon.png`
- `setup.html` — branded first-run (`Install vapurr.exe`). Not a `vapurr://` pane.
- `vendor/vela.global.min.js` â€” Velaâ„¢ `@luxalgo/vela` 0.6.17 (Apache-2.0). NOTICE/LICENSE beside it. Attribution on the Ketcharts screen.
- `robinhood-chain-logo-black.svg`, `robinhood-chain-logo-white.svg`
- `cursors/arrow.svg`, `cursors/text.svg`
- `fonts/Sora-*.ttf`
- `vendor/three.webgpu.min.js`, `vendor/three.tsl.min.js`, `vendor/three.core.min.js` â€” PUSD globe (`pusd.html` / `globe.js`)
- `vendor/BloomNode.js`, `vendor/RoomEnvironment.js` â€” Three addons on disk; `globe.js` does not import them yet
- Scan Liquidity is SVG in `explorer.js`
- `frontend/ketbook/` — generated HonKit output. `npm run docs:app`. Served at `/ketbook/`. Not a `pane_url` file; `ketbook.html` frames it.
- `frontend/ketflix/` — posters, `catalog.json`, trailers. Served at `/ketflix/`.

## 404 vs KetPay
- **404** = branded page when navigation fails to load (asset `404.png`). Not payments.
- **KetPay** = `vapurr://pay` / `vapurr://ketpay` → `pay.html` (HTTP 402 / x402, `$PUSD` on 46630).
