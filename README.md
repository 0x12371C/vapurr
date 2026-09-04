# vapurr

Windows browser for Robinhood Chain.

The shell is a native process (`tao` + wry). Chrome is HTML served at `http://vapurr.localhost`. Guest pages use WebView2. Wallet, mail, Scan, and DeFi stay in-process — they do not spawn a site renderer.

Mark is a cat. Palette and type: `DESIGN.md`, `frontend/tokens.css`.

## Requirements

- Rust stable, Windows GNU target (`x86_64-pc-windows-gnu`)
- `gcc` / `windres` on `PATH` (WinLibs mingw64, or set `WINLIBS_BIN`)
- Microsoft Edge WebView2 Runtime
- Optional: Node.js, only to rebuild Ketbook (`npm run docs:app`)

## Build

```
.\pack.ps1
```

Writes `dist\vapurr-<version>-windows-x64.zip`. Open `Install vapurr.exe`. Profile and keys live under `%LOCALAPPDATA%\vapurr`, not in this tree.

Git tracks source. `dist/`, `target/`, overnight logs, and `frontend/ketflix/trailers/*.mp4` stay local. Trailers are not rust-embedded.

```
cargo +stable-x86_64-pc-windows-gnu test --workspace
```

`.\run.ps1` packs and launches. Do not run it against a window someone is using.

## Layout

| Path | What |
|---|---|
| `crates/` | Workspace. `vapurr-shell` is the window. |
| `frontend/` | Chrome HTML/CSS/JS, rust-embedded in the exe |
| `contracts/` | `PusdMarket`, Outbid, KetList, PNS, MockUsdg |
| `ketbook/` | Product book (HonKit). In-app: `vapurr://ketbook` |
| `docs/` | Ship bar, chrome map, status |
| `assets/` | Brand board and fonts (reference, not blit into chrome) |

## Home chain

Source of truth: `crates/vapurr-rhc/src/lib.rs`.

- Robinhood Chain `4663`
- USDG `0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168` (6 decimals)
- RPC `https://rpc.mainnet.chain.robinhood.com`
- Explorer `https://robinhoodchain.blockscout.com`
- Testnet `46630` for econ bootstrap until mainnet has gas

## Docs

- [ARCHITECTURE.md](ARCHITECTURE.md) — process model and crates
- [docs/V1.md](docs/V1.md) — what v1 is and is not
- [docs/STATUS.md](docs/STATUS.md) — what already runs
- [docs/SURFACES.md](docs/SURFACES.md) — `vapurr://` map
- [BRAND.md](BRAND.md) / [DESIGN.md](DESIGN.md)

## License

MIT. See [LICENSE](LICENSE). Vendor notices sit next to the files they cover (`frontend/vendor/`).
