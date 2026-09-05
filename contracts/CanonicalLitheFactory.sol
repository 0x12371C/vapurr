// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {VapurrToken, RebasePolicy, gVAPURR} from "./GvFed.sol";
import {PusdMarketFed} from "./PusdMarketFed.sol";
import {PusdLoop} from "./PusdLoop.sol";
import {LegacyVConverter} from "./LegacyVConverter.sol";
import {ILegacyLitheMarket, LitheCutoverMigrator} from "./LitheCutoverMigrator.sol";
import {GenesisAllocation} from "./GenesisAllocation.sol";

interface ILegacyVSupply {
    function totalSupply() external view returns (uint256);
}

/// One-transaction successor deployment for canonical V and canonical Lithe.
///
/// HARD LOCK: genesis mint is exactly GENESIS_MINT (1_000_000 launch + 200_000
/// DevFund). Legacy converter inventory is carved from TREASURY_GROSS (800k) so
/// total minted stays 1.2M — never mint legacy on top.
///
/// Then: fund converter, send remaining launch+DevFund to initiator for
/// LaunchBootstrap (BrowserStream / POL / House / GenesisTreasury), assign Lithe
/// as marketMinter, hand policy minter to gV.
///
/// bootstrapV_ constructor arg is ignored; launch size is LAUNCH_V.
/// Remittance / sPUSD / House remain post-deploy (initiator).
contract CanonicalLitheFactory is GenesisAllocation {
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
    uint256 public immutable devFundAllocation;
    uint256 public immutable treasuryNet;

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
        uint256 bootstrapV,
        uint256 devFundAllocation,
        uint256 treasuryNet
    );

    constructor(address legacyMarket_, address legacyV_, uint256 legacyVSupply_, uint256 bootstrapV_, uint256 rate_) {
        require(legacyMarket_ != address(0) && legacyV_ != address(0) && legacyVSupply_ > 0 && rate_ > 0, "TO");
        require(ILegacyLitheMarket(legacyMarket_).vapurr() == legacyV_, "LEGACY");
        require(ILegacyVSupply(legacyV_).totalSupply() == legacyVSupply_, "SUPPLY");
        require(legacyVSupply_ <= TREASURY_GROSS, "TREASURY");
        // bootstrapV_ is deprecated (lean/fat overrides). Launch is locked at 1M.
        bootstrapV_;

        legacyMarket = legacyMarket_;
        legacyV = legacyV_;
        legacyVSupply = legacyVSupply_;
        bootstrapV = LAUNCH_V;
        devFundAllocation = DEV_FUND_AMOUNT;
        treasuryNet = TREASURY_GROSS - legacyVSupply_;

        canonicalV = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(canonicalV), address(policy));
        policy.bindGV(address(gV));

        converter = new LegacyVConverter(legacyV_, address(canonicalV));
        market = new PusdMarketFed(address(canonicalV), rate_, msg.sender);
        migrator = new LitheCutoverMigrator(legacyMarket_, address(market), address(converter));
        loop = new PusdLoop(address(market));

        // Exactly 1.2M. Converter inventory carved from the 800k remainder.
        canonicalV.mint(address(this), GENESIS_MINT);
        require(canonicalV.totalSupply() == GENESIS_MINT, "MINT");
        require(canonicalV.approve(address(converter), legacyVSupply_), "ALLOW");
        converter.fund(legacyVSupply_);
        require(canonicalV.transfer(msg.sender, GENESIS_MINT - legacyVSupply_), "DEV");

        // Dual printers: Lithe = seigniorage marketMinter; gV = policy minter.
        canonicalV.setMarketMinter(address(market));
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
            LAUNCH_V,
            DEV_FUND_AMOUNT,
            treasuryNet
        );
    }
}
