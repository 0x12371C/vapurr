# VAPURR — Style Reference
> void canvas, lime cat, one chromatic signal. A dark terminal for money that still feels like a product, not a dapp screenshot.

**Theme:** dark
**Mark:** geometric **cat** (not a fox). Vapor + purr.
**Spec image:** `assets/brand-board.png` — reference only. Do not blit it into the UI.

VAPURR sits on near-black (`#0E0E0E`) with a single lime (`#c0f800`) used for the cat, icons, 404, and primary actions. Type is Sora. Display wordmark is huge, tracked, white. Body never uses full white — muted sage-gray. Elevation is color steps, not drop shadows. Motion is a lime thinking-orb (MetalForge-class: particles, pulse, glass rings) behind the home wordmark — generated, not a video.

## Tokens — Colors

Shipped values live in `frontend/tokens.css`. Keep this table matched to that file.

| Name | Value | Token | Role |
|------|-------|-------|------|
| Lime | `#c0f800` | `--color-lime` | The only chromatic. Cat stroke, icons, primary fill, 404, connected pip |
| Forest | `#2a3800` | `--color-forest` | Hover/pressed on void, tile borders, sheen |
| Void | `#0E0E0E` | `--color-void` | Page canvas |
| Steel | `#1F2327` | `--color-steel` | Tiles, inputs, elevated modules |
| Snow | `#F2F3F4` | `--color-snow` | Display / wordmark / primary text |
| Muted | `#8AA090` | `--color-muted` | Body, hints, secondary labels |

Optional **light** theme (`html[data-theme="light"]` in `frontend/tokens.css`). Sage, not cream/paper:

| Name | Value | Role |
|------|-------|------|
| Lime | `#4d8a00` | chromatic on light void |
| Forest | `#c8d6b0` | raised / hover |
| Void | `#f3f5f0` | page canvas |
| Steel | `#e6ebe0` | tiles, inputs |
| Snow | `#161816` | primary text |
| Muted | `#3f5340` | body, hints |

Default remains dark. Toggle is the rail sun/moon and Settings.

## Tokens — Typography

**Sora** (Regular 400 UI, SemiBold 600 wordmark). No second display face.

| Role | Size | Weight | Line height | Tracking |
|------|------|--------|-------------|----------|
| mono-label | 11px | 600 | 1.2 | 0.12em uppercase |
| body-sm | 13px | 400 | 1.4 | 0 |
| body | 15px | 400 | 1.45 | 0 |
| subhead | 18px | 400 | 1.3 | 0 |
| heading | 28px | 600 | 1.05 | 0.02em |
| display | 72–88px | 600 | 0.92 | 0.08em **VAPURR** |

## Tokens — Spacing & Shape

**Base unit:** 4px

| Name | Value |
|------|-------|
| rail | 64px |
| tile gap | 12px |
| card padding | 16–24px |
| section | 32–48px |

| Element | Radius |
|---------|--------|
| inputs / buttons / nav | 8px |
| tiles / cards | 12–16px |
| search | 10px |
| pill | 9999px |
| physical card plate | 14px |

## Components

### Cat mark
Lime stroke geometric cat: vertical ears, inner-ear notches, almond eyes filled lime, short muzzle, whiskers. Drawn as paths. Never a raster of the board.

### Home display
Centered **VAPURR** in Sora SemiBold 72–88px snow, tracking 0.08em, sitting on a lime thinking-orb (pulse rings + particles, lime on void).

### Command search
Steel fill, forest 1px stroke, 10px radius. Placeholder: `Search the chain or type a command`. Lime arrow submit.

### Feature tile
Steel plate, 12px radius, lime line-icon, snow 13px label. Home tiles today: Wallet, Swap, Stake, Bridge, PUSD, vapurrbid, PNS, Live Trenches, Scan, dApps.

### KetPay sheet
Product name **KetPay**. Wire is HTTP 402 / x402. Amount in snow 36px. Lime CTA. 404 is load-fail only.

### VAPURR card
Void plate, forest hairline, drawn cat, `VAPURR` / `BY AVALANCHE` / `VISA`. Diagonal steel sheen. No photo of the board.

### Sidebar rail
64px void. Cat at top. Line icons: home, wallet, PUSD, Live Trenches, scan, history, bookmarks, earn, zzzmail, settings, theme, shield. Lime when active.

## Do

- One chromatic: lime. Everything else is void/steel/snow/muted.
- Cat, not fox. Purr is the name.
- Sora only.
- Flat elevation (void → steel → forest). No drop shadows.
- Thinking-orb motion in lime only.

## Don't

- Do not paste `brand-board.png` into the chrome.
- Do not introduce a second accent in vapurr chrome (no purple, no orange, no BAT orange). Maestro radio may keep its own palette.
- Do not invent a cream/paper mock. Light theme is the sage set in `tokens.css` if it is on.
- Do not set body copy to `#FFFFFF`.
- Do not treat WebView2 / wry as the **product** engine. They are the current https guest. Product engine is Servo.
