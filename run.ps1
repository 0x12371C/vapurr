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
}
Use-GnuCc
Set-Location $PSScriptRoot

# Process name is "Install vapurr" when we launch the zip stub — not "vapurr".
Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -match '(?i)vapurr' -or
    ($_.ExecutablePath -and $_.ExecutablePath -match '(?i)vapurr')
  } |
  ForEach-Object {
    & taskkill.exe /F /T /PID $_.ProcessId 2>$null | Out-Null
  }
Start-Sleep -Milliseconds 400

& (Join-Path $PSScriptRoot "pack.ps1")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$exe = Join-Path $PSScriptRoot "dist\vapurr\vapurr.exe"
if (-not (Test-Path $exe)) {
  $exe = Join-Path $PSScriptRoot "dist\vapurr-new\vapurr.exe"
}
if (-not (Test-Path $exe)) {
  $exe = Join-Path $PSScriptRoot "dist\vapurr\Install vapurr.exe"
}
if (-not (Test-Path $exe)) {
  Write-Error "packed exe missing"
  exit 1
}

# Start-Process children die with the packing shell. WMI Create is a separate process.
# Quote the path — "Install vapurr.exe" has a space (WMI return 9 otherwise).
# --app skips the setup window so we get the browser.
$startup = ([wmiclass]"Win32_ProcessStartup").CreateInstance()
$startup.ShowWindow = 1
$cmd = '"{0}" --app' -f $exe
$result = ([wmiclass]"Win32_Process").Create($cmd, (Split-Path $exe), $startup)
if ($result.ReturnValue -ne 0) {
  Write-Error "Win32_Process.Create failed: $($result.ReturnValue)"
  exit 1
}
Write-Output "started pid=$($result.ProcessId)"
