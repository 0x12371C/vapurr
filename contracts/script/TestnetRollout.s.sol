// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {VapurrToken, RebasePolicy, gVAPURR} from "../GvFed.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";
import {PusdLoop} from "../PusdLoop.sol";

/// Dry-runable gen-5 testnet rollout skeleton (chain 46630).
///
/// SAFETY: does NOT broadcast unless CONFIRM_TESTNET_DEPLOY=1 is set in the env.
/// Default path is simulation / address planning only.
///
/// Vanity target (STATUS MAINNET_MARKET_VANITY - reuse as Lithe proxy target for staged rollout):
///   0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2
/// STATUS deployer: 0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5
/// Verified: vanity == CREATE(deployer, nonce=0).
///
/// Preferred vanity land:
///   1) Deploy impl from a different key (or CREATE2).
///   2) From STATUS deployer at nonce 0: CREATE ERC1967Proxy(impl, init) - exact vanity.
/// CREATE2 salt hunt: see VanityCreate2Hunt.s.sol / docs/econ/TESTNET_ROLLOUT.md.
contract TestnetRollout is Script {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;
    address constant STATUS_DEPLOYER = 0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5;

    function run() external {
        bool confirm = vm.envOr("CONFIRM_TESTNET_DEPLOY", uint256(0)) == 1;
        uint256 rate = vm.envOr("LITHE_RATE_WAD", uint256(1 ether));
        address owner_ = vm.envOr("ROLLOUT_OWNER", address(0));

        console2.log("chain planned: 46630 (testnet)");
        console2.log("CONFIRM_TESTNET_DEPLOY", confirm ? uint256(1) : uint256(0));
        console2.log("vanity target:", VANITY);
        console2.log("STATUS deployer:", STATUS_DEPLOYER);

        if (!confirm) {
            console2.log("DRY-RUN only - no broadcast. Set CONFIRM_TESTNET_DEPLOY=1 to enable live deploy.");
            _plan(rate, owner_ == address(0) ? msg.sender : owner_);
            return;
        }

        // Live path still requires explicit broadcast flag from the operator.
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(pk);
        if (owner_ == address(0)) owner_ = deployer;

        vm.startBroadcast(pk);
        _deployCore(rate, owner_);
        vm.stopBroadcast();
    }

    function _plan(uint256 rate, address owner_) internal view {
        console2.log("plan owner:", owner_);
        console2.log("plan lithe rate wad:", rate);
        console2.log("ordered steps (see docs/econ/TESTNET_ROLLOUT.md):");
        console2.log("  1 Fed V + RebasePolicy + gV (dynamic 1-9%)");
        console2.log("  2 Lithe impl + ERC1967Proxy (UUPS) - prefer vanity");
        console2.log("  3 Oliver (PusdLoop) behind market");
        console2.log("  4 BondMarket (USDG BondAssetTag only)");
        console2.log("  5 Remittance / SavingsRouter wiring");
        console2.log("  6 LaunchBootstrap: DevFund 200k -> Oliver collateral + V/ETH+V/NVDA+V/AMD");
        console2.log("  7 House / wgV follow-up (not in factory)");
        console2.log("HONEST: gen-4 remains live on 46630 until approved cutover.");
    }

    function _deployCore(uint256 rate, address owner_) internal returns (address proxy) {
        VapurrToken v = new VapurrToken();
        RebasePolicy policy = new RebasePolicy();
        gVAPURR gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));

        PusdMarketFedUpgradeable impl = new PusdMarketFedUpgradeable();
        bytes memory initData = abi.encodeCall(PusdMarketFedUpgradeable.initialize, (address(v), rate, owner_));
        proxy = address(new ERC1967Proxy(address(impl), initData));

        PusdLoop oliver = new PusdLoop(proxy);
        oliver.setOwner(owner_);

        // BondMarket constructor args are environment-specific - wire post-plan.
        // Remittance / DevFund / pairs: LaunchBootstrap companion (parallel track).

        policy.setOwner(owner_);
        // Leave V minter with deployer until genesis mint + DevFund allocation, then setMinter(gV).

        console2.log("deployed V", address(v));
        console2.log("deployed policy", address(policy));
        console2.log("deployed gV", address(gV));
        console2.log("deployed lithe impl", address(impl));
        console2.log("deployed lithe proxy", proxy);
        console2.log("deployed oliver", address(oliver));
        if (proxy == VANITY) {
            console2.log("vanity MATCH");
        } else {
            console2.log("vanity MISS - use STATUS deployer nonce-0 CREATE or CREATE2 salt hunt");
        }
    }
}


