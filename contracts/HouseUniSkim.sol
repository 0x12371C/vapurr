// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./HouseFeeRemit.sol";

/// Uni protocol-fee skim adapter (sketch).
///
/// Future Uni v4 hook / swapper lands *realized* $PUSD protocol carve here,
/// then this adapter forwards inventory into HouseFeeRemit.creditFees.
/// LP fees stay with LPs — this is protocol carve only.
///
/// INVARIANT: never mints. Only transferFrom + creditFees (inventory).
/// INVARIANT: only authorized hook/owner may skim.
/// Not a full Uni v4 IHooks implementation — inventory bridge for overnight prove.
contract HouseUniSkim {
    IERC20Remit public immutable pusd;
    HouseFeeRemit public immutable feeRemit;
    address public owner;
    address public hook; // authorized Uni hook / swapper

    event OwnerUpdated(address indexed owner);
    event HookUpdated(address indexed hook);
    event Skimmed(address indexed from, uint256 amount);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    modifier onlyHook() {
        require(msg.sender == hook || msg.sender == owner, "AUTH");
        _;
    }

    constructor(address pusd_, address feeRemit_) {
        require(pusd_ != address(0) && feeRemit_ != address(0), "TO");
        pusd = IERC20Remit(pusd_);
        feeRemit = HouseFeeRemit(feeRemit_);
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    function setHook(address h) external onlyOwner {
        require(h != address(0), "TO");
        hook = h;
        emit HookUpdated(h);
    }

    /// Pull realized protocol carve from the hook and credit HouseFeeRemit.
    /// amount must already be held/approved by msg.sender (hook inventory).
    function skimToCredit(uint256 amount) external onlyHook returns (uint256) {
        require(amount > 0, "TINY");
        require(pusd.transferFrom(msg.sender, address(this), amount), "PULL");
        require(pusd.approve(address(feeRemit), amount), "ALLOW");
        uint256 credited = feeRemit.creditFees(amount);
        emit Skimmed(msg.sender, credited);
        return credited;
    }
}