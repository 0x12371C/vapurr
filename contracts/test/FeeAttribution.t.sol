// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../FeeAttribution.sol";
import "../Remittance.sol";
import "../HouseFeeRemit.sol";

contract MockAttrPusd is IERC20Remit {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amt) external {
        balanceOf[to] += amt;
    }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        return true;
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        require(balanceOf[msg.sender] >= amt, "BAL");
        balanceOf[msg.sender] -= amt;
        balanceOf[to] += amt;
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        require(a >= amt && balanceOf[from] >= amt, "ALLOW");
        if (a != type(uint256).max) allowance[from][msg.sender] = a - amt;
        balanceOf[from] -= amt;
        balanceOf[to] += amt;
        return true;
    }
}

contract FeeAttributionTest is Test {
    MockAttrPusd pusd;
    RunwayFloor runway;
    RemittanceSink sink;
    FeeAttribution attr;
    HouseFeeRemit house;

    address lithe = address(0x117E);
    address oliver = address(0x01171);
    address payer = address(0xFEE);

    function setUp() public {
        pusd = new MockAttrPusd();
        runway = new RunwayFloor(1_000 ether);
        sink = new RemittanceSink(address(pusd), address(runway));
        attr = new FeeAttribution(address(pusd), address(sink));
        house = new HouseFeeRemit(address(pusd));
        house.setRemittance(address(attr));

        attr.register(address(house), FeeAttribution.Source.House);
        attr.register(lithe, FeeAttribution.Source.Lithe);
        attr.register(oliver, FeeAttribution.Source.Oliver);

        pusd.mint(payer, 100_000 ether);
        vm.prank(payer);
        pusd.approve(address(house), type(uint256).max);
        vm.prank(payer);
        pusd.approve(address(attr), type(uint256).max);
        vm.prank(lithe);
        pusd.approve(address(attr), type(uint256).max);
        vm.prank(oliver);
        pusd.approve(address(attr), type(uint256).max);
    }

    function test_house_remit_attributes_and_lands_in_sink() public {
        vm.prank(payer);
        house.creditFees(5_000 ether);
        uint256 sent = house.remitSurplus(0);
        assertEq(sent, 5_000 ether);
        assertEq(attr.contributed(FeeAttribution.Source.House), 5_000 ether);
        assertEq(attr.totalContributed(), 5_000 ether);
        assertEq(sink.accountedRfv(), 5_000 ether);
        assertEq(pusd.balanceOf(address(attr)), 0, "attr holds no cash");
    }

    function test_three_sources_breakdown_and_share_bps() public {
        // House via fee carve
        vm.prank(payer);
        house.creditFees(2_000 ether);
        house.remitSurplus(0);

        // Lithe / Oliver via registered receiveRemittance
        pusd.mint(lithe, 3_000 ether);
        pusd.mint(oliver, 5_000 ether);
        vm.prank(lithe);
        attr.receiveRemittance(3_000 ether);
        vm.prank(oliver);
        attr.receiveRemittance(5_000 ether);

        (uint256 h, uint256 l, uint256 o, uint256 total) = attr.breakdown();
        assertEq(h, 2_000 ether);
        assertEq(l, 3_000 ether);
        assertEq(o, 5_000 ether);
        assertEq(total, 10_000 ether);
        assertEq(sink.accountedRfv(), 10_000 ether);

        assertEq(attr.shareBps(FeeAttribution.Source.House), 2_000);
        assertEq(attr.shareBps(FeeAttribution.Source.Lithe), 3_000);
        assertEq(attr.shareBps(FeeAttribution.Source.Oliver), 5_000);
    }

    function test_credit_rejects_unknown_accepts_tagged() public {
        vm.prank(payer);
        vm.expectRevert(bytes("SRC"));
        attr.credit(FeeAttribution.Source.Unknown, 100 ether);

        vm.prank(payer);
        attr.credit(FeeAttribution.Source.Lithe, 100 ether);
        assertEq(attr.contributed(FeeAttribution.Source.Lithe), 100 ether);
        assertEq(sink.accountedRfv(), 100 ether);
    }

    function test_unregistered_branch_counts_unknown() public {
        address stranger = address(0xBAD);
        pusd.mint(stranger, 500 ether);
        vm.prank(stranger);
        pusd.approve(address(attr), type(uint256).max);
        vm.prank(stranger);
        attr.receiveRemittance(500 ether);
        assertEq(attr.contributed(FeeAttribution.Source.Unknown), 500 ether);
        assertEq(attr.contributed(FeeAttribution.Source.House), 0);
        assertEq(sink.accountedRfv(), 500 ether);
    }

    function test_empty_share_bps_is_zero() public {
        assertEq(attr.shareBps(FeeAttribution.Source.House), 0);
    }

    function test_attribution_does_not_bypass_sink_floor() public {
        vm.prank(payer);
        attr.credit(FeeAttribution.Source.Oliver, 1_500 ether);
        // floor = 1000; surplus forwardable = 500
        assertEq(sink.accountedRfv(), 1_500 ether);
        assertEq(sink.surplus(), 500 ether);
        assertEq(attr.contributed(FeeAttribution.Source.Oliver), 1_500 ether);
    }
}