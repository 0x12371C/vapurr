// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {VapurrToken, RebasePolicy, gVAPURR, wgVAPURR} from "../GvFed.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";
import {HousePairConfig, HousePairFactory} from "../HousePairConfig.sol";

interface ILithePusdView {
    function pusd() external view returns (address);
    function vapurr() external view returns (address);
    function litheVersion() external view returns (uint256);
}

/// Post-cutover House follow-up: deploy wgVAPURR + HousePairConfig for **wgV / $PUSD**.
///
/// NOT part of TestnetRollout core factory. Run **after** gen-5 Lithe proxy + Fed V + gV exist.
/// See docs/econ/HOUSE_PAIR.md, WGV_HOUSE.md, TESTNET_ROLLOUT.md section 9.
///
/// SAFETY:
/// - Default = local dry-run simulation (no broadcast).
/// - Live broadcast only if CONFIRM_TESTNET_DEPLOY=1 **or** CONFIRM_HOUSE_FOLLOWUP=1.
/// - Does NOT move ETH / fund vanity deployer.
/// - Does NOT deploy HouseLp / HouseSwap (need Uni v4 PositionManager / PoolManager / Permit2).
contract TestnetHouseFollowupDeploy {
    VapurrToken public v;
    RebasePolicy public policy;
    gVAPURR public gV;
    wgVAPURR public wgV;
    PusdMarketFedUpgradeable public impl;
    address public litheProxy;
    address public pusd;
    HousePairConfig public pairConfig;
    HousePairFactory public pairFactory;
    bool public usedLocalCoreStack;

    /// When GV_/PUSD_ provided: wire against existing core (post-cutover).
    /// When both zero: stand up a local Fed V + gV + Lithe proxy (dry-run compose only).
    function execute(address owner_, address gV_, address pusd_, address litheProxy_, uint256 litheRate)
        external
    {
        if (gV_ != address(0) || pusd_ != address(0) || litheProxy_ != address(0)) {
            _wireExisting(gV_, pusd_, litheProxy_);
        } else {
            _composeLocalCore(owner_, litheRate);
        }

        wgV = new wgVAPURR(address(gV));
        pairConfig = new HousePairConfig(address(wgV), pusd, address(gV));
        pairFactory = new HousePairFactory(address(pairConfig));

        // Prove gate: wgV/$PUSD accepted; raw gV would revert (covered in HousePairGuard.t.sol).
        bytes32 poolId = pairFactory.validateAndMark(address(wgV), pusd);
        require(poolId == keccak256(abi.encode(address(wgV), pusd)), "POOL");
        pairConfig.requireHouseEquity(address(wgV));
        pairConfig.requireHousePair(address(wgV), pusd);
    }

    function _wireExisting(address gV_, address pusd_, address litheProxy_) internal {
        require(gV_ != address(0), "GV");
        address cash = pusd_;
        if (cash == address(0)) {
            require(litheProxy_ != address(0), "PUSD_OR_LITHE");
            cash = ILithePusdView(litheProxy_).pusd();
        }
        require(cash != address(0), "PUSD");
        if (litheProxy_ != address(0)) {
            require(ILithePusdView(litheProxy_).litheVersion() == 1, "LITHE");
            // Prefer Lithe-reported PUSD when proxy set (sanity vs drifted env).
            address lithePusd = ILithePusdView(litheProxy_).pusd();
            if (pusd_ != address(0)) require(pusd_ == lithePusd, "PUSD_MISMATCH");
            cash = lithePusd;
            litheProxy = litheProxy_;
        }
        gV = gVAPURR(gV_);
        pusd = cash;
        usedLocalCoreStack = false;
    }

    function _composeLocalCore(address owner_, uint256 litheRate) internal {
        // Dry-run / prep only - not live gen-5 addresses.
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        policy.setOwner(owner_);

        impl = new PusdMarketFedUpgradeable();
        bytes memory initData =
            abi.encodeCall(PusdMarketFedUpgradeable.initialize, (address(v), litheRate, owner_));
        litheProxy = address(new ERC1967Proxy(address(impl), initData));
        pusd = address(PusdMarketFedUpgradeable(litheProxy).pusd());
        usedLocalCoreStack = true;
    }
}

/// Ordered House / wgV follow-up for 46630 after core cutover.
contract TestnetHouseFollowup is Script {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;

    function run() external {
        bool confirmCore = vm.envOr("CONFIRM_TESTNET_DEPLOY", uint256(0)) == 1;
        bool confirmHouse = vm.envOr("CONFIRM_HOUSE_FOLLOWUP", uint256(0)) == 1;
        bool confirm = confirmCore || confirmHouse;

        address owner_ = vm.envOr("ROLLOUT_OWNER", address(0));
        address gVIn = vm.envOr("GV", address(0));
        address pusdIn = vm.envOr("PUSD", address(0));
        address litheIn = vm.envOr("LITHE_PROXY", address(0));
        // Alias: HOUSE_LITHE_PROXY if LITHE_PROXY unset
        if (litheIn == address(0)) litheIn = vm.envOr("HOUSE_LITHE_PROXY", address(0));
        uint256 litheRate = vm.envOr("LITHE_RATE_WAD", uint256(1 ether));

        console2.log("TestnetHouseFollowup - post-cutover wgV + HousePairConfig");
        console2.log("chain planned: 46630 (testnet)");
        console2.log("CONFIRM_TESTNET_DEPLOY", confirmCore ? uint256(1) : uint256(0));
        console2.log("CONFIRM_HOUSE_FOLLOWUP", confirmHouse ? uint256(1) : uint256(0));
        console2.log("vanity Lithe target (core):", VANITY);

        _plan(gVIn, pusdIn, litheIn);

        if (!confirm) {
            console2.log("DRY-RUN only - no broadcast.");
            console2.log("Set CONFIRM_HOUSE_FOLLOWUP=1 (or CONFIRM_TESTNET_DEPLOY=1) after core cutover to broadcast.");
            if (owner_ == address(0)) owner_ = msg.sender;
            // Dry-run: if no post-cutover addrs, compose local Lithe+gV; else wire against env.
            TestnetHouseFollowupDeploy helper = new TestnetHouseFollowupDeploy();
            helper.execute(owner_, gVIn, pusdIn, litheIn, litheRate);
            _log(helper);
            return;
        }

        // Live: require existing gen-5 gV + PUSD (or Lithe proxy). Never invent addresses.
        require(gVIn != address(0), "LIVE_NEED_GV");
        require(pusdIn != address(0) || litheIn != address(0), "LIVE_NEED_PUSD_OR_LITHE");
        uint256 pk = vm.envUint("PRIVATE_KEY");
        if (owner_ == address(0)) owner_ = vm.addr(pk);

        vm.startBroadcast(pk);
        TestnetHouseFollowupDeploy live = new TestnetHouseFollowupDeploy();
        live.execute(owner_, gVIn, pusdIn, litheIn, litheRate);
        vm.stopBroadcast();
        _log(live);
    }

    function _plan(address gVIn, address pusdIn, address litheIn) internal pure {
        console2.log("ordered steps (AFTER core TestnetRollout / CutoverDeploy):");
        console2.log("  1 Record gen-5 Lithe proxy + Fed V + gV + PUSD from STATUS / rollout log");
        console2.log("  2 Deploy wgVAPURR(gV) - wrap path SoT for House equity");
        console2.log("  3 Deploy HousePairConfig(wgV, pusd, gV)");
        console2.log("  4 Deploy HousePairFactory(config); validateAndMark(wgV, pusd)");
        console2.log("  5 [STILL OPEN] HouseLp / HouseSwap need Uni v4 POSM + Permit2 + PoolManager");
        console2.log("  6 [STILL OPEN] Seed wgV inventory (stake V->gV, wrap->wgV) - never raw V/gV");
        console2.log("  7 [STILL OPEN] HouseFeeRemit / HouseUniSkim + Rust house_deploy pairConfig ABI");
        console2.log("  8 Clear gen-4 house/pair_config from local cutover book");
        console2.log("env (post-cutover live): GV, PUSD and/or LITHE_PROXY, PRIVATE_KEY, ROLLOUT_OWNER?");
        console2.log("env (optional): LITHE_RATE_WAD (dry-run local Lithe only), HOUSE_LITHE_PROXY alias");
        console2.log("input GV:", gVIn);
        console2.log("input PUSD:", pusdIn);
        console2.log("input LITHE_PROXY:", litheIn);
        console2.log("HONEST: core cutover not blocked on this; House book stays follow-up.");
    }

    function _log(TestnetHouseFollowupDeploy h) internal view {
        if (h.usedLocalCoreStack()) {
            console2.log("dry-run local core: V", address(h.v()));
            console2.log("dry-run local core: policy", address(h.policy()));
            console2.log("dry-run local core: lithe impl", address(h.impl()));
            console2.log("dry-run local core: lithe proxy", h.litheProxy());
            console2.log("(local compose only - not live gen-5 addresses)");
        } else {
            console2.log("wired against existing Lithe proxy", h.litheProxy());
        }
        console2.log("deployed gV", address(h.gV()));
        console2.log("deployed PUSD (cash leg)", h.pusd());
        console2.log("deployed wgVAPURR", address(h.wgV()));
        console2.log("deployed HousePairConfig", address(h.pairConfig()));
        console2.log("deployed HousePairFactory", address(h.pairFactory()));
        console2.log("pair equity wgV", h.pairConfig().wgV());
        console2.log("pair cash pusd", h.pairConfig().pusd());
        console2.log("pair banned raw gV", h.pairConfig().gV());
        console2.log("NOT deployed: HouseLp / HouseSwap / Uni v4 seed (need POSM/PM/Permit2)");
        console2.log("NOT done: fund vanity deployer / CONFIRM core / enable savings / UI book");
    }
}