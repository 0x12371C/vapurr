// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken as FedV} from "../GvFed.sol";
import {PusdToken} from "../PusdMarket.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";

/// v2 impl: append-only storage after parent __gap; preserves Lithe economic slots.
contract PusdMarketFedV2Harness is PusdMarketFedUpgradeable {
    uint256 public extraKnob;

    function litheVersion() external pure override returns (uint256) {
        return 2;
    }

    function setExtraKnob(uint256 v) external onlyOwner {
        extraKnob = v;
    }
}

contract PusdMarketFedProxyTest is Test {
    uint256 internal constant PRICE = 1 ether;

    FedV internal canonical;
    PusdMarketFedUpgradeable internal implV1;
    PusdMarketFedUpgradeable internal market; // proxy-typed as v1 ABI
    address internal proxyAddr;
    address internal trader = address(0xA11CE);

    function setUp() public {
        canonical = new FedV();
        canonical.mint(address(this), 200_000 ether);
        canonical.mint(trader, 50_000 ether);

        implV1 = new PusdMarketFedUpgradeable();
        bytes memory initData =
            abi.encodeCall(PusdMarketFedUpgradeable.initialize, (address(canonical), PRICE, address(this)));
        proxyAddr = address(new ERC1967Proxy(address(implV1), initData));
        market = PusdMarketFedUpgradeable(proxyAddr);

        canonical.approve(proxyAddr, type(uint256).max);
        market.fundVInventory(100_000 ether);
    }

    function test_proxy_initialize_and_swap_inventory_safe() public {
        assertEq(market.litheVersion(), 1);
        assertEq(address(market.vapurr()), address(canonical));
        assertEq(market.owner(), address(this));
        assertEq(market.vInventory(), 100_000 ether);
        assertEq(market.vapurrRate(), PRICE);

        uint256 supply0 = canonical.totalSupply();
        vm.startPrank(trader);
        canonical.approve(proxyAddr, type(uint256).max);
        (uint256 pusdOut,) = market.swapVToPusd(10_000 ether);
        vm.stopPrank();
        assertGt(pusdOut, 0);
        assertEq(canonical.totalSupply(), supply0, "proxy Lithe never mints V");
    }

    function test_upgrade_v1_to_v2_preserves_storage() public {
        PusdToken pusd = market.pusd();
        uint256 inv0 = market.vInventory();
        uint256 rate0 = market.vapurrRate();
        address owner0 = market.owner();

        // Mutate economic state pre-upgrade.
        vm.startPrank(trader);
        canonical.approve(proxyAddr, type(uint256).max);
        market.swapVToPusd(5_000 ether);
        vm.stopPrank();
        uint256 inv1 = market.vInventory();
        uint256 yield1 = market.yieldReserve();
        assertGt(inv1, inv0);

        PusdMarketFedV2Harness implV2 = new PusdMarketFedV2Harness();
        market.upgradeToAndCall(address(implV2), "");

        PusdMarketFedV2Harness v2 = PusdMarketFedV2Harness(proxyAddr);
        assertEq(v2.litheVersion(), 2);
        assertEq(address(v2.pusd()), address(pusd), "pusd address preserved");
        assertEq(address(v2.vapurr()), address(canonical), "vapurr preserved");
        assertEq(v2.owner(), owner0, "owner preserved");
        assertEq(v2.vapurrRate(), rate0, "rate preserved");
        assertEq(v2.vInventory(), inv1, "inventory preserved");
        assertEq(v2.yieldReserve(), yield1, "yield reserve preserved");

        v2.setExtraKnob(42);
        assertEq(v2.extraKnob(), 42);

        // Still swaps after upgrade.
        uint256 supply0 = canonical.totalSupply();
        vm.startPrank(trader);
        (uint256 out,) = v2.swapVToPusd(1_000 ether);
        vm.stopPrank();
        assertGt(out, 0);
        assertEq(canonical.totalSupply(), supply0);
    }

    function test_upgrade_reverts_for_non_owner() public {
        PusdMarketFedV2Harness implV2 = new PusdMarketFedV2Harness();
        vm.prank(trader);
        vm.expectRevert(bytes("OWN"));
        market.upgradeToAndCall(address(implV2), "");
    }

    function test_cannot_reinitialize() public {
        vm.expectRevert();
        market.initialize(address(canonical), PRICE, address(this));
    }

    function test_desk_snapshot_abi_width() public view {
        (bool ok, bytes memory raw) = proxyAddr.staticcall(abi.encodeWithSignature("snapshot(address)", trader));
        assertTrue(ok);
        assertEq(raw.length, 12 * 32, "12-word market snapshot");
    }
}
