// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Relic HARD LOCK 2026-09-05: genesis mint shape before setMinter(gV).
/// 1_000_000 launch + 200_000 DevFund = 1_200_000 V. Legacy converter is
/// carved from the 800k treasury remainder — never minted on top.
/// See docs/econ/GENESIS_ALLOCATION.md.
contract GenesisAllocation {
    uint256 public constant LAUNCH_V = 1_000_000 ether;
    uint256 public constant DEV_FUND_AMOUNT = 200_000 ether;
    uint256 public constant GENESIS_MINT = 1_200_000 ether;

    uint256 public constant BROWSERSTREAM_V = 50_000 ether;
    uint256 public constant POL_ETH_V = 80_000 ether;
    uint256 public constant POL_NVDA_V = 25_000 ether;
    uint256 public constant POL_AMD_V = 25_000 ether;
    uint256 public constant HOUSE_SEED_V = 20_000 ether;
    uint256 public constant TREASURY_GROSS = 800_000 ether;
}
