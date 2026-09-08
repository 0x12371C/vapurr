// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken} from "../GvFed.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";
import {Create2Factory} from "../proxy/Create2Factory.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";

contract LitheProxyTest is Test {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;

    function test_uups_proxy_initializes_and_seigniorage_swap() public {
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

        v.setMarketMinter(address(lithe));
        uint256 supply0 = v.totalSupply();
        lithe.swapVToPusd(10_000 ether);
        assertEq(v.totalSupply(), supply0 - 10_000 ether, "expand burns V");
        assertEq(lithe.vInventory(), 0, "no inventory under seigniorage");

        vm.expectRevert(bytes("SEIGNIORAGE"));
        lithe.fundVInventory(1 ether);
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
        vm.expectRevert();
        impl.initialize(address(0x1), 1 ether, address(this));
    }
}
