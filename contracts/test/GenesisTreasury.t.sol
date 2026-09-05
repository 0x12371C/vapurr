// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken, RebasePolicy, gVAPURR} from "../GvFed.sol";
import {GenesisTreasury} from "../GenesisTreasury.sol";

contract MockPusd {
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
        require(balanceOf[msg.sender] >= amt, "PUSD");
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

contract MockOliver {
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
        require(VapurrToken(vapurr).transferFrom(msg.sender, address(this), amt), "PULL");
        collatV[msg.sender] += amt;
    }

    function borrow(uint256 amt) external {
        MockPusd(pusdToken).mint(msg.sender, amt);
    }

    function repay(uint256) external {}
}

contract GenesisTreasuryTest is Test {
    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    MockPusd internal pusd;
    MockOliver internal oliver;
    GenesisTreasury internal tre;
    address internal drawer = address(0xD0);

    function setUp() public {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        pusd = new MockPusd();
        oliver = new MockOliver(address(v), address(pusd));
        tre = new GenesisTreasury(address(v), address(gV), address(oliver), drawer);
        v.mint(address(this), 512_000 ether);
        v.setMinter(address(gV));
    }

    function test_lock_stakes_gv_then_oliver_and_bans_market_sell() public {
        v.approve(address(tre), 512_000 ether);
        tre.fund(512_000 ether);
        tre.lock();
        assertEq(tre.stakedGv(), 512_000 ether, "staked as gV");
        assertEq(v.balanceOf(address(gV)), 512_000 ether, "V sitting in gV");

        tre.collateralizeOliver(type(uint256).max);
        assertEq(oliver.collatV(address(tre)), 512_000 ether, "then Oliver collateral");
        assertEq(tre.stakedGv(), 0);

        vm.expectRevert(GenesisTreasury.NoMarketSell.selector);
        tre.withdrawV(address(this), 1 ether);
        vm.expectRevert(GenesisTreasury.NoMarketSell.selector);
        tre.claimV(address(this), 1 ether);
        vm.expectRevert(GenesisTreasury.NoMarketSell.selector);
        tre.unstakeToWallet(address(this), 1 ether);

        vm.prank(drawer);
        uint256 paid = tre.drawPusd(1_000 ether);
        assertEq(paid, 1_000 ether);
        assertEq(pusd.balanceOf(drawer), 1_000 ether);
        assertEq(v.balanceOf(drawer), 0, "PUSD-only");
    }
}
