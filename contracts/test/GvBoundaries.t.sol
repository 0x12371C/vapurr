// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";

/// Trust-boundary proofs for Fed gV / BrowserStream / wgV.
contract GvBoundariesTest is Test {
    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    wgVAPURR internal wgV;
    BrowserStream internal stream;

    address internal staker = address(0xA11CE);
    address internal browser = address(0xB10);
    address internal earnUser = address(0xE4A);

    uint256 internal constant YEAR = 365 days;

    function setUp() public {
        _deployFresh();
    }

    function _deployFresh() internal {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        wgV = new wgVAPURR(address(gV));
        stream = new BrowserStream(address(v));
        stream.setDistributor(browser);

        // Seed while test contract is still V minter
        v.mint(address(this), 1_000_000 ether);
        v.mint(staker, 100_000 ether);
        // Hand mint rights to gV — sole inflation path thereafter
        v.setMinter(address(gV));
    }

    function test_annualized_rebase_approx_3_5_pct() public {
        vm.startPrank(staker);
        v.approve(address(gV), type(uint256).max);
        gV.stake(100_000 ether);
        vm.stopPrank();

        uint256 beforeBal = gV.balanceOf(staker);
        uint256 beforeSupply = v.totalSupply();

        vm.warp(block.timestamp + YEAR);
        vm.prank(policy.owner());
        uint256 minted = policy.rebase();

        uint256 afterBal = gV.balanceOf(staker);
        uint256 afterSupply = v.totalSupply();

        uint256 expected = (beforeBal * 350) / 10_000;
        assertEq(minted, expected, "minted != 3.5%");
        assertApproxEqRel(afterBal, beforeBal + expected, 1e12, "staker bal ~ +3.5%");
        assertEq(afterSupply, beforeSupply + minted, "V supply rose only by rebase mint");
        uint256 bps = ((afterBal - beforeBal) * 10_000) / beforeBal;
        assertEq(bps, 350, "annualized bps");
    }

    function test_browser_stream_drip_does_not_mint() public {
        // Dedicated deploy: fund earmark before transferring V minter to gV
        VapurrToken v2 = new VapurrToken();
        RebasePolicy p2 = new RebasePolicy();
        gVAPURR g2 = new gVAPURR(address(v2), address(p2));
        p2.bindGV(address(g2));
        BrowserStream s2 = new BrowserStream(address(v2));
        s2.setDistributor(browser);

        v2.mint(address(this), 50_000 ether);
        v2.approve(address(s2), 50_000 ether);
        s2.fund(50_000 ether);
        s2.startStream();
        v2.setMinter(address(g2));

        uint256 supplyBefore = v2.totalSupply();
        vm.warp(block.timestamp + (3 * YEAR) / 2);
        uint256 due = s2.releasable();
        assertGt(due, 0, "should vest");

        vm.prank(browser);
        s2.drip(earnUser, due);

        assertEq(v2.totalSupply(), supplyBefore, "HARD WALL: drip must not mint");
        assertEq(v2.balanceOf(earnUser), due, "user received transfer");
    }

    function test_browse_cannot_call_rebase() public {
        vm.startPrank(staker);
        v.approve(address(gV), type(uint256).max);
        gV.stake(10_000 ether);
        vm.stopPrank();

        vm.warp(block.timestamp + YEAR);

        vm.prank(browser);
        vm.expectRevert(bytes("POLICY"));
        gV.rebase();

        vm.prank(browser);
        vm.expectRevert(bytes("OWN"));
        policy.rebase();
    }

    function test_wgV_shares_track_gV_across_rebase() public {
        vm.startPrank(staker);
        v.approve(address(gV), type(uint256).max);
        gV.stake(50_000 ether);
        gV.approve(address(wgV), type(uint256).max);
        uint256 shares = wgV.wrap(40_000 ether);
        vm.stopPrank();

        assertEq(shares, 40_000 ether - wgV.DEAD_SHARES(), "first wrap locks dead shares");
        uint256 rate0 = wgV.gvPerShare();
        uint256 gv0 = gV.balanceOf(address(wgV));

        vm.warp(block.timestamp + YEAR);
        vm.prank(policy.owner());
        policy.rebase();

        uint256 rate1 = wgV.gvPerShare();
        uint256 gv1 = gV.balanceOf(address(wgV));
        assertEq(wgV.balanceOf(staker), shares, "wgV shares non-rebasing");
        assertGt(gv1, gv0, "wrapper gV grew");
        assertGt(rate1, rate0, "gv per share rose");

        vm.prank(staker);
        uint256 out = wgV.unwrap(shares);
        assertApproxEqRel(out, (40_000 ether * 10_350) / 10_000, 1e12, "unwrap tracks rebase");
    }
}
