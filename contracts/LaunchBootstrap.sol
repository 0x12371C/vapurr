// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {ExogenousPairRegistry, ExogenousSeedMarket, EXO_TAG_ETH, EXO_TAG_NVDA, EXO_TAG_AMD} from "./ExogenousPairRegistry.sol";
import {DevFundStream} from "./DevFundStream.sol";

interface IVapurrApprove {
    function approve(address spender, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function totalSupply() external view returns (uint256);
}

/// Companion to CanonicalLitheFactory cutover.
/// Constructor stands up registry + DevFundStream (+ optional POL seeds).
/// `fundAndStart()` pulls genesis 200k from initiator into DevFund (approve this
/// contract after deploy ? avoids CREATE-address nonce footguns).
/// DevFund V auto-locks as Oliver collateral; recipient draws $PUSD only.
/// Does NOT deploy V/USDG or PUSD/USDG. No silent chain deploy.
contract LaunchBootstrap {
    uint256 public constant DEV_FUND_AMOUNT = 200_000 ether;

    ExogenousPairRegistry public immutable registry;
    DevFundStream public immutable devFund;
    address public immutable vapurr;
    address public immutable oliver;
    address public immutable initiator;

    address public ethSeed;
    address public nvdaSeed;
    address public amdSeed;
    bool public funded;

    event Bootstrapped(
        address indexed initiator,
        address registry,
        address devFund,
        address oliver,
        address eth,
        address nvda,
        address amd,
        address ethSeed,
        address nvdaSeed,
        address amdSeed
    );
    event DevFundStarted(address indexed stream, uint256 amount);

    error ZeroAddr();
    error BadFund();
    error NotInitiator();
    error AlreadyFunded();

    constructor(
        address vapurr_,
        address oliver_,
        address recipient_,
        address usdg_,
        address pusd_,
        address eth_,
        address nvda_,
        address amd_,
        bool seedPol_
    ) {
        if (vapurr_ == address(0) || oliver_ == address(0) || recipient_ == address(0)) revert ZeroAddr();
        if (eth_ == address(0) || nvda_ == address(0) || amd_ == address(0)) revert ZeroAddr();

        vapurr = vapurr_;
        oliver = oliver_;
        initiator = msg.sender;

        registry = new ExogenousPairRegistry(vapurr_, usdg_, pusd_);
        registry.registerPair(EXO_TAG_ETH, eth_, true);
        registry.registerPair(EXO_TAG_NVDA, nvda_, true);
        registry.registerPair(EXO_TAG_AMD, amd_, true);

        devFund = new DevFundStream(vapurr_, oliver_, recipient_);

        if (seedPol_) {
            ExogenousSeedMarket ethM = new ExogenousSeedMarket(address(registry), EXO_TAG_ETH, vapurr_, eth_);
            ExogenousSeedMarket nvdaM = new ExogenousSeedMarket(address(registry), EXO_TAG_NVDA, vapurr_, nvda_);
            ExogenousSeedMarket amdM = new ExogenousSeedMarket(address(registry), EXO_TAG_AMD, vapurr_, amd_);
            ethSeed = address(ethM);
            nvdaSeed = address(nvdaM);
            amdSeed = address(amdM);
            registry.bindPool(EXO_TAG_ETH, ethSeed);
            registry.bindPool(EXO_TAG_NVDA, nvdaSeed);
            registry.bindPool(EXO_TAG_AMD, amdSeed);
            ethM.setOwner(msg.sender);
            nvdaM.setOwner(msg.sender);
            amdM.setOwner(msg.sender);
        }

        registry.setOwner(msg.sender);
        // Keep stream owned by bootstrap until fundAndStart hands owner to initiator.
        emit Bootstrapped(
            msg.sender,
            address(registry),
            address(devFund),
            oliver_,
            eth_,
            nvda_,
            amd_,
            ethSeed,
            nvdaSeed,
            amdSeed
        );
    }

    /// Pull 200k from initiator into DevFundStream, start lockup, hand ownership over.
    function fundAndStart() external {
        if (msg.sender != initiator) revert NotInitiator();
        if (funded) revert AlreadyFunded();
        funded = true;

        IVapurrApprove v = IVapurrApprove(vapurr);
        require(v.transferFrom(msg.sender, address(devFund), DEV_FUND_AMOUNT), "PULL");
        devFund.fund(DEV_FUND_AMOUNT);
        if (devFund.deposited() != DEV_FUND_AMOUNT) revert BadFund();
        devFund.startStream();
        devFund.setOwner(initiator);
        emit DevFundStarted(address(devFund), DEV_FUND_AMOUNT);
    }
}
