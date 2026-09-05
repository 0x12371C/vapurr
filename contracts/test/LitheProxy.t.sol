// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken} from "../GvFed.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";
import {Create2Factory} from "../proxy/Create2Factory.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";

contract LitheProxyTest is Test {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;

    function test_uups_proxy_initializes_and_funds_inventory() public {
        VapurrToken v = new VapurrToken();
        v.mint(address(this), 100_000 ether);

        PusdMarketFedUpgradeable impl = new PusdMarketFedUpgradeable();
        bytes memory initData =
            abi.encodeCall(PusdMarketFedUpgradeable.initialize, (address(v), uint256(1 ether), address(this)));
        ERC1967Proxy proxy = new ERC1967Proxy(address(impl), initData);
        PusdMarketFedUpgradeable lithe = PusdMarketFedUpgradeable(address(proxy));

        assertEq(address(lithe.vapurr()), address(v));
        assertTrue(address(lithe.pusd()) != address(0));
        assertEq(lithe.vapurrRate(), 1 ether);
        assertEq(lithe.owner(), address(this));
        assertEq(lithe.vInventory(), 0);

        v.approve(address(lithe), 10_000 ether);
        lithe.fundVInventory(10_000 ether);
        assertEq(lithe.vInventory(), 10_000 ether);
        assertEq(v.totalSupply(), 100_000 ether, "fund does not mint");
    }

    function test_create2_predict_matches_deploy() public {
        Create2Factory factory = new Create2Factory();
        PusdMarketFedUpgradeable impl = new PusdMarketFedUpgradeable();
        VapurrToken v = new VapurrToken();
        bytes memory initData =
            abi.encodeCall(PusdMarketFedUpgradeable.initialize, (address(v), uint256(1 ether), address(this)));
        bytes memory proxyInitCode =
            abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(address(impl), initData));
        bytes32 salt = bytes32(uint256(0x46630));
        address predicted = factory.computeAddress(salt, keccak256(proxyInitCode));
        address deployed = factory.deploy(salt, proxyInitCode);
        assertEq(deployed, predicted);
        assertTrue(VANITY != address(0));
    }

    function test_implementation_initializer_disabled() public {
        PusdMarketFedUpgradeable impl = new PusdMarketFedUpgradeable();
        // Implementation should reject a second/direct initialize (Initializable guard).
        vm.expectRevert();
        impl.initialize(address(0x1), 1 ether, address(this));
    }
}
