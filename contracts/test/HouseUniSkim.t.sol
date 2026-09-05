// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../HouseUniSkim.sol";
import "../HouseFeeRemit.sol";
import "../Remittance.sol";

contract MockSkimPusd is IERC20Remit {
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

contract HouseUniSkimTest is Test {
    MockSkimPusd pusd;
    RunwayFloor runway;
    RemittanceSink sink;
    HouseFeeRemit house;
    HouseUniSkim skim;
    address hook = address(0xFEEE);
    address stranger = address(0xBADD);

    function setUp() public {
        pusd = new MockSkimPusd();
        runway = new RunwayFloor(1_000 ether);
        sink = new RemittanceSink(address(pusd), address(runway));
        house = new HouseFeeRemit(address(pusd));
        house.setRemittance(address(sink));
        skim = new HouseUniSkim(address(pusd), address(house));
        skim.setHook(hook);
        pusd.mint(hook, 20_000 ether);
        vm.prank(hook);
        pusd.approve(address(skim), type(uint256).max);
    }

    function test_skim_credits_house_then_remit_to_sink() public {
        vm.prank(hook);
        uint256 credited = skim.skimToCredit(4_000 ether);
        assertEq(credited, 4_000 ether);
        assertEq(house.feeReserve(), 4_000 ether);
        assertEq(pusd.balanceOf(address(skim)), 0);
        assertEq(pusd.balanceOf(address(house)), 4_000 ether);

        uint256 sent = house.remitSurplus(0);
        assertEq(sent, 4_000 ether);
        assertEq(sink.accountedRfv(), 4_000 ether);
        assertEq(house.feeReserve(), 0);
    }

    function test_stranger_cannot_skim() public {
        pusd.mint(stranger, 100 ether);
        vm.prank(stranger);
        pusd.approve(address(skim), type(uint256).max);
        vm.prank(stranger);
        vm.expectRevert(bytes("AUTH"));
        skim.skimToCredit(50 ether);
    }

    function test_zero_skim_reverts() public {
        vm.prank(hook);
        vm.expectRevert(bytes("TINY"));
        skim.skimToCredit(0);
    }

    function test_owner_can_skim_without_hook_role() public {
        address own = skim.owner();
        pusd.mint(own, 1_000 ether);
        vm.prank(own);
        pusd.approve(address(skim), type(uint256).max);
        vm.prank(own);
        skim.skimToCredit(250 ether);
        assertEq(house.feeReserve(), 250 ether);
    }
}