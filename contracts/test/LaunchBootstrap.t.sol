// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken as FedV, BrowserStream} from "../GvFed.sol";
import {PusdMarket, VapurrToken as LegacyV} from "../PusdMarket.sol";
import {CanonicalLitheFactory} from "../CanonicalLitheFactory.sol";
import {LaunchBootstrap} from "../LaunchBootstrap.sol";
import {ExogenousPairRegistry, ExogenousSeedMarket, EXO_TAG_ETH} from "../ExogenousPairRegistry.sol";
import {DevFundStream} from "../DevFundStream.sol";
import {GenesisTreasury} from "../GenesisTreasury.sol";
import {GenesisAllocation} from "../GenesisAllocation.sol";

contract MockExoToken {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) allowance[from][msg.sender] = a - amt;
        balanceOf[from] -= amt;
        balanceOf[to] += amt;
        return true;
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        balanceOf[msg.sender] -= amt;
        balanceOf[to] += amt;
        return true;
    }
}

contract MockOliverBoot {
    address public immutable vapurr;
    address public immutable pusdToken;
    mapping(address => uint256) public collatV;

    constructor(address vapurr_, address pusd_) {
        vapurr = vapurr_;
        pusdToken = pusd_;
    }

    function pusd() external view returns (address) {
        return pusdToken;
    }

    function depositV(uint256 amt) external {
        require(FedV(vapurr).transferFrom(msg.sender, address(this), amt), "PULL");
        collatV[msg.sender] += amt;
    }

    function borrow(uint256) external pure {}
    function repay(uint256) external pure {}
}

contract LaunchBootstrapTest is Test, GenesisAllocation {
    uint256 internal constant LEGACY = 288_000 ether;

    function _factory() internal returns (CanonicalLitheFactory factory, FedV v) {
        PusdMarket legacyMarket = new PusdMarket(1 ether);
        LegacyV legacyV = legacyMarket.vapurr();
        uint256 have = legacyV.totalSupply();
        if (have > LEGACY) legacyMarket.swapVToPusd(have - LEGACY);
        factory = new CanonicalLitheFactory(address(legacyMarket), address(legacyV), LEGACY, 0, 1 ether);
        v = factory.canonicalV();
    }

    function test_bootstrap_allocates_1M_and_devfund_200k() public {
        (CanonicalLitheFactory factory, FedV v) = _factory();
        address pusd = address(factory.market().pusd());
        MockOliverBoot oliver = new MockOliverBoot(address(v), pusd);

        LaunchBootstrap boot = new LaunchBootstrap(
            address(v),
            address(oliver),
            address(factory.gV()),
            address(0xDE01),
            address(0x55554447),
            pusd,
            address(new MockExoToken()),
            address(new MockExoToken()),
            address(new MockExoToken()),
            factory.treasuryNet(),
            false
        );

        uint256 pull = boot.pullAmount();
        assertEq(pull, GENESIS_MINT - LEGACY, "pull is 1.2M minus carve");
        v.approve(address(boot), pull);
        boot.fundAndStart();

        assertTrue(boot.funded());
        assertEq(boot.devFund().deposited(), DEV_FUND_AMOUNT);
        assertEq(v.balanceOf(address(boot.devFund())), DEV_FUND_AMOUNT);
        assertEq(v.balanceOf(address(0xDE01)), 0, "no V to DevFund recipient");

        BrowserStream stream = boot.browserStream();
        assertEq(v.balanceOf(address(stream)), BROWSERSTREAM_V, "50k BrowserStream");

        assertEq(ExogenousSeedMarket(boot.ethSeed()).reserve0(), POL_ETH_V, "80k V/ETH");
        assertEq(ExogenousSeedMarket(boot.nvdaSeed()).reserve0(), POL_NVDA_V, "25k NVDA");
        assertEq(ExogenousSeedMarket(boot.amdSeed()).reserve0(), POL_AMD_V, "25k AMD");
        assertEq(boot.houseSeedHeld(), HOUSE_SEED_V, "20k House earmark");
        assertEq(v.balanceOf(address(boot)), HOUSE_SEED_V, "House seed sits on bootstrap");

        GenesisTreasury tre = boot.treasury();
        assertEq(tre.deposited(), factory.treasuryNet());
        assertEq(oliver.collatV(address(tre)), factory.treasuryNet(), "treasury remainder in Oliver");
        assertEq(tre.stakedGv(), 0, "moved gV -> Oliver at genesis");
        assertEq(v.balanceOf(address(this)), 0, "initiator has no leftover dump float");
        assertEq(v.totalSupply(), GENESIS_MINT, "supply still 1.2M");

        vm.expectRevert(GenesisTreasury.NoMarketSell.selector);
        tre.withdrawV(address(this), 1);

        vm.warp(block.timestamp + 365 days);
        boot.devFund().settle();
        assertEq(v.balanceOf(address(0xDE01)), 0, "no V to recipient after vest");
        assertGt(oliver.collatV(address(boot.devFund())), 0, "DevFund V locked in Oliver");
        assertEq(v.minter(), address(factory.gV()));
    }

    function test_bootstrap_registers_exo_pairs_bans_usdg() public {
        (CanonicalLitheFactory factory2, FedV v) = _factory();
        address pusd = address(factory2.market().pusd());
        MockOliverBoot oliver = new MockOliverBoot(address(v), pusd);

        address usdg = address(0x55554447);
        MockExoToken eth = new MockExoToken();

        LaunchBootstrap boot = new LaunchBootstrap(
            address(v),
            address(oliver),
            address(factory2.gV()),
            address(0xDE01),
            usdg,
            pusd,
            address(eth),
            address(new MockExoToken()),
            address(new MockExoToken()),
            factory2.treasuryNet(),
            true
        );

        v.approve(address(boot), boot.pullAmount());
        boot.fundAndStart();

        ExogenousPairRegistry reg = boot.registry();
        (address exoEth,, bool en) = reg.pairOf(EXO_TAG_ETH);
        assertEq(exoEth, address(eth));
        assertTrue(en);
        assertTrue(boot.ethSeed() != address(0));

        vm.expectRevert(ExogenousPairRegistry.UsdgNotPairAsset.selector);
        reg.registerPair("USDG", usdg, true);
    }
}
