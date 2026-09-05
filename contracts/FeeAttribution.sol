// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";

/// Contribution ledger: branch remittances tagged by source (House / Lithe / Oliver)
/// before cash lands in RemittanceSink.
///
/// Wire path (tagged):
///   HouseFeeRemit / PusdMarket / PusdLoop  ->  FeeAttribution  ->  RemittanceSink
///     (realized $PUSD)                         (who paid)         (runway floor)
///
/// Branches may still remit direct to RemittanceSink (untagged). This contract is
/// the SoT for "who paid the yield" when UI/TVL need source shares.
///
/// INVARIANT: never mints. Inventory transferFrom only.
/// INVARIANT: does not apply runway floor - RemittanceSink remains sole floor SoT.
/// INVARIANT: Unknown source is allowed on receiveRemittance (unregistered branch)
///            but credit() requires an explicit House/Lithe/Oliver tag.
contract FeeAttribution is IRemittance {
    enum Source {
        Unknown,
        House,
        Lithe,
        Oliver
    }

    IERC20Remit public immutable asset;
    IRemittance public immutable sink;
    address public owner;

    mapping(address => Source) public sourceOf;
    mapping(Source => uint256) public contributed; // lifetime $PUSD by source
    uint256 public totalContributed;

    event OwnerUpdated(address indexed owner);
    event BranchRegistered(address indexed branch, Source source);
    event Contributed(address indexed from, Source source, uint256 amount);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address asset_, address sink_) {
        require(asset_ != address(0) && sink_ != address(0), "TO");
        asset = IERC20Remit(asset_);
        sink = IRemittance(sink_);
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    /// Map a branch / fee carve contract to a source bucket.
    function register(address branch, Source src) external onlyOwner {
        require(branch != address(0), "TO");
        sourceOf[branch] = src;
        emit BranchRegistered(branch, src);
    }

    /// IRemittance: pull from msg.sender, attribute via sourceOf, forward to sink.
    function receiveRemittance(uint256 amount) external returns (bool) {
        require(amount > 0, "TINY");
        Source src = sourceOf[msg.sender];
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        _record(msg.sender, src, amount);
        require(asset.approve(address(sink), amount), "ALLOW");
        require(sink.receiveRemittance(amount), "SINK");
        return true;
    }

    /// Explicit tagged credit (ops / adapters / tests). Rejects Unknown.
    function credit(Source src, uint256 amount) external returns (uint256) {
        require(src == Source.House || src == Source.Lithe || src == Source.Oliver, "SRC");
        require(amount > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        _record(msg.sender, src, amount);
        require(asset.approve(address(sink), amount), "ALLOW");
        require(sink.receiveRemittance(amount), "SINK");
        return amount;
    }

    /// Share of lifetime contributions in bps (0 if empty).
    function shareBps(Source src) external view returns (uint256) {
        if (totalContributed == 0) return 0;
        return (contributed[src] * 10_000) / totalContributed;
    }

    /// Convenience: House / Lithe / Oliver lifetime totals (skips Unknown).
    function breakdown()
        external
        view
        returns (uint256 house, uint256 lithe, uint256 oliver, uint256 total)
    {
        house = contributed[Source.House];
        lithe = contributed[Source.Lithe];
        oliver = contributed[Source.Oliver];
        total = totalContributed;
    }

    function _record(address from, Source src, uint256 amount) internal {
        contributed[src] += amount;
        totalContributed += amount;
        emit Contributed(from, src, amount);
    }
}