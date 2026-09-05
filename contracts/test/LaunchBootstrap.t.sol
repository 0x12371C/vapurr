// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken as FedV} from "../GvFed.sol";
import {PusdMarket, VapurrToken as LegacyV} from "../PusdMarket.sol";
import {CanonicalLitheFactory} from "../CanonicalLitheFactory.sol";
import {LaunchBootstrap} from "../LaunchBootstrap.sol";
import {ExogenousPairRegistry, EXO_TAG_ETH} from "../ExogenousPairRegistry.sol";
import {DevFundStream} from "../DevFundStream.sol";

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

contract LaunchBootstrapTest is Test {
    function _factory() internal returns (CanonicalLitheFactory factory, FedV v) {
        PusdMarket legacyMarket = new PusdMarket(1 ether);
        LegacyV legacyV = legacyMarket.vapurr();
        factory = new CanonicalLitheFactory(
            address(legacyMarket), address(legacyV), legacyV.totalSupply(), 50_000 ether, 1 ether
        );
        v = factory.canonicalV();
    }

    function test_bootstrap_wires_devfund_to_oliver() public {
        (CanonicalLitheFactory factory, FedV v) = _factory();
        address pusd = address(factory.market().pusd());
        MockOliverBoot oliver = new MockOliverBoot(address(v), pusd);

        LaunchBootstrap boot = new LaunchBootstrap(
            address(v),
            address(oliver),
            address(0xDE01),
            address(0x55554447),
            pusd,
            address(new MockExoToken()),
            address(new MockExoToken()),
            address(new MockExoToken()),
            false
        );

        v.approve(address(boot), 200_000 ether);
        boot.fundAndStart();

        DevFundStream fund = boot.devFund();
        assertTrue(boot.funded());
        assertEq(fund.deposited(), 200_000 ether);
        assertEq(address(fund.oliver()), address(oliver));
        assertEq(v.balanceOf(address(0xDE01)), 0);

        vm.warp(block.timestamp + 365 days);
        fund.settle();
        assertEq(v.balanceOf(address(0xDE01)), 0, "no V to recipient");
        assertGt(oliver.collatV(address(fund)), 0, "V locked in Oliver");
        assertEq(v.minter(), address(factory.gV()));
    }

    function test_bootstrap_registers_exo_pairs_bans_usdg() public {
        (, FedV v) = _factory();
        address pusd = address(0xBEEF);
        // Oliver needs matching vapurr; use a dedicated mock + real pusd stand-in address for ban list.
        MockOliverBoot oliver = new MockOliverBoot(address(v), pusd);
        // Re-bind: DevFundStream requires oliver.pusd() ? mock returns pusd. V must match.
        // For registry bans we need the factory market pusd ? rebuild lightly:
        (CanonicalLitheFactory factory2, FedV v2) = _factory();
        pusd = address(factory2.market().pusd());
        oliver = new MockOliverBoot(address(v2), pusd);
        v = v2;

        address usdg = address(0x55554447);
        MockExoToken eth = new MockExoToken();

        LaunchBootstrap boot = new LaunchBootstrap(
            address(v),
            address(oliver),
            address(0xDE01),
            usdg,
            pusd,
            address(eth),
            address(new MockExoToken()),
            address(new MockExoToken()),
            true
        );

        v.approve(address(boot), 200_000 ether);
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
