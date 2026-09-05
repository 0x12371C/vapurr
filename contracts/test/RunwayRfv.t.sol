// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../PusdLoop.sol";
import "../Remittance.sol";
import "../SPUSD.sol";

/// P0: sink-level RFV / cross-branch floor + realized-only remittance.
contract RunwayRfvTest is Test {
    PusdMarket internal market;
    VapurrToken internal vapurr;
    PusdToken internal pusd;
    PusdLoop internal loop;
    RunwayFloor internal runway;
    RemittanceSink internal sink;
    SPUSD internal spusd;

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
        spusd = new SPUSD(address(pusd));
        sink.setForward(address(spusd));

        vapurr.transfer(supplier, 200_000 ether);
        vm.startPrank(supplier);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(100_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vapurr.approve(address(loop), type(uint256).max);
        vm.stopPrank();
    }

    /// Dual modules wire one sink + one RunwayFloor (sink is RFV SoT).
    function test_oliver_and_lithe_share_same_runway_floor() public {
        loop.setRemittance(address(sink), address(runway), false);
        market.setRemittance(address(sink), address(runway), false);

        assertEq(address(loop.runway()), address(runway), "Oliver runway");
        assertEq(address(market.runway()), address(runway), "Lithe runway");
        assertEq(address(sink.runway()), address(runway), "sink runway SoT");
        assertEq(address(loop.runway()), address(market.runway()), "shared instance");

        runway.setFloor(42 ether);
        assertEq(loop.runway().floor(), 42 ether, "Oliver sees shared floor");
        assertEq(market.runway().floor(), 42 ether, "Lithe sees shared floor");
        assertEq(sink.runway().floor(), 42 ether, "sink sees shared floor");
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

    /// Realized remit consolidates into sink; floor gates sink forward, not branch retain.
    /// Remitting must not impair depositor claims (no RFV + user double-count).
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

        // Branch remits full realized into sink even with a high floor (no local retain).
        runway.setFloor(free0);
        PusdLoop.Snap memory snapBefore = loop.snapshot(supplier);
        uint256 sent = loop.remitReserve(type(uint256).max / 4);
        assertEq(sent, free0, "full realized remits to sink");
        assertEq(sink.accountedRfv(), sent, "sink consolidated RFV");
        assertEq(sink.surplus(), 0, "at sink floor => nothing forwardable");
        assertEq(sink.retainedFloor(), sent, "floor retained in sink");

        vm.expectRevert(bytes("FLOOR"));
        sink.forwardSurplus(0);

        // Half floor: forward only surplus above sink floor.
        runway.setFloor(sent / 2);
        uint256 expectFwd = sent - sent / 2;
        assertEq(sink.surplus(), expectFwd, "sink surplus above floor");
        uint256 fwd = sink.forwardSurplus(0);
        assertEq(fwd, expectFwd, "forward surplus only");
        assertEq(sink.accountedRfv(), sent / 2, "floor retained after forward");
        assertEq(sink.surplus(), 0, "no further surplus");

        PusdLoop.Snap memory snapAfter = loop.snapshot(supplier);
        // Depositor claim must remain backed by remaining cash + borrows.
        assertGe(snapAfter.cash + snapAfter.totalBorrowAssets_, snapAfter.supplied, "user claim still backed");
        // RFV dollars left the vault cash+borrow book (owner reserve shares burned).
        assertLt(
            snapAfter.cash + snapAfter.totalBorrowAssets_,
            snapBefore.cash + snapBefore.totalBorrowAssets_,
            "RFV exited book"
        );
    }

    /// Two branches remit into one sink; floor enforced once on consolidated cash.
    function test_two_branches_remit_one_sink_floor() public {
        // Lithe fee inventory.
        vm.startPrank(supplier);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(40_000 ether);
        vm.stopPrank();
        uint256 litheReserve = market.yieldReserve();
        assertGt(litheReserve, 1 ether, "fees funded");

        // Oliver realized interest.
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
        market.setRemittance(address(sink), address(runway), false);

        vm.warp(block.timestamp + 365 days);
        loop.accrue();
        vm.prank(borrower);
        loop.repay(2_000 ether);
        uint256 oliverFree = loop.realizedRemittable();
        assertGt(oliverFree, 0, "Oliver realized");

        // High floor does not block branch remits into sink.
        // remitSurplus accrues/drips first — snapshot remaining realized after settle.
        market.accrue();
        uint256 litheLeft = market.yieldReserve();
        assertGt(litheLeft, 0, "Lithe still has realized fees");
        assertLt(litheLeft, litheReserve, "drip consumed some yieldReserve");
        runway.setFloor(type(uint256).max);
        uint256 fromLithe = market.remitSurplus(0);
        uint256 fromOliver = loop.remitReserve(type(uint256).max / 4);
        assertApproxEqAbs(fromLithe, litheLeft, 1, "Lithe remits full realized");
        assertApproxEqAbs(fromOliver, oliverFree, 1, "Oliver remits full realized");

        uint256 consolidated = sink.accountedRfv();
        assertApproxEqAbs(consolidated, fromLithe + fromOliver, 1, "one sink holds both");
        assertEq(sink.surplus(), 0, "max floor => no forwardable");
        assertEq(market.yieldReserve(), 0, "Lithe emptied into sink");
        assertEq(loop.realizedRemittable(), 0, "Oliver emptied into sink");

        // Cannot drain below floor; lower floor unlocks consolidated surplus once.
        runway.setFloor(consolidated / 2);
        uint256 expectFwd = consolidated - consolidated / 2;
        assertEq(sink.surplus(), expectFwd, "shared surplus");
        uint256 fwd = sink.forwardSurplus(0);
        assertEq(fwd, expectFwd, "single floor forward");
        assertEq(sink.retainedFloor(), consolidated / 2, "retained once");
        vm.expectRevert(bytes("FLOOR"));
        sink.forwardSurplus(1);

        // User claims on Oliver still protected after branch remits.
        PusdLoop.Snap memory s = loop.snapshot(supplier);
        assertGe(s.cash + s.totalBorrowAssets_, s.supplied, "user claim backed");
    }

    /// Lithe remits full realized into sink; floor retained at sink not in yieldReserve.
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
        uint256 sent = market.remitSurplus(0);
        assertEq(sent, reserve, "Lithe remits full into sink");
        assertEq(market.yieldReserve(), 0, "no local floor retain");
        assertEq(sink.accountedRfv(), reserve, "sink holds RFV");
        assertEq(sink.surplus(), 0, "at floor nothing forwardable");
        vm.expectRevert(bytes("FLOOR"));
        sink.forwardSurplus(0);

        runway.setFloor(reserve / 2);
        uint256 fwd = sink.forwardSurplus(0);
        assertEq(fwd, reserve - reserve / 2, "forward above sink floor");
        assertEq(sink.accountedRfv(), reserve / 2, "floor retained in sink");
    }

    /// Liq that collects accrued interest must realize pending reserve the same way repay does.
    function test_liq_with_accrued_interest_realizes_reserve() public {
        vm.prank(supplier);
        loop.supply(20_000 ether);

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

        PusdLoop.Snap memory s = loop.snapshot(supplier);
        assertGe(s.cash + s.totalBorrowAssets_, s.supplied, "user claim backed after liq");

        uint256 free = loop.realizedRemittable();
        runway.setFloor(free / 2); // does not block branch remit
        uint256 sent = loop.remitReserve(type(uint256).max / 4);
        assertEq(sent, free, "full realized remits to sink after liq");
        assertEq(sink.accountedRfv(), sent, "sink got liq-realized surplus");
        assertEq(sink.surplus(), sent - sent / 2, "floor retained at sink");

        PusdLoop.Snap memory afterRemit = loop.snapshot(supplier);
        assertGe(
            afterRemit.cash + afterRemit.totalBorrowAssets_,
            afterRemit.supplied,
            "user claim still backed after remit"
        );
    }
}
