// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Spendable exogenous RFV sink for BondMarket.treasury (future bonds).
/// NOT a replacement for GenesisTreasury V-carve locker.
/// Relic: retarget BondMarket STOCKS treasury here after deploy; existing GT balances stay sunk.
interface IERC20 {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function approve(address, uint256) external returns (bool);
}

contract ExoRfvSink {
    address public owner;
    mapping(address => bool) public keeper;

    event OwnerUpdated(address indexed owner);
    event KeeperUpdated(address indexed keeper, bool allowed);
    event Spent(address indexed token, address indexed to, uint256 amount);
    event Approved(address indexed token, address indexed spender, uint256 amount);

    error NotOwner();
    error NotKeeper();
    error ZeroAddr();
    error Tiny();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    modifier onlyKeeper() {
        if (msg.sender != owner && !keeper[msg.sender]) revert NotKeeper();
        _;
    }

    constructor(address owner_) {
        if (owner_ == address(0)) revert ZeroAddr();
        owner = owner_;
    }

    function setOwner(address o) external onlyOwner {
        if (o == address(0)) revert ZeroAddr();
        owner = o;
        emit OwnerUpdated(o);
    }

    function setKeeper(address k, bool allowed) external onlyOwner {
        if (k == address(0)) revert ZeroAddr();
        keeper[k] = allowed;
        emit KeeperUpdated(k, allowed);
    }

    function spend(address token, address to, uint256 amount) external onlyKeeper returns (bool) {
        if (token == address(0) || to == address(0)) revert ZeroAddr();
        if (amount == 0) revert Tiny();
        require(IERC20(token).transfer(to, amount), "XFER");
        emit Spent(token, to, amount);
        return true;
    }

    function approveSpender(address token, address spender, uint256 amount) external onlyKeeper returns (bool) {
        if (token == address(0) || spender == address(0)) revert ZeroAddr();
        require(IERC20(token).approve(spender, amount), "ALLOW");
        emit Approved(token, spender, amount);
        return true;
    }
}
