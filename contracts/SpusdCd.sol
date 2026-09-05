// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";

/// Time-locked PUSD savings. Terms are fixed at entry; coupons depend on funded surplus.
/// Underfunded coupons share cash pro rata across all open coupon targets, including
/// unmatured positions. Closing settles the position; unpaid targets are not arrears.
/// Principal remains in this contract and is never lent or counted as coupon funding.
contract SpusdCd is IRemittance {
    IERC20Remit public immutable asset;
    address public owner;

    uint256 public couponBps; // target coupon per term, NOT annual APY
    uint256 public breakFeeBps;
    uint256 public term;
    uint256 public surplus; // credited coupon cash + retained early-exit fees
    uint256 public totalPrincipal;
    uint256 public totalCouponDue; // sum of fixed targets on all open positions
    uint256 private _locked = 1;

    struct Position {
        address owner;
        uint256 principal;
        uint64 unlockAt;
        bool open;
    }

    uint256 public nextId = 1;
    mapping(uint256 => Position) public positions;
    mapping(uint256 => uint256) public couponDue;
    mapping(uint256 => uint256) public positionBreakFeeBps;

    event Opened(uint256 indexed id, address indexed owner, uint256 principal, uint64 unlockAt);
    event TermsLocked(uint256 indexed id, uint256 couponDue, uint256 breakFeeBps);
    event Closed(
        uint256 indexed id, address indexed owner, uint256 principalOut, uint256 couponOut, uint256 feeToSurplus
    );
    event SurplusCredited(address indexed from, uint256 amount);
    event Params(uint256 couponBps, uint256 breakFeeBps, uint256 term);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    modifier lock() {
        require(_locked == 1, "LOCK");
        _locked = 2;
        _;
        _locked = 1;
    }

    constructor(address asset_, uint256 couponBps_, uint256 breakFeeBps_, uint256 term_) {
        require(asset_ != address(0), "TO");
        asset = IERC20Remit(asset_);
        owner = msg.sender;
        _setParams(couponBps_, breakFeeBps_, term_);
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
    }

    /// Changes the offer for future positions only.
    function setParams(uint256 couponBps_, uint256 breakFeeBps_, uint256 term_) external onlyOwner {
        _setParams(couponBps_, breakFeeBps_, term_);
    }

    function _setParams(uint256 couponBps_, uint256 breakFeeBps_, uint256 term_) internal {
        require(couponBps_ <= 10_000 && breakFeeBps_ <= 10_000 && term_ > 0, "PARAM");
        require(term_ <= type(uint64).max - block.timestamp, "TERM");
        couponBps = couponBps_;
        breakFeeBps = breakFeeBps_;
        term = term_;
        emit Params(couponBps_, breakFeeBps_, term_);
    }

    /// Credits only received assets. Donations / PUSD rebases do not silently become coupon budget.
    function receiveRemittance(uint256 amount) external lock returns (bool) {
        uint256 received = _pull(amount);
        surplus += received;
        emit SurplusCredited(msg.sender, received);
        return true;
    }

    function open(uint256 principal) external lock returns (uint256 id) {
        require(term <= type(uint64).max - block.timestamp, "TERM");
        principal = _pull(principal);
        id = nextId++;
        uint64 unlockAt = uint64(block.timestamp + term);
        positions[id] = Position({owner: msg.sender, principal: principal, unlockAt: unlockAt, open: true});
        uint256 due = principal * couponBps / 10_000;
        couponDue[id] = due;
        positionBreakFeeBps[id] = breakFeeBps;
        totalPrincipal += principal;
        totalCouponDue += due;
        emit Opened(id, msg.sender, principal, unlockAt);
        emit TermsLocked(id, due, breakFeeBps);
    }

    /// Nominal PUSD coupon cash, capped by inventory after all open principal claims.
    function availableSurplus() public view returns (uint256) {
        uint256 cash = asset.balanceOf(address(this));
        uint256 free = cash > totalPrincipal ? cash - totalPrincipal : 0;
        return surplus < free ? surplus : free;
    }

    function previewClose(uint256 id)
        public
        view
        returns (uint256 principalOut, uint256 couponOut, uint256 feeToSurplus)
    {
        Position storage p = positions[id];
        require(p.open, "POS");
        if (block.timestamp < p.unlockAt) {
            feeToSurplus = p.principal * positionBreakFeeBps[id] / 10_000;
            principalOut = p.principal - feeToSurplus;
        } else {
            principalOut = p.principal;
            uint256 due = couponDue[id];
            uint256 free = availableSurplus();
            if (due > 0) {
                // No first-closer sweep: each target receives its share of funded cash.
                couponOut = free >= totalCouponDue ? due : free * due / totalCouponDue;
            }
        }
    }

    function close(uint256 id) external lock returns (uint256 principalOut, uint256 couponOut, uint256 feeToSurplus) {
        Position storage p = positions[id];
        require(p.open && p.owner == msg.sender, "POS");
        require(asset.balanceOf(address(this)) >= totalPrincipal, "PRINCIPAL");
        (principalOut, couponOut, feeToSurplus) = previewClose(id);
        p.open = false;
        totalPrincipal -= p.principal;
        totalCouponDue -= couponDue[id];
        surplus = surplus + feeToSurplus - couponOut;
        uint256 pay = principalOut + couponOut;
        if (pay > 0) require(asset.transfer(msg.sender, pay), "PUSD");
        emit Closed(id, msg.sender, principalOut, couponOut, feeToSurplus);
    }

    function _pull(uint256 amount) internal returns (uint256 received) {
        require(amount > 0, "TINY");
        uint256 beforeCash = asset.balanceOf(address(this));
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        received = asset.balanceOf(address(this)) - beforeCash;
        require(received > 0, "TINY");
    }
}
