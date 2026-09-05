# Build the downloadable tree. Same flags as a user-facing release â€” not target-cpu=native.
$ErrorActionPreference = "Stop"

function Find-SignTool {
  $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  $roots = @(
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
    "${env:ProgramFiles}\Windows Kits\10\bin"
  )
  foreach ($root in $roots) {
    if (-not (Test-Path $root)) { continue }
    $hit = Get-ChildItem $root -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
      Where-Object { $_.FullName -match '\\x64\\' } |
      Sort-Object FullName -Descending |
      Select-Object -First 1
    if ($hit) { return $hit.FullName }
  }
  return $null
}

# Authenticode hook — see docs/SIGNING.md
# Env:
#   VAPURR_SIGN_CERT     = cert thumbprint (CurrentUser\My or LocalMachine\My)
#   VAPURR_SIGN_TIMESTAMP = TSA URL (default DigiCert)
#   VAPURR_REQUIRE_SIGN  = 1 to fail pack when unsigned (stranger-ship gate)
#   VAPURR_ALLOW_UNSIGNED = 1 to override REQUIRE_SIGN for local/dev packs
function Invoke-VapurrSign {
  param([Parameter(Mandatory=$true)][string[]]$Paths)
  $existing = @($Paths | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -Unique)
  if ($existing.Count -eq 0) { return $false }

  $thumb = ($env:VAPURR_SIGN_CERT -as [string]).Trim()
  if (-not $thumb) {
    Write-Warning "UNSIGNED — VAPURR_SIGN_CERT not set. Public downloads will trip Defender Wacatac.C!ml. See docs/SIGNING.md"
    if ($env:VAPURR_REQUIRE_SIGN -eq '1' -and $env:VAPURR_ALLOW_UNSIGNED -ne '1') {
      throw "pack: VAPURR_REQUIRE_SIGN=1 but VAPURR_SIGN_CERT is empty (set VAPURR_ALLOW_UNSIGNED=1 to override)"
    }
    return $false
  }

  $signtool = Find-SignTool
  if (-not $signtool) {
    throw "pack: signtool.exe not found (install Windows SDK). Needed to sign with VAPURR_SIGN_CERT=$thumb"
  }
  $tsa = ($env:VAPURR_SIGN_TIMESTAMP -as [string]).Trim()
  if (-not $tsa) { $tsa = 'http://timestamp.digicert.com' }

  foreach ($p in $existing) {
    Write-Output "sign $p"
    & $signtool sign /fd SHA256 /td SHA256 /tr $tsa /sha1 $thumb /v -- $p
    if ($LASTEXITCODE -ne 0) { throw "pack: signtool failed on $p (exit $LASTEXITCODE)" }
    $sig = Get-AuthenticodeSignature -LiteralPath $p
    if ($sig.Status -ne 'Valid') {
      throw "pack: signature not Valid on $p — Status=$($sig.Status) $($sig.StatusMessage)"
    }
    Write-Output ("signed {0} subject={1}" -f $p, $sig.SignerCertificate.Subject)
  }
  return $true
}

function Use-GnuCc {
  if (Get-Command gcc.exe -ErrorAction SilentlyContinue) { return }
  $candidates = @(
    $env:WINLIBS_BIN,
    (Join-Path $env:USERPROFILE "winlibs\mingw64\bin"),
    (Join-Path $env:LOCALAPPDATA "winlibs\mingw64\bin"),
    "C:\winlibs\mingw64\bin",
    "C:\mingw64\bin"
  ) | Where-Object { $_ }
  foreach ($p in $candidates) {
    if (Test-Path (Join-Path $p "gcc.exe")) {
      $env:Path = $p + ";" + $env:Path
      return
    }
  }
  Write-Warning "gcc.exe not on PATH. Install WinLibs mingw64 or set WINLIBS_BIN."
}
Use-GnuCc
Set-Location $PSScriptRoot
# Kill running vapurr so channel/Programs/dist copies are not locked.
Get-Process vapurr -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400

Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue

$ver = "1.1.0"
$m = Select-String -Path (Join-Path $PSScriptRoot "Cargo.toml") -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
if ($m) { $ver = $m.Matches[0].Groups[1].Value }

# Ketbook (HonKit) is rust-embedded from frontend/ketbook/.
if (Get-Command npm -ErrorAction SilentlyContinue) {
  npm run docs:app
  if ($LASTEXITCODE -ne 0) { Write-Warning "ketbook build failed; packing without it" }
}

# --target so artifacts land in target\x86_64-pc-windows-gnu\release
# (host gnu without --target writes target\release) and so
# .cargo/config.toml [target.x86_64-pc-windows-gnu] rustflags apply.
cargo +stable-x86_64-pc-windows-gnu build --release -p vapurr-shell --target x86_64-pc-windows-gnu
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$exeDir = Join-Path $PSScriptRoot "target\x86_64-pc-windows-gnu\release"
$exe = Join-Path $exeDir "vapurr.exe"
if (-not (Test-Path $exe)) {
  Write-Error "pack: missing $exe"
  exit 1
}

# Public download gate: release bins must not ship home-dir paths.
if (-not $env:VAPURR_SKIP_PATH_GATE) {
  $hay = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($exe))
  $user = $env:USERNAME
  $needle = "C:\Users\$user"
  $pathHits = ([regex]::Matches($hay, [regex]::Escape($needle))).Count
  if ($pathHits -gt 0) {
    Write-Error "pack: refusing release - exe contains $pathHits path leak(s) for $needle (remap-path-prefix in .cargo/config.toml). VAPURR_SKIP_PATH_GATE=1 to override."
    exit 1
  }
  Write-Output "path-gate ok (no $needle in exe)"
}

$loader = Get-ChildItem (Join-Path $exeDir "build") -Recurse -Filter "WebView2Loader.dll" -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -match "\\x64\\" } |
  Select-Object -First 1
if (-not $loader) {
  $loader = Get-ChildItem $exeDir -Filter "WebView2Loader.dll" | Select-Object -First 1
}
if (-not $loader) {
  Write-Error "pack: missing WebView2Loader.dll next to the gnu release"
  exit 1
}



$distRoot = Join-Path $PSScriptRoot "dist"
$stage = Join-Path $distRoot "vapurr"
New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
if (Test-Path $stage) {
  try {
    Remove-Item $stage -Recurse -Force -ErrorAction Stop
  } catch {
    Write-Warning "pack: dist\vapurr is in use (running vapurr.exe?). Staging beside it."
    $stage = Join-Path $distRoot "vapurr-new"
    if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
  }
}
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item $exe (Join-Path $stage "vapurr.exe") -Force
Copy-Item $exe (Join-Path $stage "Install vapurr.exe") -Force
Copy-Item $exe (Join-Path $distRoot "vapurr-setup.exe") -Force
Copy-Item $loader.FullName (Join-Path $stage "WebView2Loader.dll") -Force
# Never ship a sibling vapurr.exe â€” that makes â€œrun without installingâ€ the real app.
$stray = Join-Path $stage "vapurr.exe"
if (Test-Path $stray) { Remove-Item $stray -Force }
# Zip timestamps cannot be before 1980. The loader DLL from mingw is dated 1973.
Get-ChildItem $stage -File | ForEach-Object { $_.LastWriteTime = Get-Date }

# Sign release exe first (channel + Install/setup are copies of it). Hash AFTER sign.
$null = Invoke-VapurrSign -Paths @($exe, (Join-Path $stage 'Install vapurr.exe'), (Join-Path $distRoot 'vapurr-setup.exe'), (Join-Path $stage 'WebView2Loader.dll'))
# Refresh staged Install/setup from signed $exe so all public artifacts share one signature blob.
if ($env:VAPURR_SIGN_CERT) {
  Copy-Item $exe (Join-Path $stage 'Install vapurr.exe') -Force
  Copy-Item $exe (Join-Path $distRoot 'vapurr-setup.exe') -Force
  $null = Invoke-VapurrSign -Paths @((Join-Path $stage 'Install vapurr.exe'), (Join-Path $distRoot 'vapurr-setup.exe'))
  Get-ChildItem $stage -File | ForEach-Object { $_.LastWriteTime = Get-Date }
}

$built = (Get-Date).ToUniversalTime().ToString("o")
$sha = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLowerInvariant()
$rev = ""
try { $rev = (git -C $PSScriptRoot rev-parse --short HEAD 2>$null | ForEach-Object { $_.Trim() }) } catch { $rev = "" }
$size = (Get-Item $exe).Length
$manifest = @"
{
  "name": "vapurr",
  "version": "$ver",
  "build": "$built",
  "rev": "$rev",
  "sha256": "$sha",
  "size": $size,
  "target": "x86_64-pc-windows-gnu",
  "file": "vapurr.exe"
}
"@
@"
vapurr $ver
windows x86_64-pc-windows-gnu
built $built
rev $rev
sha $sha
"@ | Set-Content -Path (Join-Path $stage "VERSION.txt") -Encoding ascii
Set-Content -Path (Join-Path $stage "manifest.json") -Value $manifest -Encoding ascii
$chan = Join-Path $env:LOCALAPPDATA "vapurr\channel"
New-Item -ItemType Directory -Force -Path $chan | Out-Null
Copy-Item $exe (Join-Path $chan "vapurr.exe") -Force
Copy-Item $loader.FullName (Join-Path $chan "WebView2Loader.dll") -Force -ErrorAction SilentlyContinue
Set-Content -Path (Join-Path $chan "manifest.json") -Value $manifest -Encoding ascii
Copy-Item (Join-Path $stage "VERSION.txt") (Join-Path $chan "VERSION.txt") -Force
Write-Output "channel $chan"

Copy-Item (Join-Path $PSScriptRoot "LICENSE") (Join-Path $stage "LICENSE.txt") -Force

@"
vapurr $ver

Open Install vapurr.exe. No administrator account is required.

Installs to  %LOCALAPPDATA%\Programs\vapurr
Profile      %LOCALAPPDATA%\vapurr

Requires Microsoft Edge WebView2 Runtime
https://go.microsoft.com/fwlink/p/?LinkId=2124703
"@ | Set-Content -Path (Join-Path $stage "README.txt") -Encoding utf8

$zip = Join-Path $distRoot "vapurr-$ver-windows-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
# Compress-Archive needs a folder named vapurr at the zip root.
$zipSrc = Join-Path $distRoot "_zip-vapurr"
if (Test-Path $zipSrc) { Remove-Item $zipSrc -Recurse -Force }
New-Item -ItemType Directory -Path $zipSrc | Out-Null
Copy-Item $stage (Join-Path $zipSrc "vapurr") -Recurse -Force
Compress-Archive -Path (Join-Path $zipSrc "vapurr") -DestinationPath $zip -Force
Remove-Item $zipSrc -Recurse -Force

Write-Output "packed $stage"
Write-Output "zip    $zip"
Get-ChildItem $stage | ForEach-Object { "{0}`t{1}" -f $_.Name, $_.Length }
Get-Item $zip | ForEach-Object { "{0}`t{1}" -f $_.Name, $_.Length }

$alias = Join-Path $distRoot "vapurr-windows.zip"
Copy-Item $zip $alias -Force
Write-Output "alias  $alias"
$setupOut = Join-Path $distRoot "vapurr-setup.exe"
Copy-Item $exe $setupOut -Force
if ($env:VAPURR_SIGN_CERT) { $null = Invoke-VapurrSign -Paths @($setupOut) }
Write-Output "setup  $setupOut"
Get-Item $setupOut | ForEach-Object { "{0}`t{1}" -f $_.Name, $_.Length }
