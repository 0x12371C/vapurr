// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";
import "../IVapurrMinter.sol";
import "../PusdMarket.sol" as Mkt;

/// Single-minter pattern + interim dual-token fences (Fed V vs market-embedded V).
contract MintAuthorityTest is Test {
    VapurrToken internal fedV;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    BrowserStream internal stream;

    Mkt.PusdMarket internal market;

    address internal staker = address(0xA11CE);
    address internal browser = address(0xB10);
    address internal stranger = address(0xBAD);

    uint256 internal constant PRICE = 1e18;
    uint256 internal constant YEAR = 365 days;

    function setUp() public {
        fedV = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(fedV), address(policy));
        policy.bindGV(address(gV));
        stream = new BrowserStream(address(fedV));
        stream.setDistributor(browser);

        // Seed while test is minter, then hand sole mint role to gV (Fed end-state).
        fedV.mint(address(this), 1_000_000 ether);
        fedV.mint(staker, 100_000 ether);
        fedV.setMinter(address(gV));

        market = new Mkt.PusdMarket(PRICE);
        // Move some market genesis V to trader for inventory path.
        market.vapurr().transfer(staker, 100_000 ether);
    }

    function test_only_policy_minter_gV_can_mint() public {
        assertEq(IVapurrMinter(address(fedV)).minter(), address(gV), "sole minter is gV");

        uint256 supply0 = fedV.totalSupply();
        vm.startPrank(staker);
        fedV.approve(address(gV), type(uint256).max);
        gV.stake(50_000 ether);
        vm.stopPrank();

        vm.warp(block.timestamp + YEAR);
        vm.prank(policy.owner());
        uint256 minted = policy.rebase();
        assertGt(minted, 0, "rebase minted");
        assertEq(fedV.totalSupply(), supply0 + minted, "supply rose only via gV mint");

        vm.prank(stranger);
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(stranger, 1 ether);

        vm.prank(browser);
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(browser, 1 ether);

        vm.prank(address(stream));
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(address(stream), 1 ether);

        // Policy is not the token minter — gV holds mint rights.
        vm.prank(address(policy));
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(address(policy), 1 ether);
    }

    function test_stream_drip_and_browse_do_not_mint() public {
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
        vm.warp(block.timestamp + YEAR);
        uint256 due = s2.releasable();
        assertGt(due, 0);

        vm.prank(browser);
        s2.drip(staker, due);
        assertEq(v2.totalSupply(), supplyBefore, "stream drip must not mint");

        vm.prank(browser);
        vm.expectRevert(bytes("MINTER"));
        v2.setMinter(browser);

        vm.prank(address(s2));
        vm.expectRevert(bytes("MINTER"));
        v2.mint(browser, 1);
    }

    function test_setMinter_zero_or_one() public {
        vm.prank(address(gV));
        fedV.setMinter(address(0));
        assertEq(fedV.minter(), address(0), "zero minters");

        vm.prank(address(gV));
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(staker, 1);

        vm.prank(stranger);
        vm.expectRevert(bytes("MINTER"));
        fedV.setMinter(stranger);
    }

    function test_market_redeem_does_not_increase_fed_token_supply() public {
        uint256 fedSupply0 = fedV.totalSupply();
        Mkt.VapurrToken mktV = market.vapurr();
        Mkt.PusdToken pusd = market.pusd();

        assertTrue(address(mktV) != address(fedV), "Fed V != market V (interim)");

        vm.startPrank(staker);
        mktV.approve(address(market), type(uint256).max);
        pusd.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(100 ether);
        vm.stopPrank();

        uint256 mktSupplyAfter = mktV.totalSupply();
        // Heal poolDelta (slot 3) so reverse redeem is not CP-blocked.
        vm.store(address(market), bytes32(uint256(3)), bytes32(uint256(0)));

        vm.prank(staker);
        (uint256 vOut,) = market.swapPusdToV(ask / 2);
        assertGt(vOut, 0, "inventory redeem paid V");

        assertEq(mktV.totalSupply(), mktSupplyAfter, "market redeem did not mint market V");
        assertEq(fedV.totalSupply(), fedSupply0, "market redeem must not touch Fed V supply");
    }

    function test_market_has_no_mint_call_on_redeem_path() public {
        Mkt.VapurrToken mktV = market.vapurr();
        Mkt.PusdToken pusd = market.pusd();
        uint256 supply0 = mktV.totalSupply();

        vm.startPrank(staker);
        mktV.approve(address(market), type(uint256).max);
        pusd.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(50 ether);
        vm.stopPrank();

        vm.store(address(market), bytes32(uint256(3)), bytes32(uint256(0)));
        vm.prank(staker);
        market.swapPusdToV(ask / 2);

        assertEq(mktV.totalSupply(), supply0, "inventory unwrap only");
        assertEq(mktV.minter(), address(market), "embedded V minter still market");
    }
}