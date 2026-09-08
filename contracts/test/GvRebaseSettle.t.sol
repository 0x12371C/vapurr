// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";

/// gV rebase settle proofs.
/// FAIL before fix / PASS after: late stake captures unpaid interval; empty-pool years award.
contract GvRebaseSettleTest is Test {
    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;

    address internal alice = address(0xA11CE);
    address internal bob = address(0xB0B);

    uint256 internal constant YEAR = 365 days;

    function setUp() public {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));

        v.mint(alice, 100_000 ether);
        v.mint(bob, 100_000 ether);
        v.setMinter(address(gV));
    }

    /// Attack: Bob stakes after a full interval and captures Alice emissions on next rebase.
    /// Post-fix: stake settles first, so Bob shares mint after Alice interval accrues.
    /// Uses unbound-bond mid default (350 bps).
    function test_late_stake_does_not_capture_prior_interval() public {
        assertEq(policy.policyRateBps(), 350, "mid default");
        vm.startPrank(alice);
        v.approve(address(gV), type(uint256).max);
        gV.stake(50_000 ether);
        vm.stopPrank();

        uint256 aliceBefore = gV.balanceOf(alice);
        vm.warp(block.timestamp + YEAR);

        vm.startPrank(bob);
        v.approve(address(gV), type(uint256).max);
        gV.stake(50_000 ether);
        vm.stopPrank();

        uint256 aliceAfterStake = gV.balanceOf(alice);
        uint256 bobAfterStake = gV.balanceOf(bob);

        uint256 expectedAlice = aliceBefore + (aliceBefore * 350) / 10_000;
        assertApproxEqRel(aliceAfterStake, expectedAlice, 1e12, "alice accrued before bob shares");
        assertApproxEqRel(bobAfterStake, 50_000 ether, 1e12, "bob gets no prior interval");

        vm.prank(policy.owner());
        uint256 minted = policy.rebase();
        assertEq(minted, 0, "interval already settled on stake");
        assertApproxEqRel(gV.balanceOf(alice), aliceAfterStake, 1e12, "alice unchanged");
        assertApproxEqRel(gV.balanceOf(bob), bobAfterStake, 1e12, "bob unchanged");
    }

    /// Attack: empty pool leaves lastRebase stale for years; first staker grabs multi-year mint.
    /// Post-fix: stake accrues (clocks lastRebase) before share mint.
    function test_empty_pool_stake_does_not_award_stale_years() public {
        vm.warp(block.timestamp + 5 * YEAR);

        vm.startPrank(alice);
        v.approve(address(gV), type(uint256).max);
        gV.stake(10_000 ether);
        vm.stopPrank();

        assertApproxEqRel(gV.balanceOf(alice), 10_000 ether, 1e12, "no multi-year award on empty-pool stake");

        vm.prank(policy.owner());
        uint256 minted = policy.rebase();
        assertEq(minted, 0, "lastRebase settled at stake; no backlog mint");
        assertApproxEqRel(gV.balanceOf(alice), 10_000 ether, 1e12, "still principal only");
    }
}
