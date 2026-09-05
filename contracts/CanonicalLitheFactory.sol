// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {VapurrToken, RebasePolicy, gVAPURR} from "./GvFed.sol";
import {PusdMarketFed} from "./PusdMarketFed.sol";
import {PusdLoop} from "./PusdLoop.sol";
import {LegacyVConverter} from "./LegacyVConverter.sol";
import {ILegacyLitheMarket, LitheCutoverMigrator} from "./LitheCutoverMigrator.sol";

interface ILegacyVSupply {
    function totalSupply() external view returns (uint256);
}

/// One-transaction successor deployment for canonical V and canonical Lithe.
///
/// The factory verifies the legacy V supply, pre-funds 1:1 conversion inventory,
/// creates the Lithe-to-Lithe PUSD route, seeds explicit canonical-Lithe inventory,
/// and hands all configurable roles to the initiating wallet before construction ends.
contract CanonicalLitheFactory {
    VapurrToken public immutable canonicalV;
    RebasePolicy public immutable policy;
    gVAPURR public immutable gV;
    PusdMarketFed public immutable market;
    PusdLoop public immutable loop;
    LegacyVConverter public immutable converter;
    LitheCutoverMigrator public immutable migrator;
    address public immutable legacyMarket;
    address public immutable legacyV;
    uint256 public immutable legacyVSupply;
    uint256 public immutable bootstrapV;

    event Deployed(
        address indexed initiator,
        address indexed legacyMarket,
        address indexed canonicalV,
        address market,
        address loop,
        address converter,
        address migrator,
        address policy,
        address gV,
        uint256 legacyVSupply,
        uint256 bootstrapV
    );

    constructor(address legacyMarket_, address legacyV_, uint256 legacyVSupply_, uint256 bootstrapV_, uint256 rate_) {
        require(legacyMarket_ != address(0) && legacyV_ != address(0) && legacyVSupply_ > 0 && rate_ > 0, "TO");
        require(ILegacyLitheMarket(legacyMarket_).vapurr() == legacyV_, "LEGACY");
        require(ILegacyVSupply(legacyV_).totalSupply() == legacyVSupply_, "SUPPLY");

        legacyMarket = legacyMarket_;
        legacyV = legacyV_;
        legacyVSupply = legacyVSupply_;
        bootstrapV = bootstrapV_;

        canonicalV = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(canonicalV), address(policy));
        policy.bindGV(address(gV));

        converter = new LegacyVConverter(legacyV_, address(canonicalV));
        market = new PusdMarketFed(address(canonicalV), rate_, msg.sender);
        migrator = new LitheCutoverMigrator(legacyMarket_, address(market), address(converter));
        loop = new PusdLoop(address(market));

        canonicalV.mint(address(this), legacyVSupply_ + bootstrapV_);
        require(canonicalV.approve(address(converter), legacyVSupply_), "ALLOW");
        converter.fund(legacyVSupply_);
        if (bootstrapV_ > 0) {
            require(canonicalV.approve(address(market), bootstrapV_), "ALLOW");
            market.fundVInventory(bootstrapV_);
        }

        // No factory role survives deployment: gV is the sole V minter and the
        // initiating wallet controls the policy and successor vault configuration.
        canonicalV.setMinter(address(gV));
        policy.setOwner(msg.sender);
        loop.setOwner(msg.sender);

        emit Deployed(
            msg.sender,
            legacyMarket_,
            address(canonicalV),
            address(market),
            address(loop),
            address(converter),
            address(migrator),
            address(policy),
            address(gV),
            legacyVSupply_,
            bootstrapV_
        );
    }
}
