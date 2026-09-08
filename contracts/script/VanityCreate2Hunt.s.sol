// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";

/// CREATE2 salt hunt notes + helper for landing ERC1967Proxy at MAINNET_MARKET_VANITY.
///
/// FACT (verified off-chain against STATUS):
///   VANITY 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2
///     == CREATE(0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5, nonce=0)
///
/// Paths:
/// A) Preferred (no hunt): deploy impl elsewhere; from STATUS deployer send nonce-0
///    CREATE of ERC1967Proxy(impl, initData) - exact vanity.
/// B) CREATE2: find salt s such that
///    keccak256(0xff ++ deployer ++ s ++ keccak256(proxyCreationCode ++ abi.encode(impl, data)))[12:]
///    == VANITY. Requires fixed impl address + init calldata before the hunt.
///
/// This script only computes / searches locally (no broadcast).
contract VanityCreate2Hunt is Script {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;
    address constant STATUS_DEPLOYER = 0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5;

    function run() external view {
        address impl = vm.envOr("LITHE_IMPL", address(0));
        address vapurr = vm.envOr("CANONICAL_V", address(0));
        uint256 rate = vm.envOr("LITHE_RATE_WAD", uint256(1 ether));
        address owner_ = vm.envOr("ROLLOUT_OWNER", STATUS_DEPLOYER);
        uint256 maxIters = vm.envOr("SALT_ITERS", uint256(500_000));

        console2.log("VANITY", VANITY);
        console2.log("STATUS_DEPLOYER", STATUS_DEPLOYER);
        console2.log("path A: CREATE proxy at deployer nonce 0 (no salt)");
        console2.log("path B: CREATE2 hunt needs LITHE_IMPL + CANONICAL_V");

        if (impl == address(0) || vapurr == address(0)) {
            console2.log("missing LITHE_IMPL or CANONICAL_V - notes only, no hunt");
            return;
        }

        bytes memory initData =
            abi.encodeWithSignature("initialize(address,uint256,address)", vapurr, rate, owner_);
        bytes memory creation = abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(impl, initData));
        bytes32 initCodeHash = keccak256(creation);
        console2.logBytes32(initCodeHash);

        address deployer = vm.envOr("CREATE2_DEPLOYER", STATUS_DEPLOYER);
        for (uint256 i = 0; i < maxIters; i++) {
            bytes32 salt = bytes32(i);
            address predicted = vm.computeCreate2Address(salt, initCodeHash, deployer);
            if (predicted == VANITY) {
                console2.log("FOUND salt index", i);
                console2.logBytes32(salt);
                return;
            }
        }
        console2.log("no salt in range - raise SALT_ITERS or use path A (nonce-0 CREATE)");
    }
}

