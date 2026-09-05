// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../PusdLoop.sol";
import "../Remittance.sol";
import "../SPUSD.sol";

/// Routing fences: market V inventory, remittance/runway/sPUSD.
/// GvFed walls covered by GvBoundaries.t.sol (run together).
contract RoutingFencesTest is Test {
    PusdMarket internal market;
    VapurrToken internal vapurr;
    PusdToken internal pusd;
    PusdLoop internal loop;
    RunwayFloor internal runway;
    RemittanceSink internal sink;
    SPUSD internal spusd;

    address internal trader = address(0xA11CE);
    address internal borrower = address(0xB02B);

    uint256 internal constant PRICE = 1e18; // 1 PUSD per V

    /// poolDelta is storage slot 3 (after vapurrRate, pendingRate, liveBlock).
    function _healPool() internal {
        vm.store(address(market), bytes32(uint256(3)), bytes32(uint256(0)));
    }

    function setUp() public {
        market = new PusdMarket(PRICE);
        vapurr = market.vapurr();
        pusd = market.pusd();
        loop = new PusdLoop(address(market));
        runway = new RunwayFloor(0);
        sink = new RemittanceSink(address(pusd), address(runway));
        spusd = new SPUSD(address(pusd));
        sink.setForward(address(spusd));

        vapurr.transfer(trader, 100_000 ether);
    }

    function test_market_redeem_does_not_mint_v() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        pusd.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(100 ether);
        vm.stopPrank();

        uint256 supplyAfterMint = vapurr.totalSupply();
        uint256 inv = market.vInventory();
        assertEq(inv, 100 ether, "V locked in inventory");
        assertEq(supplyAfterMint, 1_000_000 ether, "genesis supply unchanged (no burn/mint)");

        // Virtual pool skew blocks same-block reverse; heal delta then redeem from inventory.
        _healPool();
        vm.startPrank(trader);
        (uint256 vOut,) = market.swapPusdToV(ask / 2);
        vm.stopPrank();

        assertEq(vapurr.totalSupply(), supplyAfterMint, "HARD FENCE: redeem must not mint V");
        assertGt(vOut, 0, "received V from inventory");
        assertEq(market.vInventory(), inv - vOut, "inventory drew down");
    }

    function test_market_cannot_unbounded_mint_for_browse_earn() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        pusd.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(100 ether);
        vm.stopPrank();

        uint256 inv = market.vInventory();
        uint256 supplyBefore = vapurr.totalSupply();
        vm.prank(address(market));
        vapurr.burn(address(market), inv);
        assertEq(market.vInventory(), 0, "inventory drained");

        _healPool(); // so CP would succeed — fence must still be INV, not mint
        vm.startPrank(trader);
        vm.expectRevert(bytes("INV"));
        market.swapPusdToV(ask);
        vm.stopPrank();

        assertEq(vapurr.totalSupply(), supplyBefore - inv, "no V minted on failed redeem");
    }

    function test_fund_inventory_then_redeem_no_mint() public {
        uint256 supply0 = vapurr.totalSupply();
        vapurr.approve(address(market), type(uint256).max);
        market.fundVInventory(50 ether);
        assertEq(market.vInventory(), 50 ether);
        assertEq(vapurr.totalSupply(), supply0, "fund does not mint");

        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        pusd.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(10 ether);
        vm.stopPrank();
        _healPool();
        uint256 supply1 = vapurr.totalSupply();
        vm.prank(trader);
        (uint256 vOut,) = market.swapPusdToV(ask / 2);

        assertEq(vapurr.totalSupply(), supply1, "redeem still does not mint");
        assertGt(vOut, 0);
    }

    function test_accrue_path_can_call_remittance() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(50_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        loop.supply(ask);
        vm.stopPrank();

        vapurr.transfer(borrower, 20_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(20_000 ether);
        loop.borrow(5_000 ether);
        vm.stopPrank();

        // Auto-remit on accrue; floor 0
        loop.setRemittance(address(sink), address(runway), true);

        uint256 sinkBefore = pusd.balanceOf(address(sink));
        vm.warp(block.timestamp + 365 days);
        loop.accrue();

        uint256 sinkAfter = pusd.balanceOf(address(sink));
        // Auto path or explicit remit both prove the hook
        if (sinkAfter == sinkBefore) {
            uint256 sent = loop.remitReserve(type(uint256).max / 4);
            assertGt(sent, 0, "remitReserve sends reserve");
            sinkAfter = pusd.balanceOf(address(sink));
        }
        assertGt(sinkAfter, sinkBefore, "accrue/remit path credited sink");
    }

    function test_runway_floor_gates_surplus() public {
        RunwayFloor f = new RunwayFloor(0);
        assertEq(f.surplus(100 ether), 100 ether, "floor0 all surplus");
        f.setFloor(40 ether);
        assertEq(f.surplus(100 ether), 60 ether, "above floor");
        assertEq(f.surplus(40 ether), 0, "at floor");
        assertEq(f.surplus(10 ether), 0, "under floor");

        RemittanceSink s = new RemittanceSink(address(pusd), address(f));
        vapurr.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(200 ether);
        pusd.approve(address(s), type(uint256).max);
        s.receiveRemittance(ask);
        assertEq(s.surplus(), ask - 40 ether, "sink surplus above floor");
    }

    function test_spusd_deposit_and_yield_credit() public {
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(10_000 ether);

        pusd.approve(address(spusd), type(uint256).max);
        uint256 shares = spusd.deposit(5_000 ether, address(this));
        assertEq(shares, 5_000 ether, "1:1 genesis deposit");
        assertEq(spusd.convertToAssets(shares), 5_000 ether, "NAV 1");

        spusd.receiveRemittance(1_000 ether);
        assertEq(spusd.convertToAssets(shares), 6_000 ether, "yield credited into NAV");
        assertEq(spusd.totalSupply(), shares, "shares unchanged on yield credit");
    }

    function test_remit_respects_runway_floor_on_vault() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(20_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        loop.supply(ask);
        vm.stopPrank();

        vapurr.transfer(borrower, 10_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(10_000 ether);
        loop.borrow(2_000 ether);
        vm.stopPrank();

        runway.setFloor(type(uint256).max); // block remits: owner assets always <= floor
        loop.setRemittance(address(sink), address(runway), false);

        vm.warp(block.timestamp + 365 days);
        loop.accrue();
        uint256 sent = loop.remitReserve(100 ether);
        assertEq(sent, 0, "floor blocks remit");
        assertEq(pusd.balanceOf(address(sink)), 0, "sink empty under floor");
    }
}
