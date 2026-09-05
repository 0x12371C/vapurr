# SignPath OSS application — draft answers for vapurr

## Links to paste
- Repository: https://github.com/0x12371C/vapurr
- License: MIT (see LICENSE in repo root; Cargo.toml license = "MIT")
- Code signing policy: https://github.com/0x12371C/vapurr/blob/master/docs/SIGNING.md
- Free download / distribution: https://thesecretlab.app/vapurr
- Primary artifacts: vapurr-setup.exe, vapurr-windows.zip (Install vapurr.exe inside)

## Project description (short)
vapurr is a native Windows on-chain browser for Robinhood Chain. MIT-licensed. Public downloads are free. We need Authenticode signing because Windows Defender flags our unsigned installer as Trojan:Win32/Wacatac.C!ml (ML heuristic). Independent checks (ClamAV + PE inspection) show the build is our own release binary.

## What should be signed
CI-built Windows release binaries from this repository:
- Install vapurr.exe / vapurr-setup.exe (PE)
- optionally WebView2Loader.dll shipped beside them
- the release zip that contains those files

Local developer packs are not submitted as release unless produced by the signed CI path.

## Build / provenance (honest current state)
Today releases are packed with pack.ps1 on a Windows builder. For SignPath we will wire GitHub Actions on master/tags so only CI artifacts are submitted for signing (required by SignPath). pack.ps1 already has an Authenticode hook (VAPURR_SIGN_CERT) as a fallback if we later use our own OV cert.

## Publisher name expectation
We understand Windows may show "SignPath Foundation" as publisher while on the OSS program.

## Contact
GitHub: 0x12371C

## Checklist before submit
- [x] Public repo
- [x] OSI license (MIT)
- [x] Free download URL
- [x] Signing policy doc in repo
- [ ] SignPath account / apply form submitted
- [ ] GitHub Actions workflow that uploads artifacts to SignPath

## Form paste (2026-09-04)

- Project name: vapurr
- Repository URL: https://github.com/0x12371C/vapurr
- Project homepage: https://thesecretlab.app/vapurr
- Download URL (page, not raw exe): https://thesecretlab.app/vapurr
  - Note: that page must mention SignPath Foundation for code signing (also mirrored in README). Prefer the page URL over a direct .exe link.
- Privacy Policy URL: https://github.com/0x12371C/vapurr/blob/master/PRIVACY.md
- Wikipedia: leave blank
- Tagline: Native Windows on-chain browser for Robinhood Chain.
- Description: vapurr is an MIT-licensed Windows browser for Robinhood Chain with free public downloads. We need Authenticode signing because Windows Defender ML flags our unsigned installer (Wacatac.C!ml false positive). Independent ClamAV/PE checks match our release build. CI will submit PE artifacts (vapurr-setup.exe / Install vapurr.exe) to SignPath; see docs/SIGNING.md.

