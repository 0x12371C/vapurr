// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../HouseFeeRemit.sol";
import "../Remittance.sol";

contract MockHousePusd is IERC20Remit {
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

contract HouseFeeRemitTest is Test {
    MockHousePusd pusd;
    RunwayFloor runway;
    RemittanceSink sink;
    HouseFeeRemit house;
    address skimmer = address(0xFEE);

    function setUp() public {
        pusd = new MockHousePusd();
        runway = new RunwayFloor(1_000 ether);
        sink = new RemittanceSink(address(pusd), address(runway));
        house = new HouseFeeRemit(address(pusd));
        house.setRemittance(address(sink));
        pusd.mint(skimmer, 50_000 ether);
        vm.prank(skimmer);
        pusd.approve(address(house), type(uint256).max);
    }

    function test_credit_then_remit_lands_in_sink() public {
        vm.prank(skimmer);
        house.creditFees(5_000 ether);
        assertEq(house.feeReserve(), 5_000 ether);
        uint256 sent = house.remitSurplus(0);
        assertEq(sent, 5_000 ether);
        assertEq(house.feeReserve(), 0);
        assertEq(sink.accountedRfv(), 5_000 ether);
        assertEq(pusd.balanceOf(address(house)), 0);
    }

    function test_empty_remit_reverts() public {
        vm.expectRevert(bytes("TINY"));
        house.remitSurplus(0);
    }

    function test_partial_remit_keeps_remainder() public {
        vm.prank(skimmer);
        house.creditFees(10_000 ether);
        uint256 sent = house.remitSurplus(3_000 ether);
        assertEq(sent, 3_000 ether);
        assertEq(house.feeReserve(), 7_000 ether);
        assertEq(sink.accountedRfv(), 3_000 ether);
        assertEq(pusd.balanceOf(address(house)), 7_000 ether);
    }

    function test_unset_remittance_reverts() public {
        HouseFeeRemit bare = new HouseFeeRemit(address(pusd));
        vm.prank(skimmer);
        pusd.approve(address(bare), type(uint256).max);
        vm.prank(skimmer);
        bare.creditFees(100 ether);
        vm.expectRevert(bytes("REMIT"));
        bare.remitSurplus(0);
    }
}