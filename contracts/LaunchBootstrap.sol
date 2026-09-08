// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {ExogenousPairRegistry, ExogenousSeedMarket, EXO_TAG_ETH, EXO_TAG_NVDA, EXO_TAG_AMD} from "./ExogenousPairRegistry.sol";
import {DevFundStream} from "./DevFundStream.sol";
import {BrowserStream} from "./GvFed.sol";
import {GenesisTreasury} from "./GenesisTreasury.sol";
import {GenesisAllocation} from "./GenesisAllocation.sol";

interface IVapurrApprove {
    function approve(address spender, uint256 amt) external returns (bool);
    function transfer(address to, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function totalSupply() external view returns (uint256);
}

/// Companion to CanonicalLitheFactory cutover.
/// Allocates the 1M launch pile + 200k DevFund pulled from initiator:
///   50k BrowserStream · 80k V/ETH · 25k V/NVDA · 25k V/AMD · 20k House seed
///   + treasuryNet (800k − legacy carve) into GenesisTreasury (gV + Oliver).
/// DevFund V auto-locks as Oliver collateral; recipient draws $PUSD only.
/// Does NOT deploy V/USDG or PUSD/USDG. No silent chain deploy.
contract LaunchBootstrap is GenesisAllocation {
    ExogenousPairRegistry public immutable registry;
    DevFundStream public immutable devFund;
    BrowserStream public immutable browserStream;
    GenesisTreasury public immutable treasury;
    address public immutable vapurr;
    address public immutable oliver;
    address public immutable gV;
    address public immutable initiator;
    uint256 public immutable treasuryRemainder;

    address public ethSeed;
    address public nvdaSeed;
    address public amdSeed;
    bool public funded;
    uint256 public houseSeedHeld;

    event Bootstrapped(
        address indexed initiator,
        address registry,
        address devFund,
        address browserStream,
        address treasury,
        uint256 treasuryRemainder
    );
    event DevFundStarted(address indexed stream, uint256 amount);
    event Allocated(
        uint256 browser,
        uint256 polEth,
        uint256 polNvda,
        uint256 polAmd,
        uint256 house,
        uint256 treasuryNet
    );
    event HouseSeedClaimed(address indexed to, uint256 amount);

    error ZeroAddr();
    error BadFund();
    error NotInitiator();
    error AlreadyFunded();
    error BadTreasury();

    constructor(
        address vapurr_,
        address oliver_,
        address gV_,
        address recipient_,
        address usdg_,
        address pusd_,
        address eth_,
        address nvda_,
        address amd_,
        uint256 treasuryRemainder_,
        bool seedPol_
    ) {
        if (vapurr_ == address(0) || oliver_ == address(0) || gV_ == address(0) || recipient_ == address(0)) {
            revert ZeroAddr();
        }
        if (eth_ == address(0) || nvda_ == address(0) || amd_ == address(0)) revert ZeroAddr();
        if (treasuryRemainder_ > TREASURY_GROSS) revert BadTreasury();

        vapurr = vapurr_;
        oliver = oliver_;
        gV = gV_;
        initiator = msg.sender;
        treasuryRemainder = treasuryRemainder_;

        registry = new ExogenousPairRegistry(vapurr_, usdg_, pusd_);
        registry.registerPair(EXO_TAG_ETH, eth_, true);
        registry.registerPair(EXO_TAG_NVDA, nvda_, true);
        registry.registerPair(EXO_TAG_AMD, amd_, true);

        devFund = new DevFundStream(vapurr_, oliver_, recipient_);
        browserStream = new BrowserStream(vapurr_);
        treasury = new GenesisTreasury(vapurr_, gV_, oliver_, msg.sender);

        // Always stand up seed books so POL V earmarks have a home (seedPol_ reserved).
        seedPol_;
        ExogenousSeedMarket ethM = new ExogenousSeedMarket(address(registry), EXO_TAG_ETH, vapurr_, eth_);
        ExogenousSeedMarket nvdaM = new ExogenousSeedMarket(address(registry), EXO_TAG_NVDA, vapurr_, nvda_);
        ExogenousSeedMarket amdM = new ExogenousSeedMarket(address(registry), EXO_TAG_AMD, vapurr_, amd_);
        ethSeed = address(ethM);
        nvdaSeed = address(nvdaM);
        amdSeed = address(amdM);
        registry.bindPool(EXO_TAG_ETH, ethSeed);
        registry.bindPool(EXO_TAG_NVDA, nvdaSeed);
        registry.bindPool(EXO_TAG_AMD, amdSeed);

        registry.setOwner(msg.sender);
        emit Bootstrapped(
            msg.sender,
            address(registry),
            address(devFund),
            address(browserStream),
            address(treasury),
            treasuryRemainder_
        );
    }

    function pullAmount() public view returns (uint256) {
        return DEV_FUND_AMOUNT + BROWSERSTREAM_V + POL_ETH_V + POL_NVDA_V + POL_AMD_V + HOUSE_SEED_V
            + treasuryRemainder;
    }

    /// Pull launch+DevFund from initiator and allocate locked buckets.
    function fundAndStart() external {
        if (msg.sender != initiator) revert NotInitiator();
        if (funded) revert AlreadyFunded();
        funded = true;

        IVapurrApprove v = IVapurrApprove(vapurr);
        uint256 pull = pullAmount();
        require(v.transferFrom(msg.sender, address(this), pull), "PULL");

        require(v.transfer(address(devFund), DEV_FUND_AMOUNT), "DEV");
        devFund.fund(DEV_FUND_AMOUNT);
        if (devFund.deposited() != DEV_FUND_AMOUNT) revert BadFund();
        devFund.startStream();
        devFund.setOwner(initiator);
        emit DevFundStarted(address(devFund), DEV_FUND_AMOUNT);

        require(v.approve(address(browserStream), BROWSERSTREAM_V), "ALLOW");
        browserStream.fund(BROWSERSTREAM_V);
        browserStream.startStream();
        browserStream.setOwner(initiator);

        require(v.approve(ethSeed, POL_ETH_V), "ALLOW");
        require(v.approve(nvdaSeed, POL_NVDA_V), "ALLOW");
        require(v.approve(amdSeed, POL_AMD_V), "ALLOW");
        ExogenousSeedMarket(ethSeed).fundV(POL_ETH_V);
        ExogenousSeedMarket(nvdaSeed).fundV(POL_NVDA_V);
        ExogenousSeedMarket(amdSeed).fundV(POL_AMD_V);
        ExogenousSeedMarket(ethSeed).setOwner(initiator);
        ExogenousSeedMarket(nvdaSeed).setOwner(initiator);
        ExogenousSeedMarket(amdSeed).setOwner(initiator);

        houseSeedHeld = HOUSE_SEED_V;

        if (treasuryRemainder > 0) {
            require(v.approve(address(treasury), treasuryRemainder), "ALLOW");
            treasury.fund(treasuryRemainder);
            treasury.lock();
            treasury.collateralizeOliver(type(uint256).max);
            treasury.setOwner(initiator);
        }

        emit Allocated(
            BROWSERSTREAM_V, POL_ETH_V, POL_NVDA_V, POL_AMD_V, HOUSE_SEED_V, treasuryRemainder
        );
    }

    /// House 20k is earmarked for wgV/$PUSD wrap — not AMM dump. Initiator claims later.
    function claimHouseSeed() external {
        if (msg.sender != initiator) revert NotInitiator();
        uint256 amt = houseSeedHeld;
        if (amt == 0) revert BadFund();
        houseSeedHeld = 0;
        require(IVapurrApprove(vapurr).transfer(initiator, amt), "HOUSE");
        emit HouseSeedClaimed(initiator, amt);
    }
}
