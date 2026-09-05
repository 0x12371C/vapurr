// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../PusdLoop.sol";

/// Stub Fed backstop: pulls pre-approved PUSD into the vault up to `need`.
contract StubFedBackstop is IFedBackstop {
    IERC20 public immutable pusd;
    address public immutable funder;

    constructor(address pusd_, address funder_) {
        pusd = IERC20(pusd_);
        funder = funder_;
    }

    function coverBadDebt(address vault, uint256 need) external returns (uint256 funded) {
        funded = need;
        uint256 bal = pusd.balanceOf(funder);
        if (funded > bal) funded = bal;
        if (funded == 0) return 0;
        require(pusd.transferFrom(funder, vault, funded), "PULL");
    }
}

/// Reverting backstop — absorb must still socialize without freezing the book.
contract RevertingBackstop is IFedBackstop {
    function coverBadDebt(address, uint256) external pure returns (uint256) {
        revert("BACKSTOP_DOWN");
    }
}

/// Oliver oracle freshness + bad-debt absorb proofs (P0 fix #4).
contract OliverOracleBadDebtTest is Test {
    PusdMarket internal market;
    VapurrToken internal vapurr;
    PusdToken internal pusd;
    PusdLoop internal loop;

    address internal supplier = address(0x5110);
    address internal borrower = address(0xB02B);
    address internal other = address(0x3333);

    uint256 internal constant PRICE = 1e18;

    function setUp() public {
        market = new PusdMarket(PRICE);
        vapurr = market.vapurr();
        pusd = market.pusd();
        loop = new PusdLoop(address(market));

        vapurr.transfer(supplier, 300_000 ether);
        vm.startPrank(supplier);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(150_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vapurr.approve(address(loop), type(uint256).max);
        vm.stopPrank();
    }

    function _seedBook() internal {
        vm.startPrank(supplier);
        loop.supply(50_000 ether);
        vm.stopPrank();

        vapurr.transfer(borrower, 10_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(10_000 ether);
        // 85% LTV on 10k V @ 1 PUSD = 8500 borrow max; take 8000
        loop.borrow(8_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vm.stopPrank();
    }

    /// Stale oracle must block unsafe borrow (FAIL before freshness / PASS after).
    function test_stale_rate_blocks_borrow() public {
        _seedBook();

        // Heartbeat goes stale.
        vm.warp(block.timestamp + loop.MAX_RATE_AGE() + 1);

        // Even a tiny top-up borrow must reject STALE (would size against inflated/stale px).
        vm.prank(borrower);
        vm.expectRevert(bytes("STALE"));
        loop.borrow(1 ether);
    }

    function test_stale_rate_blocks_withdraw_v() public {
        _seedBook();
        vm.warp(block.timestamp + loop.MAX_RATE_AGE() + 1);

        vm.prank(borrower);
        vm.expectRevert(bytes("STALE"));
        loop.withdrawV(1 ether);
    }

    function test_fresh_feed_restores_borrow_room_check() public {
        _seedBook();
        vm.warp(block.timestamp + loop.MAX_RATE_AGE() + 1);

        // Owner heartbeats same rate — freshness restored; over-LTV borrow still LTV-reverts.
        market.feed(PRICE);
        vm.prank(borrower);
        vm.expectRevert(bytes("LTV"));
        loop.borrow(1_000 ether);
    }

    function test_pending_devaluation_tightens_before_swap() public {
        _seedBook();
        // Crash pending to 0.6 without swap apply — credit path must prefer lower pending.
        market.feed(6e17);

        // At 0.6, 10k V = 6k collat; debt 8k > 85% of 6k => already over LTV on any action needing health.
        // withdrawV of dust must fail LTV under conservative pending.
        vm.prank(borrower);
        vm.expectRevert(bytes("LTV"));
        loop.withdrawV(1);
    }

    function test_feed_jump_clamp_rejects_spike() public {
        // >50% jump from live
        vm.expectRevert(bytes("JUMP"));
        market.feed(PRICE + (PRICE * 6e17) / 1e18); // +60%
    }

    /// Underwater zero-collat position can be absorbed without freezing repay for others.
    function test_absorb_bad_debt_does_not_freeze_repay() public {
        _seedBook();

        // Crash collateral to ~dust via oracle, then seize via liquidate path until empty.
        // Drop within jump clamp stepwise: 1.0 -> 0.5 -> 0.25 -> 0.125
        market.feed(5e17);
        // Apply first-spot so live rate matches (liq uses credit which already prefers pending).
        // Force apply by touching a zero-size? Need a swap for live; credit path uses pending min already.
        // Make borrower liquidatable: debt 8k, collat 10k*0.5=5k, LLTV 90% => max debt 4.5k; 8k > 4.5k => liquidatable.

        address keeper = address(0x1111);
        vm.startPrank(supplier);
        pusd.transfer(keeper, 20_000 ether);
        vm.stopPrank();
        vm.startPrank(keeper);
        pusd.approve(address(loop), type(uint256).max);
        // Max repay ~= collat / 1.05. Seize all V.
        loop.liquidate(borrower, 10_000 ether);
        vm.stopPrank();

        // After liq, dust collat may remain from rounding; residual debt remains.
        PusdLoop.Snap memory s = loop.snapshot(borrower);
        assertGt(s.debt, 0, "residual bad debt");
        assertLt(s.collatValue, s.debt, "underwater");

        // Absorb sweeps dust + socializes residual.
        uint256 debtBefore = s.debt;
        loop.absorbBadDebt(borrower);
        assertEq(loop.borrowShares(borrower), 0, "debt shares cleared");
        assertGe(loop.badDebtSocialized(), debtBefore, "socialized accounted");

        // Other borrower can still repay — book not frozen.
        vapurr.transfer(other, 5_000 ether);
        vm.startPrank(other);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(5_000 ether);
        loop.borrow(1_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        loop.repay(500 ether);
        vm.stopPrank();

        PusdLoop.Snap memory o = loop.snapshot(other);
        assertLt(o.debt, 1_000 ether, "partial repay worked");
    }

    function test_absorb_with_reverting_backstop_still_clears() public {
        _seedBook();
        market.feed(5e17);

        address keeper = address(0x1111);
        vm.startPrank(supplier);
        pusd.transfer(keeper, 20_000 ether);
        vm.stopPrank();
        vm.startPrank(keeper);
        pusd.approve(address(loop), type(uint256).max);
        loop.liquidate(borrower, 10_000 ether);
        vm.stopPrank();

        loop.setBackstop(address(new RevertingBackstop()));
        loop.absorbBadDebt(borrower);
        assertEq(loop.borrowShares(borrower), 0, "cleared despite backstop revert");

        // Supplier withdraw of free cash still works (repay path unrelated to absorb).
        vm.prank(supplier);
        loop.withdraw(100 ether);
    }

    function test_absorb_prefers_backstop_cover() public {
        _seedBook();
        market.feed(5e17);

        address keeper = address(0x1111);
        vm.startPrank(supplier);
        pusd.transfer(keeper, 20_000 ether);
        vm.stopPrank();
        vm.startPrank(keeper);
        pusd.approve(address(loop), type(uint256).max);
        loop.liquidate(borrower, 10_000 ether);
        vm.stopPrank();

        uint256 residual = loop.snapshot(borrower).debt;
        assertGt(residual, 0, "residual");

        address funder = address(0xFE21);
        vm.prank(supplier);
        pusd.transfer(funder, residual);
        StubFedBackstop bs = new StubFedBackstop(address(pusd), funder);
        vm.prank(funder);
        pusd.approve(address(bs), type(uint256).max);
        loop.setBackstop(address(bs));

        uint256 socialBefore = loop.badDebtSocialized();
        uint256 cashBefore = pusd.balanceOf(address(loop));
        loop.absorbBadDebt(borrower);
        assertEq(loop.borrowShares(borrower), 0, "cleared");
        assertEq(loop.badDebtSocialized(), socialBefore, "no socialize when covered");
        assertGe(pusd.balanceOf(address(loop)), cashBefore, "backstop cash landed");
    }

    function test_stale_blocks_liquidate_sizing() public {
        _seedBook();
        market.feed(5e17);
        vm.warp(block.timestamp + loop.MAX_RATE_AGE() + 1);

        address keeper = address(0x1111);
        vm.prank(supplier);
        pusd.transfer(keeper, 5_000 ether);
        vm.startPrank(keeper);
        pusd.approve(address(loop), type(uint256).max);
        vm.expectRevert(bytes("STALE"));
        loop.liquidate(borrower, 1_000 ether);
        vm.stopPrank();
    }
}
