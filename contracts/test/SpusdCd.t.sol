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

    function test_parameter_changes_only_affect_new_positions() public {
        cd.receiveRemittance(10_000 ether);
        vm.prank(user);
        uint256 oldId = cd.open(10_000 ether);
        cd.setParams(1000, 9000, 90 days);
        vm.prank(user);
        uint256 newId = cd.open(10_000 ether);
        assertEq(cd.couponDue(oldId), 500 ether);
        assertEq(cd.couponDue(newId), 1_000 ether);
        (,, uint64 oldUnlock,) = cd.positions(oldId);
        (,, uint64 newUnlock,) = cd.positions(newId);
        assertEq(newUnlock - oldUnlock, 60 days);
        (,, uint256 oldFee) = cd.previewClose(oldId);
        assertEq(oldFee, 200 ether);
        vm.warp(oldUnlock);
        vm.prank(user);
        (, uint256 coupon,) = cd.close(oldId);
        assertEq(coupon, 500 ether);
        assertEq(cd.totalPrincipal(), 10_000 ether);
        assertEq(cd.totalCouponDue(), 1_000 ether);
    }

    function test_underfunded_equal_coupons_share_surplus_in_either_order() public {
        cd.receiveRemittance(100 ether);
        vm.startPrank(user);
        uint256 first = cd.open(10_000 ether);
        uint256 second = cd.open(10_000 ether);
        vm.warp(block.timestamp + 30 days);
        (, uint256 secondCoupon,) = cd.close(second);
        (, uint256 firstCoupon,) = cd.close(first);
        vm.stopPrank();
        assertEq(firstCoupon, 50 ether);
        assertEq(secondCoupon, 50 ether);
        assertEq(cd.totalPrincipal(), 0);
        assertEq(cd.totalCouponDue(), 0);
        assertEq(cd.surplus(), 0);
    }

    function test_unmatured_coupon_share_is_not_swept_by_first_maturity() public {
        cd.receiveRemittance(100 ether);
        vm.prank(user);
        uint256 first = cd.open(10_000 ether);
        vm.warp(block.timestamp + 15 days);
        vm.prank(user);
        uint256 second = cd.open(10_000 ether);
        vm.warp(block.timestamp + 15 days);
        vm.prank(user);
        (, uint256 firstCoupon,) = cd.close(first);
        assertEq(firstCoupon, 50 ether);
        assertEq(cd.surplus(), 50 ether);
        vm.warp(block.timestamp + 15 days);
        vm.prank(user);
        (, uint256 secondCoupon,) = cd.close(second);
        assertEq(secondCoupon, 50 ether);
    }

    function test_early_close_releases_coupon_target_and_retains_break_fee() public {
        cd.receiveRemittance(100 ether);
        vm.startPrank(user);
        uint256 first = cd.open(10_000 ether);
        uint256 second = cd.open(10_000 ether);
        cd.close(first);
        vm.stopPrank();
        assertEq(cd.totalCouponDue(), 500 ether);
        assertEq(cd.surplus(), 300 ether);
        vm.warp(block.timestamp + 30 days);
        vm.prank(user);
        (uint256 principal, uint256 coupon,) = cd.close(second);
        assertEq(principal, 10_000 ether);
        assertEq(coupon, 300 ether);
        assertEq(cd.totalPrincipal(), 0);
    }

    function test_principal_and_direct_donation_are_not_coupon_budget() public {
        vm.prank(user);
        uint256 id = cd.open(10_000 ether);
        pusd.transfer(address(cd), 1_000 ether);
        assertEq(cd.availableSurplus(), 0);
        vm.warp(block.timestamp + 30 days);
        vm.prank(user);
        (uint256 principal, uint256 coupon,) = cd.close(id);
        assertEq(principal, 10_000 ether);
        assertEq(coupon, 0);
        assertEq(pusd.balanceOf(address(cd)), 1_000 ether);
    }

    function test_only_position_owner_can_close_and_cannot_close_twice() public {
        vm.prank(user);
        uint256 id = cd.open(10_000 ether);
        vm.expectRevert(bytes("POS"));
        cd.close(id);
        vm.prank(user);
        cd.close(id);
        vm.prank(user);
        vm.expectRevert(bytes("POS"));
        cd.close(id);
    }

    function test_invalid_term_cannot_wrap_unlock_timestamp() public {
        vm.expectRevert(bytes("TERM"));
        cd.setParams(500, 200, type(uint64).max);
    }

    function testFuzz_underfunded_weighted_coupons_preserve_principal(uint256 a, uint256 b, uint256 funding) public {
        a = bound(a, 1 ether, 100_000 ether);
        b = bound(b, 1 ether, 100_000 ether);
        funding = bound(funding, 1, 100 ether);
        cd.receiveRemittance(funding);
        vm.startPrank(user);
        uint256 first = cd.open(a);
        uint256 second = cd.open(b);
        vm.warp(block.timestamp + 30 days);
        (uint256 p1, uint256 c1,) = cd.close(first);
        (uint256 p2, uint256 c2,) = cd.close(second);
        vm.stopPrank();
        assertEq(p1 + p2, a + b);
        assertLe(c1 + c2, funding);
        assertLe(c1, a * 500 / 10_000);
        assertLe(c2, b * 500 / 10_000);
        assertEq(cd.totalPrincipal(), 0);
        assertEq(cd.totalCouponDue(), 0);
        assertEq(pusd.balanceOf(address(cd)), funding - c1 - c2);
    }
}
