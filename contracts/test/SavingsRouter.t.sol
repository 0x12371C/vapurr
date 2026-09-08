// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../SavingsRouter.sol";
import "./SpusdCd.t.sol";

contract SavingsRouterTest is Test {
    MockCdPusd asset;
    RunwayFloor runway;
    RemittanceSink sink;
    SPUSD liquid;
    SpusdCd cd;
    SavingsRouter router;
    address saver = address(0x5A);

    function setUp() public {
        asset = new MockCdPusd();
        runway = new RunwayFloor(100 ether);
        sink = new RemittanceSink(address(asset), address(runway));
        liquid = new SPUSD(address(asset));
        cd = new SpusdCd(address(asset), 500, 200, 30 days);
        router = new SavingsRouter(address(sink), address(liquid), address(cd));
        sink.setForward(address(router));
        asset.mint(address(this), 1_000_000 ether);
        asset.approve(address(sink), type(uint256).max);
        asset.approve(address(liquid), type(uint256).max);
        liquid.deposit(1_000 ether, saver);
    }

    function test_enabled_by_default() public view {
        assertTrue(router.enabled());
        assertEq(router.cdBps(), 2_500);
    }

    function test_owner_disable_killswitch_keeps_cash_at_sink() public {
        router.setAllocation(false, 0);
        sink.receiveRemittance(1_100 ether);
        vm.expectRevert(bytes("DISABLED"));
        sink.forwardSurplus(0);
        assertEq(sink.accountedRfv(), 1_100 ether);
        assertEq(router.totalReceived(), 0);
    }

    function test_splits_only_above_floor_without_minting_shares() public {
        sink.receiveRemittance(1_100 ether);
        router.setAllocation(true, 2500);
        uint256 shares = liquid.totalSupply();
        sink.forwardSurplus(0);
        assertEq(sink.accountedRfv(), 100 ether);
        assertEq(liquid.totalAssets(), 1_750 ether);
        assertEq(liquid.totalSupply(), shares);
        assertEq(cd.surplus(), 250 ether);
        assertEq(cd.totalPrincipal(), 0);
        assertEq(router.totalReceived(), 1_000 ether);
        assertEq(router.totalLiquid(), 750 ether);
        assertEq(router.totalCd(), 250 ether);
        assertEq(asset.balanceOf(address(router)), 0);
        assertEq(asset.allowance(address(router), address(liquid)), 0);
        assertEq(asset.allowance(address(router), address(cd)), 0);
        vm.expectRevert(bytes("FLOOR"));
        sink.forwardSurplus(1);
    }

    function test_only_sink_can_allocate_and_only_owner_sets_split() public {
        vm.prank(saver);
        vm.expectRevert(bytes("SINK"));
        router.receiveRemittance(100 ether);
        vm.prank(saver);
        vm.expectRevert(bytes("OWN"));
        router.setAllocation(true, 5000);
        vm.expectRevert(bytes("BPS"));
        router.setAllocation(true, 10001);
    }

    function test_empty_liquid_vault_cannot_receive_first_depositor_windfall() public {
        SPUSD empty = new SPUSD(address(asset));
        SavingsRouter emptyRouter = new SavingsRouter(address(sink), address(empty), address(cd));
        sink.setForward(address(emptyRouter));
        emptyRouter.setAllocation(true, 2500);
        sink.receiveRemittance(1_100 ether);
        vm.expectRevert(bytes("EMPTY"));
        sink.forwardSurplus(0);
        assertEq(sink.accountedRfv(), 1_100 ether);
        assertEq(asset.balanceOf(address(emptyRouter)), 0);
        assertEq(cd.surplus(), 0);
        // All-CD allocations can pre-fund coupons before any savings deposit.
        emptyRouter.setAllocation(true, 10000);
        sink.forwardSurplus(0);
        assertEq(cd.surplus(), 1_000 ether);
    }

    function test_failed_second_leg_rolls_back_first_leg_and_counters() public {
        router.setAllocation(true, 5000);
        sink.receiveRemittance(1_100 ether);
        vm.mockCallRevert(address(cd), abi.encodeWithSelector(IRemittance.receiveRemittance.selector), bytes("FAIL"));
        vm.expectRevert();
        sink.forwardSurplus(0);
        assertEq(sink.accountedRfv(), 1_100 ether);
        assertEq(liquid.totalAssets(), 1_000 ether);
        assertEq(router.totalReceived(), 0);
        assertEq(router.totalLiquid(), 0);
        assertEq(asset.balanceOf(address(router)), 0);
        assertEq(asset.allowance(address(sink), address(router)), 0);
    }

    function test_rejects_mismatched_underlying() public {
        MockCdPusd other = new MockCdPusd();
        SpusdCd wrong = new SpusdCd(address(other), 500, 200, 30 days);
        vm.expectRevert(bytes("ASSET"));
        new SavingsRouter(address(sink), address(liquid), address(wrong));
    }

    function testFuzz_allocation_conserves_cash_and_floor(uint256 amount, uint256 bps) public {
        amount = bound(amount, 1 ether, 100_000 ether);
        bps = bound(bps, 0, 10_000);
        router.setAllocation(true, bps);
        sink.receiveRemittance(amount + 100 ether);
        sink.forwardSurplus(0);
        assertEq(sink.accountedRfv(), 100 ether);
        assertEq(router.totalLiquid() + router.totalCd(), amount);
        assertEq(liquid.totalAssets() - 1_000 ether + cd.surplus(), amount);
        assertEq(asset.balanceOf(address(router)), 0);
    }
}
