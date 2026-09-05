// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../SpusdCd.sol";

contract MockCdPusd is IERC20Remit {
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

contract SpusdCdTest is Test {
    MockCdPusd pusd;
    SpusdCd cd;
    address user = address(0xBEEF);

    function setUp() public {
        pusd = new MockCdPusd();
        cd = new SpusdCd(address(pusd), 500, 200, 30 days);
        pusd.mint(user, 1_000_000 ether);
        pusd.mint(address(this), 100_000 ether);
        vm.prank(user);
        pusd.approve(address(cd), type(uint256).max);
        pusd.approve(address(cd), type(uint256).max);
    }

    function test_early_exit_break_fee_to_surplus() public {
        vm.prank(user);
        uint256 id = cd.open(10_000 ether);
        vm.prank(user);
        (uint256 principalOut, uint256 couponOut, uint256 fee) = cd.close(id);
        assertEq(couponOut, 0);
        assertEq(fee, 200 ether);
        assertEq(principalOut, 9_800 ether);
        assertEq(cd.surplus(), 200 ether);
        assertEq(pusd.balanceOf(user), 1_000_000 ether - 200 ether);
    }

    function test_maturity_coupon_from_surplus_only() public {
        cd.receiveRemittance(1_000 ether);
        vm.prank(user);
        uint256 id = cd.open(10_000 ether);
        vm.warp(block.timestamp + 30 days);
        vm.prank(user);
        (uint256 principalOut, uint256 couponOut, uint256 fee) = cd.close(id);
        assertEq(fee, 0);
        assertEq(principalOut, 10_000 ether);
        assertEq(couponOut, 500 ether);
        assertEq(cd.surplus(), 1_000 ether - 500 ether);
        assertEq(pusd.balanceOf(user), 1_000_000 ether + 500 ether);
    }

    function test_underfunded_coupon_does_not_mint() public {
        cd.receiveRemittance(100 ether);
        vm.prank(user);
        uint256 id = cd.open(10_000 ether);
        vm.warp(block.timestamp + 30 days);
        vm.prank(user);
        (uint256 principalOut, uint256 couponOut,) = cd.close(id);
        assertEq(principalOut, 10_000 ether);
        assertEq(couponOut, 100 ether);
        assertEq(cd.surplus(), 0);
    }
}