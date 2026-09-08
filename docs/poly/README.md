# Polymarket research desk (`vapurr://poly`)

Chrome surface for local Polymarket trade history and the pendulumflow orderbook archive.

## Open

- Desk hub: omnibox `vapurr://poly` (aliases: `polymarket`, `polydata`, `polydesk`) or Home / dApps **Poly** tile
- Archive in-tab: `vapurr://pendulum` (aliases: `polyarchive`, `pmarchive`) or desk CTA / dApps **Archive** tile -> https://archive.pendulumflow.com

## Services

1. **poly_data** - https://github.com/warproxxx/poly_data
   Local Polymarket trade history (who / when / price / size / counterparty). First sync long; updates fast. Needs `HYPERSYNC_API` with HyperSync product access.

2. **pendulumflow archive** - https://archive.pendulumflow.com
   Second-by-second book events. Donation-supported. Ready-made Claude Code questions on the front page. Credit pendulumflow; say it runs on donations if you publish work that used it.

## Honesty

- No fake metrics. UI shows **not configured** until a real local status exists.
- No income / raise claims in this surface.
- Does not touch wallet or KYC.

## Sync commands (Windows)

```powershell
powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
git clone https://github.com/warproxxx/poly_data.git
cd poly_data
# .env: HYPERSYNC_API=... from https://envio.dev/app/api-tokens
uv sync
uv run poly-data
```

Resolver: `pane_url` in `crates/vapurr-shell/src/nav.rs`. Row in `docs/SURFACES.md`.
