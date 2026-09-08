# PROXY DEPLOY GATE proof -- docs/econ/PROXY_DEPLOY_GATE.md
# Reads the EIP-1967 implementation slot for -Address and exits 0 only if
# it is non-zero (a real proxy pointing at a deployed impl). Exits non-zero
# on a bare/zero impl slot, an RPC error, or `code == 0x` at the target.
#
# Usage:
#   powershell -File scripts/verify-proxy.ps1 -Address 0x...
#   powershell -File scripts/verify-proxy.ps1 -Address 0x... -Rpc https://rpc.testnet.chain.robinhood.com

param(
  [Parameter(Mandatory = $true)][string]$Address,
  [string]$Rpc = "https://rpc.testnet.chain.robinhood.com"
)

$ErrorActionPreference = "Stop"

# keccak256("eip1967.proxy.implementation") - 1
$ImplSlot = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"
$AdminSlot = "0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103"

function Invoke-Rpc([string]$Method, [object[]]$Params) {
  $body = @{ jsonrpc = "2.0"; id = 1; method = $Method; params = $Params } | ConvertTo-Json -Depth 6
  $resp = Invoke-RestMethod -Uri $Rpc -Method Post -ContentType "application/json" -Body $body
  if ($resp.error) { throw "RPC error: $($resp.error.message)" }
  return $resp.result
}

try {
  $code = Invoke-Rpc -Method "eth_getCode" -Params @($Address, "latest")
  if ($code -eq "0x" -or [string]::IsNullOrEmpty($code)) {
    Write-Host "FAIL: no code at $Address"
    exit 1
  }

  $implRaw = Invoke-Rpc -Method "eth_getStorageAt" -Params @($Address, $ImplSlot, "latest")
  $adminRaw = Invoke-Rpc -Method "eth_getStorageAt" -Params @($Address, $AdminSlot, "latest")
  $implAddr = "0x" + $implRaw.Substring($implRaw.Length - 40)
  $adminAddr = "0x" + $adminRaw.Substring($adminRaw.Length - 40)

  if ($implAddr -eq "0x0000000000000000000000000000000000000000") {
    Write-Host "FAIL: EIP-1967 impl slot is zero at $Address -- bare implementation or wrong target."
    Write-Host "Do not wire this address into DEPLOYED / TESTNET_* / treasury / market.json."
    exit 1
  }

  Write-Host "PASS: $Address impl slot -> $implAddr"
  if ($adminAddr -ne "0x0000000000000000000000000000000000000000") {
    Write-Host "note: admin slot set -> $adminAddr (transparent proxy shape, or vapurr ERC1967Proxy's constructor admin -- gate keys only on impl != 0)"
  }
  exit 0
} catch {
  Write-Host "FAIL: $($_.Exception.Message)"
  exit 1
}
