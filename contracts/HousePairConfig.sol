// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title HousePairConfig - rebase-safe House equity/cash pairing
/// @notice Canon: House AMM leg is **wgV / $PUSD** only (see docs/econ/HOUSE_PAIR.md).
/// Raw rebasing gVAPURR must never appear as a Uni v4 / CPMM currency on the House book.
/// This config is the deploy-time / factory check until HouseLp/HouseSwap are rewired off market.vapurr().
interface IHousePairConfig {
    function wgV() external view returns (address);
    function pusd() external view returns (address);
    function gV() external view returns (address);
    function requireHouseEquity(address token) external view;
    function requireHousePair(address a, address b) external view;
    function isHousePair(address a, address b) external view returns (bool);
}

contract HousePairConfig is IHousePairConfig {
    address public immutable override wgV;
    address public immutable override pusd;
    /// Banned as a pool currency (rebasing equity). Wrap to wgV first.
    address public immutable override gV;

    error RawGvNotHouseEquity();
    error BadHousePair();
    error ZeroAddr();

    constructor(address wgV_, address pusd_, address gV_) {
        if (wgV_ == address(0) || pusd_ == address(0) || gV_ == address(0)) revert ZeroAddr();
        require(wgV_ != gV_, "WRAP");
        require(wgV_ != pusd_ && gV_ != pusd_, "PAIR");
        wgV = wgV_;
        pusd = pusd_;
        gV = gV_;
    }

    /// Equity leg MUST be the non-rebasing wrapper. Raw gV reverts RawGvNotHouseEquity.
    function requireHouseEquity(address token) public view override {
        if (token == gV) revert RawGvNotHouseEquity();
        require(token == wgV, "EQUITY");
    }

    /// Pool currencies must be exactly {wgV, pusd} (either order). Raw gV is never allowed.
    function requireHousePair(address a, address b) public view override {
        if (a == gV || b == gV) revert RawGvNotHouseEquity();
        bool ok = (a == wgV && b == pusd) || (a == pusd && b == wgV);
        if (!ok) revert BadHousePair();
    }

    function isHousePair(address a, address b) external view override returns (bool) {
        if (a == gV || b == gV || a == address(0) || b == address(0)) return false;
        return (a == wgV && b == pusd) || (a == pusd && b == wgV);
    }
}

/// Thin factory/mock: validates PoolKey currencies before any initialize/seed path.
/// Not a full Uni v4 PositionManager - documents the gate live wiring must call.
contract HousePairFactory {
    IHousePairConfig public immutable config;

    event PoolValidated(address currency0, address currency1, bytes32 poolId);

    constructor(address config_) {
        require(config_ != address(0), "CFG");
        config = IHousePairConfig(config_);
    }

    /// Rejects raw gV (and any non-{wgV,pusd} pair) before pool init.
    function validateAndMark(address currency0, address currency1) external returns (bytes32 poolId) {
        config.requireHousePair(currency0, currency1);
        poolId = keccak256(abi.encode(currency0, currency1));
        emit PoolValidated(currency0, currency1, poolId);
    }
}