// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";
import {Create2Factory} from "../proxy/Create2Factory.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";

/// TESTNET 46630 prep ONLY.
/// Does not broadcast unless CONFIRM_TESTNET_DEPLOY=1 AND operator passes --broadcast.
/// Target vanity Lithe/market proxy: 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2
/// Prefer path A in VanityCreate2Hunt / TESTNET_ROLLOUT: STATUS deployer nonce-0 CREATE.
contract LitheVanityDeploy is Script {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;

    function run() external {
        bool confirm = vm.envOr("CONFIRM_TESTNET_DEPLOY", uint256(0)) == 1;
        address vapurr = vm.envOr("VAPURR", address(0));
        uint256 rate = vm.envOr("LITHE_RATE", uint256(1 ether));
        address owner_ = vm.envOr("LITHE_OWNER", msg.sender);
        bytes32 salt = vm.envOr("LITHE_SALT", bytes32(0));

        if (!confirm) {
            console2.log("DRY-RUN only - no broadcast. Set CONFIRM_TESTNET_DEPLOY=1 to enable.");
            _simulate(vapurr, rate, owner_, salt);
            return;
        }

        vm.startBroadcast();
        _simulate(vapurr, rate, owner_, salt);
        vm.stopBroadcast();
    }

    function _simulate(address vapurr, uint256 rate, address owner_, bytes32 salt) internal {
        Create2Factory factory = new Create2Factory();
        PusdMarketFedUpgradeable impl = new PusdMarketFedUpgradeable();

        bytes memory initData = abi.encodeCall(PusdMarketFedUpgradeable.initialize, (vapurr, rate, owner_));
        bytes memory proxyInitCode =
            abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(address(impl), initData));
        bytes32 initCodeHash = keccak256(proxyInitCode);

        address predicted = factory.computeAddress(salt, initCodeHash);
        console2.log("factory", address(factory));
        console2.log("impl", address(impl));
        console2.logBytes32(initCodeHash);
        console2.logBytes32(salt);
        console2.log("predicted", predicted);
        console2.log("vanity target", VANITY);

        if (predicted == VANITY) {
            address proxy = factory.deploy(salt, proxyInitCode);
            console2.log("DEPLOYED vanity proxy", proxy);
        } else {
            console2.log("SALT MISMATCH - mine salt or use nonce-0 CREATE path; not deploying proxy");
        }
    }
}
