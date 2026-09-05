// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../PusdLoop.sol";
import "../Remittance.sol";

/// P0 fix #5: shared RunwayFloor + realized-only remittance (no circular RFV).
contract RunwayRfvTest is Test {
    PusdMarket internal market;
    VapurrToken internal vapurr;
    PusdToken internal pusd;
    PusdLoop internal loop;
    RunwayFloor internal runway;
    RemittanceSink internal sink;

    address internal supplier = address(0x5110);
    address internal borrower = address(0xB02B);

    uint256 internal constant PRICE = 1e18;

    function setUp() public {
        market = new PusdMarket(PRICE);
        vapurr = market.vapurr();
        pusd = market.pusd();
        loop = new PusdLoop(address(market));
        runway = new RunwayFloor(0);
        sink = new RemittanceSink(address(pusd), address(runway));

        vapurr.transfer(supplier, 200_000 ether);
        vm.startPrank(supplier);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(100_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vapurr.approve(address(loop), type(uint256).max);
        vm.stopPrank();
    }

    /// Dual modules share one RunwayFloor instance (single treasury runway SoT).
    function test_oliver_and_lithe_share_same_runway_floor() public {
        loop.setRemittance(address(sink), address(runway), false);
        market.setRemittance(address(sink), address(runway), false);

        assertEq(address(loop.runway()), address(runway), "Oliver runway");
        assertEq(address(market.runway()), address(runway), "Lithe runway");
        assertEq(address(loop.runway()), address(market.runway()), "shared instance");

        runway.setFloor(42 ether);
        assertEq(loop.runway().floor(), 42 ether, "Oliver sees shared floor");
        assertEq(market.runway().floor(), 42 ether, "Lithe sees shared floor");
        assertEq(runway.remittable(100 ether), 58 ether, "remittable alias");
    }

    /// Unpaid interest must not pull depositor cash into RFV (circular).
    function test_cannot_remit_unpaid_interest_from_depositor_cash() public {
        vm.prank(supplier);
        loop.supply(20_000 ether);

        vapurr.transfer(borrower, 30_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(30_000 ether);
        loop.borrow(5_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vm.stopPrank();

        loop.setRemittance(address(sink), address(runway), false);

        vm.warp(block.timestamp + 365 days);
        loop.accrue();

        // Owner has pending reserve from unpaid interest; not remittable until collected.
        assertGt(loop.pendingReserve(), 0, "fee pending after accrue");
        assertEq(loop.realizedReserve(), 0, "unpaid not yet realized");
        assertEq(loop.realizedRemittable(), 0, "unpaid interest not remittable");
        uint256 sent = loop.remitReserve(type(uint256).max / 4);
        assertEq(sent, 0, "no remit from unpaid claims");
        assertEq(pusd.balanceOf(address(sink)), 0, "sink empty - no circular RFV");

        // User claim still fully backed by cash + borrows (no double-count as RFV).
        PusdLoop.Snap memory s = loop.snapshot(supplier);
        assertGe(s.cash + s.totalBorrowAssets_, s.supplied, "user claim backed");
    }

    /// After interest is collected, remit is possible but still cannot go below floor,
    /// and remitting must not impair depositor claims (no RFV + user double-count).
    function test_realized_remit_respects_floor_and_user_claims() public {
        vm.prank(supplier);
        loop.supply(20_000 ether);

        vapurr.transfer(borrower, 30_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(30_000 ether);
        loop.borrow(5_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vm.stopPrank();

        loop.setRemittance(address(sink), address(runway), false);

        vm.warp(block.timestamp + 365 days);
        loop.accrue();

        // Collect interest into cash (realize pending reserve fees).
        vm.prank(borrower);
        loop.repay(2_000 ether);

        assertGt(loop.realizedReserve(), 0, "repay realized fee cash");
        uint256 free0 = loop.realizedRemittable();
        assertGt(free0, 0, "repay realized remittable cash");

        // Floor blocks remittance of the retained runway slice.
        runway.setFloor(free0);
        assertEq(loop.realizedRemittable(), 0, "at floor => nothing remittable");
        assertEq(loop.remitReserve(free0), 0, "cannot remit below/at floor");

        // Half floor: remit only surplus above floor.
        runway.setFloor(free0 / 2);
        uint256 expect = free0 - free0 / 2;
        PusdLoop.Snap memory snapBefore = loop.snapshot(supplier);
        uint256 sent = loop.remitReserve(type(uint256).max / 4);
        assertEq(sent, expect, "surplus above shared floor only");
        assertEq(pusd.balanceOf(address(sink)), sent, "sink got realized surplus");

        PusdLoop.Snap memory snapAfter = loop.snapshot(supplier);
        // Depositor claim must remain backed by remaining cash + borrows (not double-counted as RFV).
        assertGe(snapAfter.cash + snapAfter.totalBorrowAssets_, snapAfter.supplied, "user claim still backed");
        // RFV dollars left the vault cash+borrow book (owner reserve shares burned).
        assertLt(snapAfter.cash + snapAfter.totalBorrowAssets_, snapBefore.cash + snapBefore.totalBorrowAssets_, "RFV exited book");
        // Same dollar must not sit in sink and still fully inflate owner+user claims.
        assertEq(pusd.balanceOf(address(sink)), sent, "RFV only in sink once");
    }

    /// Lithe also cannot remit below the shared floor; floor change affects both.
    function test_lithe_cannot_remit_below_shared_floor() public {
        vm.startPrank(supplier);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(40_000 ether);
        vm.stopPrank();

        uint256 reserve = market.yieldReserve();
        assertGt(reserve, 1 ether, "fees funded");

        loop.setRemittance(address(sink), address(runway), false);
        market.setRemittance(address(sink), address(runway), false);

        runway.setFloor(reserve);
        assertEq(market.remitSurplus(0), 0, "Lithe blocked at floor");
        assertEq(market.yieldReserve(), reserve, "reserve retained");

        runway.setFloor(reserve / 2);
        uint256 sent = market.remitSurplus(0);
        assertEq(sent, reserve - reserve / 2, "Lithe remits above shared floor");
        assertEq(market.yieldReserve(), reserve / 2, "floor retained in Lithe");
        assertEq(pusd.balanceOf(address(sink)), sent, "sink credited once");
    }

    /// Liq that collects accrued interest must realize pending reserve the same way repay does,
    /// so remittable surplus grows and the book stays solvent.
    function test_liq_with_accrued_interest_realizes_reserve() public {
        vm.prank(supplier);
        loop.supply(20_000 ether);

        // Size like OliverOracleBadDebt: 10k V + 8k debt so 0.5x feed is liquidatable.
        vapurr.transfer(borrower, 10_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(10_000 ether);
        loop.borrow(8_000 ether);
        vm.stopPrank();

        loop.setRemittance(address(sink), address(runway), false);

        vm.warp(block.timestamp + 365 days);
        loop.accrue();

        uint256 pendingBefore = loop.pendingReserve();
        assertGt(pendingBefore, 0, "fee pending after accrue");
        assertEq(loop.realizedReserve(), 0, "unpaid not yet realized");
        assertEq(loop.realizedRemittable(), 0, "unpaid interest not remittable");

        // Crash collat: 10k V * 0.5 = 5k; LLTV 90% => max 4.5k < debt => LIQ.
        // Heartbeat also refreshes oracle freshness after the warp.
        market.feed(5e17);

        address keeper = address(0x1111);
        vm.startPrank(supplier);
        pusd.transfer(keeper, 20_000 ether);
        vm.stopPrank();

        uint256 realizedBefore = loop.realizedReserve();
        vm.startPrank(keeper);
        pusd.approve(address(loop), type(uint256).max);
        loop.liquidate(borrower, 5_000 ether);
        vm.stopPrank();

        uint256 realizedAfter = loop.realizedReserve();
        assertGt(realizedAfter, realizedBefore, "liq realizes fee cash");
        assertLt(loop.pendingReserve(), pendingBefore, "pending burned into realized");
        assertGt(loop.realizedRemittable(), 0, "liq interest remittable surplus");

        // Book still works: supplier claim backed; remit moves only realized surplus.
        PusdLoop.Snap memory s = loop.snapshot(supplier);
        assertGe(s.cash + s.totalBorrowAssets_, s.supplied, "user claim backed after liq");

        uint256 free = loop.realizedRemittable();
        runway.setFloor(free / 2);
        uint256 sent = loop.remitReserve(type(uint256).max / 4);
        assertEq(sent, free - free / 2, "remit surplus above floor after liq realize");
        assertEq(pusd.balanceOf(address(sink)), sent, "sink got liq-realized surplus");

        PusdLoop.Snap memory afterRemit = loop.snapshot(supplier);
        assertGe(
            afterRemit.cash + afterRemit.totalBorrowAssets_,
            afterRemit.supplied,
            "user claim still backed after remit"
        );
    }
}
