// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";

/// Time-locked sPUSD CD sketch — surplus-funded coupon, break fee on early exit.
/// See docs/econ/SPUSD.md. Does not mint $PUSD or touch gV rebase.

contract SpusdCd is IRemittance {
    IERC20Remit public immutable asset; // $PUSD
    address public owner;

    uint256 public couponBps; // e.g. 500 = 5% of principal at maturity
    uint256 public breakFeeBps; // e.g. 200 = 2% on early exit → surplus
    uint256 public term; // seconds
    uint256 public surplus; // $PUSD reserved for coupons / break fees retained

    struct Position {
        address owner;
        uint256 principal;
        uint64 unlockAt;
        bool open;
    }

    uint256 public nextId = 1;
    mapping(uint256 => Position) public positions;

    event Opened(uint256 indexed id, address indexed owner, uint256 principal, uint64 unlockAt);
    event Closed(uint256 indexed id, address indexed owner, uint256 principalOut, uint256 couponOut, uint256 feeToSurplus);
    event SurplusCredited(address indexed from, uint256 amount);
    event Params(uint256 couponBps, uint256 breakFeeBps, uint256 term);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address asset_, uint256 couponBps_, uint256 breakFeeBps_, uint256 term_) {
        require(asset_ != address(0), "TO");
        require(couponBps_ <= 10_000 && breakFeeBps_ <= 10_000 && term_ > 0, "PARAM");
        asset = IERC20Remit(asset_);
        owner = msg.sender;
        couponBps = couponBps_;
        breakFeeBps = breakFeeBps_;
        term = term_;
        emit Params(couponBps_, breakFeeBps_, term_);
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
    }

    function setParams(uint256 couponBps_, uint256 breakFeeBps_, uint256 term_) external onlyOwner {
        require(couponBps_ <= 10_000 && breakFeeBps_ <= 10_000 && term_ > 0, "PARAM");
        couponBps = couponBps_;
        breakFeeBps = breakFeeBps_;
        term = term_;
        emit Params(couponBps_, breakFeeBps_, term_);
    }

    /// Remittance / surplus credit for coupons (no new CD shares).
    function receiveRemittance(uint256 amount) external returns (bool) {
        require(amount > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        surplus += amount;
        emit SurplusCredited(msg.sender, amount);
        return true;
    }

    function open(uint256 principal) external returns (uint256 id) {
        require(principal > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), principal), "PULL");
        id = nextId++;
        uint64 unlockAt = uint64(block.timestamp + term);
        positions[id] = Position({owner: msg.sender, principal: principal, unlockAt: unlockAt, open: true});
        emit Opened(id, msg.sender, principal, unlockAt);
    }

    function close(uint256 id) external returns (uint256 principalOut, uint256 couponOut, uint256 feeToSurplus) {
        Position storage p = positions[id];
        require(p.open && p.owner == msg.sender, "POS");
        p.open = false;
        uint256 principal = p.principal;

        if (block.timestamp < p.unlockAt) {
            feeToSurplus = (principal * breakFeeBps) / 10_000;
            principalOut = principal - feeToSurplus;
            couponOut = 0;
            surplus += feeToSurplus;
        } else {
            feeToSurplus = 0;
            principalOut = principal;
            uint256 due = (principal * couponBps) / 10_000;
            couponOut = due <= surplus ? due : surplus;
            surplus -= couponOut;
        }

        uint256 pay = principalOut + couponOut;
        require(asset.transfer(msg.sender, pay), "PUSD");
        emit Closed(id, msg.sender, principalOut, couponOut, feeToSurplus);
    }
}