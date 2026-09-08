# Mine CREATE2 salt for Lithe vanity proxy on testnet 46630.
# Target: 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2
# Does NOT deploy. Requires: create2 factory address, proxy initCodeHash.
#
# Usage:
#   .\scripts\mine-lithe-vanity.ps1 -Factory 0x... -InitCodeHash 0x... [-Start 0] [-Limit 5000000]
#
# Get initCodeHash from: forge script script/LitheVanityDeploy.s.sol:LitheVanityDeploy -vvvv

param(
  [Parameter(Mandatory = $true)][string]$Factory,
  [Parameter(Mandatory = $true)][string]$InitCodeHash,
  [ulong]$Start = 0,
  [ulong]$Limit = 5000000
)

$ErrorActionPreference = "Stop"
$Target = "0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2".ToLowerInvariant()
$Factory = $Factory.ToLowerInvariant()
if (-not $Factory.StartsWith("0x")) { $Factory = "0x$Factory" }
if (-not $InitCodeHash.StartsWith("0x")) { $InitCodeHash = "0x$InitCodeHash" }

Write-Host "Mining salt for $Target"
Write-Host " factory=$Factory"
Write-Host " initCodeHash=$InitCodeHash"
Write-Host " range=[$Start, $($Start+$Limit))"

# Prefer cast if available
$cast = Get-Command cast -ErrorAction SilentlyContinue
if (-not $cast) {
  Write-Host "cast not on PATH ? install foundry or run mining via forge/cast on a machine that has it."
  Write-Host "Example: cast create2 --deployer $Factory --init-code-hash $InitCodeHash --starts-with C47f"
  exit 1
}

# cast create2 can search by prefix; vanity full match may need extended search.
# Documented approach: use cast's vanity helper when present, else iterate.
Write-Host "Trying cast create2 vanity search (prefix C47f00)..."
& cast create2 --deployer $Factory --init-code-hash $InitCodeHash --starts-with c47f00 2>&1 | Write-Host
Write-Host ""
Write-Host "If cast returns a salt, verify full address == $Target before any broadcast."
Write-Host "Relic approval required to deploy. See docs/econ/TESTNET_PROXY_46630.md"
