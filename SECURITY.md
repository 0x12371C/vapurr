# Security

Nothing in this repository is a live wallet.

Device key, login session, market addresses, and the WebView2 profile are written at runtime under `%LOCALAPPDATA%\vapurr`:

- `device.sk`
- `session.json`
- `market.json`
- `treasury.json`
- `edge\` (WebView2 user data)

Do not commit those files. Do not paste seeds, keys, or crash dumps that contain them into issues.

Report key-material and chrome bugs privately. Public issues are fine for UI and protocol questions that do not include secrets.
