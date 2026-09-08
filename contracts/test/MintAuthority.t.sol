// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";
import "../IVapurrMinter.sol";
import "../PusdMarket.sol" as Mkt;

/// Dual-printer pattern + interim dual-token fences (Fed V vs market-embedded V).
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

        fedV.mint(address(this), 1_000_000 ether);
        fedV.mint(staker, 100_000 ether);
        fedV.setMinter(address(gV));

        market = new Mkt.PusdMarket(PRICE);
        market.vapurr().transfer(staker, 100_000 ether);
    }

    function test_only_policy_minter_gV_can_mint_policy_path() public {
        assertEq(IVapurrMinter(address(fedV)).minter(), address(gV), "policy minter is gV");
        assertEq(fedV.marketMinter(), address(0), "no market minter in this fixture");

        uint256 supply0 = fedV.totalSupply();
        vm.startPrank(staker);
        fedV.approve(address(gV), type(uint256).max);
        gV.stake(50_000 ether);
        vm.stopPrank();

        vm.warp(block.timestamp + YEAR);
        vm.prank(policy.owner());
        uint256 minted = policy.rebase();
        assertGt(minted, 0, "rebase minted");
        assertEq(fedV.totalSupply(), supply0 + minted, "supply rose via gV mint");

        vm.prank(stranger);
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(stranger, 1 ether);

        vm.prank(browser);
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(browser, 1 ether);

        vm.prank(address(stream));
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(address(stream), 1 ether);

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

    function test_setMinter_zero_or_one_policy() public {
        vm.prank(address(gV));
        fedV.setMinter(address(0));
        assertEq(fedV.minter(), address(0), "zero policy minters");

        vm.prank(address(gV));
        vm.expectRevert(bytes("MINTER"));
        fedV.mint(staker, 1);

        vm.prank(stranger);
        vm.expectRevert(bytes("MINTER"));
        fedV.setMinter(stranger);
    }

    function test_embedded_market_seigniorage_burns_and_mints_own_v() public {
        uint256 fedSupply0 = fedV.totalSupply();
        Mkt.VapurrToken mktV = market.vapurr();
        Mkt.PusdToken pusd = market.pusd();

        assertTrue(address(mktV) != address(fedV), "Fed V != market V (interim)");

        vm.startPrank(staker);
        (uint256 ask,) = market.swapVToPusd(100 ether);
        uint256 afterBurn = mktV.totalSupply();
        assertEq(afterBurn, 1_000_000 ether - 100 ether, "expand burned market V");

        pusd.approve(address(market), type(uint256).max);
        vm.store(address(market), bytes32(uint256(3)), bytes32(uint256(0)));
        (uint256 vOut,) = market.swapPusdToV(ask / 2);
        vm.stopPrank();

        assertGt(vOut, 0, "seigniorage redeem minted V");
        assertEq(mktV.totalSupply(), afterBurn + vOut, "redeem minted market V");
        assertEq(fedV.totalSupply(), fedSupply0, "embedded path must not touch Fed V");
        assertEq(mktV.minter(), address(market), "embedded V minter still market");
    }

    function test_market_minter_role_enables_fed_seigniorage() public {
        // Fresh Fed V + Lithe-style caller via marketMinter
        VapurrToken v = new VapurrToken();
        v.setMarketMinter(address(this));
        v.setMinter(address(gV)); // hand policy away; we remain marketMinter

        uint256 s0 = v.totalSupply();
        v.mint(staker, 5 ether);
        assertEq(v.totalSupply(), s0 + 5 ether, "marketMinter can mint");
        v.burn(staker, 2 ether);
        assertEq(v.totalSupply(), s0 + 3 ether, "marketMinter can burn");

        vm.prank(stranger);
        vm.expectRevert(bytes("MINTER"));
        v.mint(stranger, 1);
    }

    /// P0 boundary: after handoff, policy minter (gV) mints only — burn is marketMinter-exclusive.
    function test_policy_minter_cannot_burn_holders() public {
        VapurrToken v = new VapurrToken();
        address lithe = address(uint160(0x1177E));
        v.mint(staker, 10 ether);
        v.setMarketMinter(lithe);
        v.setMinter(address(gV));

        // gV / policy path cannot burn
        vm.prank(address(gV));
        vm.expectRevert(bytes("MARKET"));
        v.burn(staker, 1 ether);

        vm.prank(stranger);
        vm.expectRevert(bytes("MARKET"));
        v.burn(staker, 1 ether);

        // marketMinter still can (seigniorage expand)
        vm.prank(lithe);
        v.burn(staker, 1 ether);
        assertEq(v.balanceOf(staker), 9 ether, "market burn ok");
    }

    /// Handoff race fence: setMarketMinter then setMinter(gV); deployer loses both roles.
    function test_handoff_order_deployer_loses_mint_after_setMinter() public {
        VapurrToken v = new VapurrToken();
        address lithe = address(uint160(0x1177E));
        v.setMarketMinter(lithe);
        v.setMinter(address(gV));

        vm.expectRevert(bytes("MINTER"));
        v.mint(staker, 1);

        vm.expectRevert(bytes("MINTER"));
        v.setMarketMinter(stranger);

        vm.prank(address(gV));
        v.mint(staker, 1 ether);
        assertEq(v.balanceOf(staker), 1 ether);
    }

}
