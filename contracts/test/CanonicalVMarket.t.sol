// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken as FedV, RebasePolicy, gVAPURR} from "../GvFed.sol";
import {PusdMarket, PusdToken, VapurrToken as LegacyV} from "../PusdMarket.sol";
import {PusdMarketFed} from "../PusdMarketFed.sol";
import {PusdLoop} from "../PusdLoop.sol";
import {LegacyVConverter} from "../LegacyVConverter.sol";
import {LitheCutoverMigrator} from "../LitheCutoverMigrator.sol";

/// Cutover proofs: seigniorage Lithe on Fed V; dual printers (gV policy + Lithe market).
contract CanonicalVMarketTest is Test {
    uint256 internal constant PRICE = 1 ether;
    uint256 internal constant YEAR = 365 days;

    FedV internal canonical;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    PusdMarketFed internal market;
    PusdLoop internal loop;
    PusdToken internal pusd;

    address internal trader = address(0xA11CE);
    address internal borrower = address(0xB0B);
    address internal legacyHolder = address(0x1E6AC);

    function setUp() public {
        canonical = new FedV();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(canonical), address(policy));
        policy.bindGV(address(gV));

        canonical.mint(address(this), 400_000 ether);
        canonical.mint(trader, 200_000 ether);
        canonical.mint(borrower, 30_000 ether);

        market = new PusdMarketFed(address(canonical), PRICE, address(this));
        pusd = market.pusd();
        // Dual printers: Lithe seigniorage then gV policy.
        canonical.setMarketMinter(address(market));
        canonical.setMinter(address(gV));

        loop = new PusdLoop(address(market));
    }

    function test_seigniorage_burn_on_expand_mint_on_redeem() public {
        uint256 supply0 = canonical.totalSupply();

        assertEq(address(market.vapurr()), address(canonical), "market uses Fed V");
        assertEq(canonical.marketMinter(), address(market), "Lithe is marketMinter");

        vm.startPrank(trader);
        (uint256 pusdOut,) = market.swapVToPusd(50_000 ether);
        uint256 supplyAfterExpand = canonical.totalSupply();
        assertEq(supplyAfterExpand, supply0 - 50_000 ether, "expand burns V");

        pusd.approve(address(market), type(uint256).max);
        (uint256 vOut,) = market.swapPusdToV(pusdOut / 2);
        vm.stopPrank();

        assertGt(vOut, 0, "redeem minted V");
        assertEq(canonical.totalSupply(), supplyAfterExpand + vOut, "redeem mints V");
        assertEq(market.vInventory(), 0, "no inventory float under seigniorage");

        PusdMarketFed.Snap memory snap = market.snapshot(trader);
        assertEq(snap.vapurrToken, address(canonical), "snapshot exposes canonical V");
        assertEq(snap.vapurrSupply, canonical.totalSupply(), "snapshot supply matches");
    }

    function test_market_keeps_the_desk_snapshot_abi() public view {
        (bool ok, bytes memory raw) = address(market).staticcall(abi.encodeWithSignature("snapshot(address)", trader));
        assertTrue(ok, "snapshot call");
        assertEq(raw.length, 12 * 32, "12-word market snapshot");
    }

    function test_dual_printers_gv_policy_and_lithe_seigniorage() public {
        assertEq(canonical.minter(), address(gV), "gV is policy minter");
        assertEq(canonical.marketMinter(), address(market), "Lithe is market minter");

        vm.startPrank(trader);
        canonical.approve(address(gV), type(uint256).max);
        gV.stake(10_000 ether);
        vm.stopPrank();

        uint256 supply0 = canonical.totalSupply();
        vm.warp(block.timestamp + YEAR);
        uint256 minted = policy.rebase();
        assertGt(minted, 0, "policy-triggered gV rebase mints");
        assertEq(canonical.totalSupply(), supply0 + minted, "gV rebase changed supply");

        // Stranger cannot mint; stream role is not a minter.
        vm.expectRevert(bytes("MINTER"));
        vm.prank(trader);
        canonical.mint(trader, 1 ether);
    }

    function test_oliver_retargets_to_canonical_v() public {
        vm.startPrank(trader);
        market.swapVToPusd(30_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        loop.supply(20_000 ether);
        vm.stopPrank();

        vm.startPrank(borrower);
        canonical.approve(address(loop), type(uint256).max);
        loop.depositV(10_000 ether);
        loop.borrow(3_000 ether);
        vm.stopPrank();

        assertEq(address(loop.vapurr()), address(canonical), "vault collateral is Fed V");
        assertEq(loop.collatV(borrower), 10_000 ether, "canonical V collateral posted");
        assertEq(pusd.balanceOf(borrower), 3_000 ether, "borrow paid PUSD cash");
    }

    function test_legacy_conversion_is_immediate_and_inventory_only() public {
        PusdMarket legacyMarket = new PusdMarket(PRICE);
        LegacyV legacy = legacyMarket.vapurr();
        FedV target = new FedV();
        LegacyVConverter converter = new LegacyVConverter(address(legacy), address(target));

        legacy.transfer(legacyHolder, 800 ether);
        target.mint(address(this), 600 ether);
        target.approve(address(converter), type(uint256).max);
        converter.fund(600 ether);

        uint256 legacySupply0 = legacy.totalSupply();
        uint256 canonicalSupply0 = target.totalSupply();

        vm.startPrank(legacyHolder);
        legacy.approve(address(converter), type(uint256).max);
        converter.convert(400 ether);
        vm.stopPrank();

        assertEq(converter.converted(), 400 ether, "conversion ledger");
        assertEq(converter.legacyLocked(), 400 ether, "legacy V remains locked");
        assertEq(target.balanceOf(legacyHolder), 400 ether, "funded canonical V paid 1:1");
        assertEq(converter.available(), 200 ether, "conversion cannot exceed inventory");
        assertEq(legacy.totalSupply(), legacySupply0, "converter does not burn or mint legacy V");
        assertEq(target.totalSupply(), canonicalSupply0, "converter does not mint canonical V");

        vm.prank(legacyHolder);
        vm.expectRevert(bytes("INV"));
        converter.convert(201 ether);
    }

    function test_lithe_migrates_legacy_pusd_through_seigniorage() public {
        PusdMarket legacyMarket = new PusdMarket(PRICE);
        LegacyV legacyV = legacyMarket.vapurr();
        PusdToken legacyPusd = legacyMarket.pusd();
        LegacyVConverter converter = new LegacyVConverter(address(legacyV), address(canonical));

        // Fund converter from pre-handoff genesis holdings on this contract (no new mint).
        canonical.approve(address(converter), type(uint256).max);
        converter.fund(50_000 ether);

        LitheCutoverMigrator migrator =
            new LitheCutoverMigrator(address(legacyMarket), address(market), address(converter));

        legacyV.transfer(legacyHolder, 20_000 ether);
        vm.startPrank(legacyHolder);
        (uint256 legacyPusdIn,) = legacyMarket.swapVToPusd(10_000 ether);
        legacyPusd.approve(address(migrator), type(uint256).max);
        vm.stopPrank();

        uint256 legacyPusdSupply0 = legacyPusd.totalSupply();
        uint256 canonicalVSupply0 = canonical.totalSupply();

        vm.prank(legacyHolder);
        uint256 canonicalPusdOut = migrator.migrate(legacyPusdIn);

        assertGt(canonicalPusdOut, 0, "canonical Lithe minted PUSD");
        assertEq(legacyPusd.balanceOf(legacyHolder), 0, "legacy PUSD was redeemed");
        assertEq(legacyPusd.totalSupply(), legacyPusdSupply0 - legacyPusdIn, "legacy Lithe burned input");
        // Expand burns the converted canonical V.
        assertLt(canonical.totalSupply(), canonicalVSupply0, "seigniorage expand burned canonical V");
        assertGt(converter.legacyLocked(), 0, "converter received legacy V");
        assertEq(pusd.balanceOf(legacyHolder), canonicalPusdOut, "holder received new PUSD");
    }
}
