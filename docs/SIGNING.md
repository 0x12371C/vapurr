# Code signing — v1 ship gate

## Problem (2026-09-04)

Windows Defender quarantines the public download as `Trojan:Win32/Wacatac.C!ml`.

- Artifact: `https://thesecretlab.app/vapurr/vapurr-setup.exe`
- SHA256: `50075f0652ba0d97cdc087950aed787c08880faa959e2c5f9122a17c84bf2f1c`
- Independent check (ClamAV + PE/strings): clean; matches our 1.1.9 pack
- `!ml` = machine-learning heuristic on **unsigned** new installers — strangers hit this too

**Unsigned public Install/setup is not shippable.** Local "Allow on device" does not fix downloads for anyone else.

## Real fix

1. **Authenticode-sign** `vapurr.exe` / `Install vapurr.exe` / `vapurr-setup.exe` (and preferably staged `WebView2Loader.dll`).
2. **Timestamp** every signature.
3. Wire signing into `pack.ps1` so every release is signed before TSL upload.
4. Submit the hash as a false positive to Microsoft Security Intelligence (helps ML; does not replace signing).

## Cert choice (2026)

EV no longer buys instant SmartScreen bypass (since ~2024). OV and EV build reputation the same way: clean download volume.

| Option | Notes |
|--------|-------|
| **Microsoft Artifact Signing** (Trusted Signing) | ~$10/mo, cloud HSM, CI-friendly, identity validation required |
| **OV code signing** (Sectigo / DigiCert / SSL.com) | Org-validated; key on hardware token or cloud HSM |
| **EV code signing** | Only for kernel drivers / some enterprise procurement — not required just for SmartScreen |

Self-signed does **not** help strangers.

## Until signed

- Do not market the Windows download as clean-install for strangers.
- Keep publishing hashes on the download page.
- Hot-patch / channel still works for Relic after Allow, but is not the public path.

## pack.ps1 hook (live)

After stage build, if `$env:VAPURR_SIGN_CERT` (thumbprint) or Artifact Signing creds are set, run `signtool sign /fd SHA256 /tr ... /td SHA256` on staged exes. If unset, pack prints `UNSIGNED — not stranger-shippable` and refuses TSL upload unless `$env:VAPURR_ALLOW_UNSIGNED=1`.


## pack.ps1 hook (live)

`pack.ps1` defines `Invoke-VapurrSign` / `Find-SignTool`.

```powershell
$env:VAPURR_SIGN_CERT = '<thumbprint>'          # required to sign
$env:VAPURR_SIGN_TIMESTAMP = 'http://timestamp.digicert.com'  # optional
$env:VAPURR_REQUIRE_SIGN = '1'                  # fail pack if unsigned
$env:VAPURR_ALLOW_UNSIGNED = '1'                # override REQUIRE_SIGN for local packs
.\pack.ps1
```

Order: build → stage copies → **sign `$exe`** → refresh Install/setup from signed exe → hash/manifest/channel/zip.
Without `VAPURR_SIGN_CERT`, pack warns `UNSIGNED` and continues unless `VAPURR_REQUIRE_SIGN=1`.

## SignPath Foundation (OSS) — code signing policy

vapurr intends to use [SignPath Foundation](https://signpath.org/) free Authenticode signing for public Windows builds.

### Policy (what we submit to SignPath)

- **Source of truth:** the public GitHub repository for vapurr (MIT-licensed).
- **What gets signed:** release artifacts produced by CI from that repository — at minimum `Install vapurr.exe` / `vapurr-setup.exe` (and the zip that contains them).
- **What does not get signed by hand:** developer laptops do not hold the SignPath private key. Local `pack.ps1` + `VAPURR_SIGN_CERT` remains an alternate path if we later buy our own OV cert.
- **Build provenance:** only CI-built binaries from the listed workflows are submitted for signing. Ad-hoc local builds are not submitted as “release”.
- **Distribution:** signed builds are published at `https://thesecretlab.app/vapurr` (and optionally GitHub Releases). Downloads are free.
- **Malware:** we do not ship malware, credential stealers, or security-circumvention tools. Public Defender `Wacatac.C!ml` on *unsigned* builds is a known false-positive class; signing is the fix.
- **Publisher name:** Windows may show **SignPath Foundation** as the publisher while we are on the OSS program.

### Apply checklist

1. Public repo + MIT `LICENSE` (done in-tree).
2. This policy page linked from README / SIGNING.
3. Apply at SignPath.io OSS with repo + download URL.
4. Wire GitHub Actions → SignPath artifact signing → publish to TSL.

