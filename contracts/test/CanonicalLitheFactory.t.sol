// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken as FedV, RebasePolicy, gVAPURR} from "../GvFed.sol";
import {PusdMarket, PusdToken, VapurrToken as LegacyV} from "../PusdMarket.sol";
import {PusdMarketFed} from "../PusdMarketFed.sol";
import {PusdLoop} from "../PusdLoop.sol";
import {LegacyVConverter} from "../LegacyVConverter.sol";
import {LitheCutoverMigrator} from "../LitheCutoverMigrator.sol";
import {CanonicalLitheFactory} from "../CanonicalLitheFactory.sol";
import {GenesisAllocation} from "../GenesisAllocation.sol";

/// Factory mints exactly 1.2M; legacy converter carved from 800k remainder.
contract CanonicalLitheFactoryTest is Test, GenesisAllocation {
    uint256 internal constant PRICE = 1 ether;
    uint256 internal constant LEGACY = 288_000 ether;

    PusdMarket internal legacyMarket;
    LegacyV internal legacyV;
    PusdToken internal legacyPusd;
    CanonicalLitheFactory internal factory;
    FedV internal canonicalV;
    PusdMarketFed internal market;
    PusdLoop internal loop;
    LegacyVConverter internal converter;
    LitheCutoverMigrator internal migrator;

    address internal holder = address(0xC0DE);

    function _legacyAt(uint256 targetSupply) internal returns (PusdMarket mkt, LegacyV v) {
        mkt = new PusdMarket(PRICE);
        v = mkt.vapurr();
        uint256 have = v.totalSupply();
        if (have > targetSupply) {
            mkt.swapVToPusd(have - targetSupply);
        }
        require(v.totalSupply() == targetSupply, "legacy fixture");
    }

    function setUp() public {
        (legacyMarket, legacyV) = _legacyAt(LEGACY);
        legacyPusd = legacyMarket.pusd();
        factory = new CanonicalLitheFactory(address(legacyMarket), address(legacyV), LEGACY, 0, PRICE);
        canonicalV = factory.canonicalV();
        market = factory.market();
        loop = factory.loop();
        converter = factory.converter();
        migrator = factory.migrator();
    }

    function test_factory_mints_exactly_1_2M_and_carves_legacy() public view {
        RebasePolicy policy = factory.policy();
        gVAPURR gV = factory.gV();

        assertEq(address(market.vapurr()), address(canonicalV), "canonical Lithe uses Fed V");
        assertEq(address(loop.vapurr()), address(canonicalV), "Oliver uses Fed V");
        assertEq(canonicalV.minter(), address(gV), "gV is policy minter");
        assertEq(canonicalV.marketMinter(), address(market), "Lithe is marketMinter");
        assertEq(policy.owner(), address(this), "policy handed to initiator");
        assertEq(market.owner(), address(this), "Lithe owner is initiator");
        assertEq(loop.owner(), address(this), "Oliver owner handed to initiator");
        assertEq(converter.available(), LEGACY, "full legacy V conversion inventory");
        assertEq(market.vInventory(), 0, "no Lithe redeem inventory under seigniorage");
        assertEq(canonicalV.totalSupply(), GENESIS_MINT, "exactly 1.2M before any rebase");
        assertEq(factory.bootstrapV(), LAUNCH_V, "launch locked at 1M");
        assertEq(factory.devFundAllocation(), DEV_FUND_AMOUNT, "DevFund 200k extra");
        assertEq(factory.treasuryNet(), TREASURY_GROSS - LEGACY, "800k minus carve");
        assertEq(
            canonicalV.balanceOf(address(this)),
            GENESIS_MINT - LEGACY,
            "initiator holds launch+DevFund minus converter carve"
        );
        assertEq(canonicalV.balanceOf(address(converter)), LEGACY, "converter holds carved remainder");
        assertEq(BROWSERSTREAM_V + POL_ETH_V + POL_NVDA_V + POL_AMD_V + HOUSE_SEED_V + TREASURY_GROSS, LAUNCH_V);
        assertEq(LAUNCH_V + DEV_FUND_AMOUNT, GENESIS_MINT);
    }

    function test_factory_wires_atomic_lithe_pusd_migration() public {
        legacyV.transfer(holder, 20_000 ether);
        vm.startPrank(holder);
        (uint256 legacyPusdIn,) = legacyMarket.swapVToPusd(10_000 ether);
        legacyPusd.approve(address(migrator), type(uint256).max);
        vm.stopPrank();

        uint256 supply0 = canonicalV.totalSupply();
        vm.prank(holder);
        uint256 canonicalPusdOut = migrator.migrate(legacyPusdIn);

        assertGt(canonicalPusdOut, 0, "canonical Lithe output");
        assertEq(legacyPusd.balanceOf(holder), 0, "old Lithe input burned");
        assertEq(market.pusd().balanceOf(holder), canonicalPusdOut, "new Lithe output paid");
        assertLt(canonicalV.totalSupply(), supply0, "expand burned canonical V");
    }

    function test_factory_rejects_a_lie_about_legacy_supply() public {
        vm.expectRevert(bytes("SUPPLY"));
        new CanonicalLitheFactory(address(legacyMarket), address(legacyV), LEGACY - 1, 0, PRICE);
    }

    function test_factory_rejects_legacy_above_treasury_gross() public {
        PusdMarket fat = new PusdMarket(PRICE);
        address lv = address(fat.vapurr());
        uint256 fatSupply = LegacyV(lv).totalSupply();
        assertGt(fatSupply, TREASURY_GROSS, "fixture is 1M gen-4 book");
        vm.expectRevert(bytes("TREASURY"));
        new CanonicalLitheFactory(address(fat), lv, fatSupply, 0, PRICE);
    }
}
