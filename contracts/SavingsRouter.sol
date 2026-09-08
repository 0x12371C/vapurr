// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";
import "./SPUSD.sol";
import "./SpusdCd.sol";

/// Splits one post-runway remittance between liquid savings and CD coupons.
/// No deposits, borrowing, minting, or second runway floor. Enabled by default;
/// owner setAllocation(false, ...) is the safety killswitch.
contract SavingsRouter is IRemittance {
    uint256 public constant BPS = 10_000;

    IERC20Remit public immutable asset;
    RemittanceSink public immutable sink;
    SPUSD public immutable liquid;
    SpusdCd public immutable cd;
    address public owner;
    bool public enabled;
    uint256 public cdBps; // share of incoming surplus, NOT a coupon rate or APY
    uint256 public totalReceived;
    uint256 public totalLiquid;
    uint256 public totalCd;
    uint256 private _locked = 1;

    event OwnerUpdated(address indexed owner);
    event AllocationUpdated(bool enabled, uint256 cdBps);
    event Allocated(uint256 received, uint256 liquidCredit, uint256 cdCredit);

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

    constructor(address sink_, address liquid_, address cd_) {
        require(sink_ != address(0) && liquid_ != address(0) && cd_ != address(0), "TO");
        require(sink_ != liquid_ && sink_ != cd_ && liquid_ != cd_, "PAIR");
        sink = RemittanceSink(sink_);
        liquid = SPUSD(liquid_);
        cd = SpusdCd(cd_);
        asset = sink.asset();
        require(address(liquid.asset()) == address(asset) && address(cd.asset()) == address(asset), "ASSET");
        owner = msg.sender;
        enabled = true; // live-by-default; owner may disable via setAllocation
        cdBps = 2_500; // sane default: 25% of post-floor surplus to CD coupons
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    /// Applies only to future remittances; existing CD terms and principal stay fixed.
    function setAllocation(bool enabled_, uint256 cdBps_) external onlyOwner {
        require(cdBps_ <= BPS, "BPS");
        enabled = enabled_;
        cdBps = cdBps_;
        emit AllocationUpdated(enabled_, cdBps_);
    }

    function receiveRemittance(uint256 amount) external lock returns (bool) {
        require(msg.sender == address(sink), "SINK");
        require(enabled, "DISABLED");
        require(amount > 0, "TINY");
        uint256 beforeCash = asset.balanceOf(address(this));
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        uint256 received = asset.balanceOf(address(this)) - beforeCash;
        require(received > 0, "TINY");
        uint256 toCd = received * cdBps / BPS;
        uint256 toLiquid = received - toCd;

        // Never seed an empty liquid vault with yield that its first depositor can capture.
        // Reverting leaves the entire allocation at the sink, including its runway floor.
        if (toLiquid > 0) require(liquid.totalSupply() > liquid.DEAD_SHARES(), "EMPTY");
        uint256 liquidCredit = _credit(address(liquid), toLiquid);
        uint256 cdCredit = _credit(address(cd), toCd);
        totalReceived += received;
        totalLiquid += liquidCredit;
        totalCd += cdCredit;
        emit Allocated(received, liquidCredit, cdCredit);
        return true;
    }

    function _credit(address receiver, uint256 amount) internal returns (uint256 credited) {
        if (amount == 0) return 0;
        uint256 beforeCash = asset.balanceOf(receiver);
        require(asset.approve(receiver, amount), "ALLOW");
        require(IRemittance(receiver).receiveRemittance(amount), "REMIT");
        require(asset.approve(receiver, 0), "ALLOW");
        credited = asset.balanceOf(receiver) - beforeCash;
        require(credited > 0, "TINY");
    }
}
