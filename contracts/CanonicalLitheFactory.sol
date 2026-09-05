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
/// The factory verifies the legacy V supply, pre-funds 1:1 conversion inventory
/// (LegacyVConverter only — not Lithe redeem float), creates the Lithe-to-Lithe
/// PUSD route, allocates genesis DevFund (200k V) to the initiator for LaunchBootstrap,
/// assigns Lithe as Fed V marketMinter (seigniorage), hands policy minter to gV,
/// and hands remaining roles to the initiating wallet before construction ends.
///
/// Does NOT auto-deploy ExogenousPairRegistry / DevFundStream / House — use
/// LaunchBootstrap companion with the DevFund allocation before any further mint.
/// Remittance / sPUSD wiring remains post-deploy (initiator).
contract CanonicalLitheFactory {
    /// Genesis developer fund allocation (minted once, then setMinter(gV)).
    /// Initiator wires LaunchBootstrap / DevFundStream — see docs/econ/DEV_FUND.md.
    uint256 public constant DEV_FUND_AMOUNT = 200_000 ether;

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
        uint256 devFundAllocation
    );

    constructor(address legacyMarket_, address legacyV_, uint256 legacyVSupply_, uint256 bootstrapV_, uint256 rate_) {
        require(legacyMarket_ != address(0) && legacyV_ != address(0) && legacyVSupply_ > 0 && rate_ > 0, "TO");
        require(ILegacyLitheMarket(legacyMarket_).vapurr() == legacyV_, "LEGACY");
        require(ILegacyVSupply(legacyV_).totalSupply() == legacyVSupply_, "SUPPLY");

        legacyMarket = legacyMarket_;
        legacyV = legacyV_;
        legacyVSupply = legacyVSupply_;
        bootstrapV = bootstrapV_;
        devFundAllocation = DEV_FUND_AMOUNT;

        canonicalV = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(canonicalV), address(policy));
        policy.bindGV(address(gV));

        converter = new LegacyVConverter(legacyV_, address(canonicalV));
        market = new PusdMarketFed(address(canonicalV), rate_, msg.sender);
        migrator = new LitheCutoverMigrator(legacyMarket_, address(market), address(converter));
        loop = new PusdLoop(address(market));

        // Genesis mint: conversion inventory + optional bootstrap float + DevFund (before handoff).
        // bootstrapV_ goes to initiator as liquid float — Lithe redeem no longer needs inventory.
        canonicalV.mint(address(this), legacyVSupply_ + bootstrapV_ + DEV_FUND_AMOUNT);
        require(canonicalV.approve(address(converter), legacyVSupply_), "ALLOW");
        converter.fund(legacyVSupply_);
        // DevFund + optional bootstrap float to initiator.
        require(canonicalV.transfer(msg.sender, DEV_FUND_AMOUNT + bootstrapV_), "DEV");

        // Dual printers: Lithe = seigniorage marketMinter; gV = policy minter (staker rebase).
        // Order matters — setMarketMinter while factory still holds policy minter.
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
            bootstrapV_,
            DEV_FUND_AMOUNT
        );
    }
}