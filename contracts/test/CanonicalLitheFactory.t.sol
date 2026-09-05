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

/// One transaction must create a fully connected successor book and leave no
/// temporary factory authority behind.
contract CanonicalLitheFactoryTest is Test {
    uint256 internal constant PRICE = 1 ether;
    uint256 internal constant BOOTSTRAP = 100_000 ether;

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

    function setUp() public {
        legacyMarket = new PusdMarket(PRICE);
        legacyV = legacyMarket.vapurr();
        legacyPusd = legacyMarket.pusd();
        factory =
            new CanonicalLitheFactory(address(legacyMarket), address(legacyV), legacyV.totalSupply(), BOOTSTRAP, PRICE);
        canonicalV = factory.canonicalV();
        market = factory.market();
        loop = factory.loop();
        converter = factory.converter();
        migrator = factory.migrator();
    }

    function test_factory_builds_one_token_lithe_stack() public view {
        RebasePolicy policy = factory.policy();
        gVAPURR gV = factory.gV();

        assertEq(address(market.vapurr()), address(canonicalV), "canonical Lithe uses Fed V");
        assertEq(address(loop.vapurr()), address(canonicalV), "Oliver uses Fed V");
        assertEq(canonicalV.minter(), address(gV), "gV is sole V minter");
        assertEq(policy.owner(), address(this), "policy handed to initiator");
        assertEq(market.owner(), address(this), "Lithe owner is initiator");
        assertEq(loop.owner(), address(this), "Oliver owner handed to initiator");
        assertEq(converter.available(), legacyV.totalSupply(), "full legacy V conversion inventory");
        assertEq(market.vInventory(), BOOTSTRAP, "explicit Lithe bootstrap inventory");
        assertEq(canonicalV.totalSupply(), legacyV.totalSupply() + BOOTSTRAP, "declared cutover allocation only");
    }

    function test_factory_wires_atomic_lithe_pusd_migration() public {
        legacyV.transfer(holder, 20_000 ether);
        vm.startPrank(holder);
        legacyV.approve(address(legacyMarket), type(uint256).max);
        (uint256 legacyPusdIn,) = legacyMarket.swapVToPusd(10_000 ether);
        legacyPusd.approve(address(migrator), type(uint256).max);
        vm.stopPrank();

        uint256 supply0 = canonicalV.totalSupply();
        vm.prank(holder);
        uint256 canonicalPusdOut = migrator.migrate(legacyPusdIn);

        assertGt(canonicalPusdOut, 0, "canonical Lithe output");
        assertEq(legacyPusd.balanceOf(holder), 0, "old Lithe input burned");
        assertEq(market.pusd().balanceOf(holder), canonicalPusdOut, "new Lithe output paid");
        assertEq(canonicalV.totalSupply(), supply0, "migration does not mint V");
    }

    function test_factory_rejects_a_lie_about_legacy_supply() public {
        uint256 legacySupply = legacyV.totalSupply();
        vm.expectRevert(bytes("SUPPLY"));
        new CanonicalLitheFactory(address(legacyMarket), address(legacyV), legacySupply - 1, BOOTSTRAP, PRICE);
    }
}
