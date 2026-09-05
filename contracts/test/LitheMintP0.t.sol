// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../Remittance.sol";
import "../SPUSD.sol";

/// P0: Lithe single-count surplus + computeSwap redeem after one-sided flow.
contract LitheMintP0Test is Test {
    PusdMarket internal market;
    VapurrToken internal vapurr;
    PusdToken internal pusd;
    RemittanceSink internal sink;
    RunwayFloor internal runway;
    SPUSD internal spusd;

    address internal trader = address(0xA11CE);
    uint256 internal constant PRICE = 1e18;

    function setUp() public {
        market = new PusdMarket(PRICE);
        vapurr = market.vapurr();
        pusd = market.pusd();
        runway = new RunwayFloor(0);
        sink = new RemittanceSink(address(pusd), address(runway));
        spusd = new SPUSD(address(pusd));
        sink.setForward(address(spusd));
        vapurr.transfer(trader, 200_000 ether);
    }

    function test_lithe_drip_burns_inventory_no_double_supply() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(50_000 ether);
        vm.stopPrank();

        uint256 reserve0 = market.yieldReserve();
        uint256 cash0 = pusd.balanceOf(address(market));
        uint256 supply0 = pusd.totalSupply();
        assertGt(reserve0, 0, "spread funded reserve");
        assertEq(cash0, reserve0, "fee inventory matches yieldReserve");

        vm.warp(block.timestamp + 30 days);
        market.accrue();

        uint256 reserve1 = market.yieldReserve();
        uint256 cash1 = pusd.balanceOf(address(market));
        uint256 supply1 = pusd.totalSupply();

        assertLt(reserve1, reserve0, "reserve consumed by drip");
        assertLt(cash1, cash0, "inventory burned with drip");
        assertApproxEqAbs(cash1, reserve1, 10, "remaining cash ~ yieldReserve");
        // Burn-all fee ? drip ? remint remainder: net supply unchanged (no double-count).
        assertApproxEqAbs(supply1, supply0, 1e12, "no double-count supply expansion");
    }

    function test_lithe_remit_after_drip_does_not_pay_twice() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(80_000 ether);
        vm.stopPrank();

        uint256 feeMinted = market.yieldReserve();
        assertGt(feeMinted, 0);

        market.setRemittance(address(sink), address(runway), false);
        vm.warp(block.timestamp + 60 days);
        market.accrue();

        uint256 remaining = market.yieldReserve();
        uint256 dripped = feeMinted - remaining;
        assertGt(dripped, 0, "some fee dripped to holders");

        uint256 sent = market.remitSurplus(0);
        assertApproxEqAbs(sent, remaining, 1, "remit sends leftover reserve only");
        // Total paid out (drip redistribution + remit) equals original fee inventory.
        assertEq(market.yieldReserve(), 0, "reserve emptied");
        assertEq(pusd.balanceOf(address(market)), 0, "no leftover fee inventory");
        assertApproxEqAbs(pusd.balanceOf(address(sink)) + dripped, feeMinted, 10, "fee single-counted across drip+remit");
    }

    function test_redeem_after_onesided_flow_not_thin() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        pusd.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(50_000 ether);
        uint256 supplyAfter = vapurr.totalSupply();
        assertEq(supplyAfter, 1_000_000 ether - 50_000 ether, "expand burned V");
        assertEq(market.vInventory(), 0, "no inventory lock");

        // No pool heal — previously reverted THIN on inverted CP spread.
        (uint256 vOut,) = market.swapPusdToV(ask / 2);
        vm.stopPrank();

        assertGt(vOut, 0, "seigniorage redeem paid");
        assertEq(vapurr.totalSupply(), supplyAfter + vOut, "redeem minted V");
    }

    function test_computeSwap_allows_negative_cp_spread_with_min() public {
        vm.startPrank(trader);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(50_000 ether);
        vm.stopPrank();

        // Counterflow quote must succeed (was THIN).
        (uint256 ret, uint256 spread) = market.computeSwap(10_000 ether, false);
        assertGt(ret, 0, "retAmt");
        assertEq(spread, market.MIN_STABILITY_SPREAD(), "floor at min spread");
    }
}
