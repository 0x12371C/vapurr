// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {gVAPURR} from "./GvFed.sol";

interface IVapurrTreasury {
    function approve(address spender, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

interface IPusdTreasury {
    function transfer(address to, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function approve(address spender, uint256 amt) external returns (bool);
}

interface IOliverTreasury {
    function depositV(uint256 amt) external;
    function borrow(uint256 amt) external;
    function repay(uint256 amt) external;
    function collatV(address user) external view returns (uint256);
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
}

/// 800k-carve treasury remainder locker.
/// NOT AMM dump float. At lock(): stake V as gV (yield + governance).
/// Only liquidity path: unstake → Oliver.depositV → borrow $PUSD.
/// withdrawV / claimV / unstakeToWallet revert NoMarketSell.
/// docs/econ/GENESIS_ALLOCATION.md
contract GenesisTreasury {
    IVapurrTreasury public immutable vapurr;
    gVAPURR public immutable gV;
    IOliverTreasury public immutable oliver;
    IPusdTreasury public immutable pusd;

    address public owner;
    address public recipient;
    uint256 public deposited;
    bool public locked;
    bool public recipientFrozen;

    event OwnerUpdated(address indexed owner);
    event RecipientUpdated(address indexed recipient);
    event Funded(uint256 amount);
    event Locked(uint256 stakedGv);
    event CollateralizedOliver(uint256 amount, uint256 totalCollat);
    event DrewPusd(address indexed to, uint256 amount);
    event Repaid(uint256 amount);

    error NotOwner();
    error NotRecipient();
    error ZeroAddr();
    error Tiny();
    error Live();
    error Frozen();
    error BadOliver();
    error BadGV();
    error NoMarketSell();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(address vapurr_, address gV_, address oliver_, address recipient_) {
        if (vapurr_ == address(0) || gV_ == address(0) || oliver_ == address(0) || recipient_ == address(0)) {
            revert ZeroAddr();
        }
        if (address(gVAPURR(gV_).vapurr()) != vapurr_) revert BadGV();
        IOliverTreasury o = IOliverTreasury(oliver_);
        if (o.vapurr() != vapurr_) revert BadOliver();
        vapurr = IVapurrTreasury(vapurr_);
        gV = gVAPURR(gV_);
        oliver = o;
        pusd = IPusdTreasury(o.pusd());
        recipient = recipient_;
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        if (o == address(0)) revert ZeroAddr();
        owner = o;
        emit OwnerUpdated(o);
    }

    function setRecipient(address r) external onlyOwner {
        if (recipientFrozen) revert Frozen();
        if (r == address(0)) revert ZeroAddr();
        recipient = r;
        emit RecipientUpdated(r);
    }

    function fund(uint256 amount) external onlyOwner {
        if (amount == 0) revert Tiny();
        require(vapurr.transferFrom(msg.sender, address(this), amount), "PULL");
        deposited += amount;
        emit Funded(amount);
    }

    /// Stake remaining V as gV. Recipient soulbound. No V to wallet.
    function lock() external onlyOwner {
        if (locked) revert Live();
        uint256 bal = vapurr.balanceOf(address(this));
        if (bal == 0 || deposited == 0) revert Tiny();
        locked = true;
        recipientFrozen = true;
        require(vapurr.approve(address(gV), bal), "ALLOW");
        uint256 shares = gV.stake(bal);
        emit Locked(shares);
    }

    /// Move gV (and any stray V) into Oliver as collatV[this].
    function collateralizeOliver(uint256 amount) public returns (uint256 moved) {
        if (msg.sender != owner && msg.sender != recipient) revert NotRecipient();
        if (!locked) revert Tiny();
        uint256 gvBal = gV.balanceOf(address(this));
        uint256 vBal = vapurr.balanceOf(address(this));
        uint256 want = amount == type(uint256).max ? gvBal + vBal : amount;
        if (want == 0) revert Tiny();
        if (want > vBal) {
            uint256 fromGv = want - vBal;
            if (fromGv > gvBal) fromGv = gvBal;
            if (fromGv > 0) gV.unstake(fromGv);
        }
        moved = vapurr.balanceOf(address(this));
        if (moved > want) moved = want;
        if (moved == 0) revert Tiny();
        require(vapurr.approve(address(oliver), moved), "ALLOW");
        oliver.depositV(moved);
        emit CollateralizedOliver(moved, oliver.collatV(address(this)));
    }

    function drawPusd(uint256 amount) external returns (uint256 paid) {
        if (msg.sender != recipient && msg.sender != owner) revert NotRecipient();
        if (oliver.collatV(address(this)) == 0) {
            uint256 gvBal = gV.balanceOf(address(this));
            uint256 vBal = vapurr.balanceOf(address(this));
            if (gvBal + vBal > 0) collateralizeOliver(type(uint256).max);
        }
        paid = amount;
        if (paid == 0) revert Tiny();
        uint256 before = pusd.balanceOf(address(this));
        oliver.borrow(paid);
        uint256 got = pusd.balanceOf(address(this)) - before;
        if (got < paid) paid = got;
        if (paid == 0) revert Tiny();
        require(pusd.transfer(recipient, paid), "PUSD");
        emit DrewPusd(recipient, paid);
    }

    function repayPusd(uint256 amount) external returns (uint256 repaid) {
        if (msg.sender != recipient && msg.sender != owner) revert NotRecipient();
        if (amount == 0) revert Tiny();
        require(pusd.transferFrom(msg.sender, address(this), amount), "PULL");
        require(pusd.approve(address(oliver), amount), "ALLOW");
        oliver.repay(amount);
        repaid = amount;
        emit Repaid(repaid);
    }

    function withdrawV(address, uint256) external pure {
        revert NoMarketSell();
    }

    function claimV(address, uint256) external pure {
        revert NoMarketSell();
    }

    function unstakeToWallet(address, uint256) external pure {
        revert NoMarketSell();
    }

    function stakedGv() external view returns (uint256) {
        return gV.balanceOf(address(this));
    }

    function oliverCollateral() external view returns (uint256) {
        return oliver.collatV(address(this));
    }
}
