// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";

/// House protocol-fee carve sketch: realized $PUSD fee inventory -> RemittanceSink.
///
/// Uni v4 LP fees stay with LPs. This contract is the *protocol* carve surface
/// (ops / hook / swapper skim) â€” not a second local runway floor.
/// Floor remains sink-level on RemittanceSink (ROUTING.md).
///
/// INVARIANT: only creditFees with cash already in hand (transferFrom). Never mint.
/// INVARIANT: remitSurplus pushes feeReserve only; empty reverts TINY.
contract HouseFeeRemit {
    IERC20Remit public immutable pusd;
    address public owner;
    IRemittance public remittance;
    uint256 public feeReserve; // realized protocol fee cash, inventory-backed

    event OwnerUpdated(address indexed owner);
    event RemittanceSet(address indexed sink);
    event FeesCredited(address indexed from, uint256 amount);
    event Remitted(address indexed sink, uint256 amount);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address pusd_) {
        require(pusd_ != address(0), "TO");
        pusd = IERC20Remit(pusd_);
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    function setRemittance(address sink) external onlyOwner {
        remittance = IRemittance(sink);
        emit RemittanceSet(sink);
    }

    /// Pull realized protocol fee carve into feeReserve (inventory only).
    function creditFees(uint256 amount) external returns (uint256) {
        require(amount > 0, "TINY");
        require(pusd.transferFrom(msg.sender, address(this), amount), "PULL");
        feeReserve += amount;
        // Cap ledger to cash (donation-safe).
        uint256 cash = pusd.balanceOf(address(this));
        if (feeReserve > cash) feeReserve = cash;
        emit FeesCredited(msg.sender, amount);
        return amount;
    }

    /// Push realized feeReserve to RemittanceSink. amount==0 remits all.
    function remitSurplus(uint256 amount) public returns (uint256 sent) {
        require(address(remittance) != address(0), "REMIT");
        uint256 free = feeReserve;
        uint256 cash = pusd.balanceOf(address(this));
        if (free > cash) free = cash;
        sent = amount == 0 ? free : amount;
        require(sent > 0 && sent <= free, "TINY");
        feeReserve -= sent;
        require(pusd.approve(address(remittance), sent), "ALLOW");
        require(remittance.receiveRemittance(sent), "SINK");
        // Dust / donation: keep ledger <= cash after pull.
        cash = pusd.balanceOf(address(this));
        if (feeReserve > cash) feeReserve = cash;
        emit Remitted(address(remittance), sent);
    }
}
