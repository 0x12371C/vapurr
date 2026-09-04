# Build the downloadable tree. Same flags as a user-facing release — not target-cpu=native.
$ErrorActionPreference = "Stop"
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
Copy-Item $loader.FullName (Join-Path $stage "WebView2Loader.dll") -Force
# Never ship a sibling vapurr.exe — that makes “run without installing” the real app.
$stray = Join-Path $stage "vapurr.exe"
if (Test-Path $stray) { Remove-Item $stray -Force }
# Zip timestamps cannot be before 1980. The loader DLL from mingw is dated 1973.
Get-ChildItem $stage -File | ForEach-Object { $_.LastWriteTime = Get-Date }

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
