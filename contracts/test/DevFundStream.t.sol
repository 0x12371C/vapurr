// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken} from "../GvFed.sol";
import {DevFundStream} from "../DevFundStream.sol";

/// Minimal $PUSD stand-in.
contract MockPusd {
    string public constant name = "PUSD";
    string public constant symbol = "PUSD";
    uint8 public constant decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amt) external {
        totalSupply += amt;
        balanceOf[to] += amt;
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        balanceOf[msg.sender] -= amt;
        balanceOf[to] += amt;
        return true;
    }

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
}

/// Mock Oliver: accepts depositV, borrows PUSD from cash, no withdrawV needed by DevFund.
contract MockOliver {
    address public immutable vapurr;
    address public immutable pusdAddr;
    mapping(address => uint256) public collatV;
    mapping(address => uint256) public debt;

    constructor(address vapurr_, address pusd_) {
        vapurr = vapurr_;
        pusdAddr = pusd_;
    }

    function pusd() external view returns (address) {
        return pusdAddr;
    }

    function depositV(uint256 amt) external {
        require(VapurrToken(vapurr).transferFrom(msg.sender, address(this), amt), "PULL");
        collatV[msg.sender] += amt;
    }

    function borrow(uint256 amt) external {
        require(collatV[msg.sender] > 0, "COLLAT");
        debt[msg.sender] += amt;
        require(MockPusd(pusdAddr).transfer(msg.sender, amt), "PUSD");
    }

    function repay(uint256 amt) external {
        uint256 d = debt[msg.sender];
        if (amt > d) amt = d;
        require(MockPusd(pusdAddr).transferFrom(msg.sender, address(this), amt), "PULL");
        debt[msg.sender] = d - amt;
    }

    /// Exists on real Oliver but DevFund must never expose a path to it.
    function withdrawV(uint256 amt) external {
        collatV[msg.sender] -= amt;
        require(VapurrToken(vapurr).transfer(msg.sender, amt), "V");
    }
}

contract DevFundStreamTest is Test {
    uint256 internal constant YEAR = 365 days;

    VapurrToken internal v;
    MockPusd internal p;
    MockOliver internal oliver;
    DevFundStream internal stream;
    address internal recipient = address(0xDE01);

    function setUp() public {
        v = new VapurrToken();
        p = new MockPusd();
        oliver = new MockOliver(address(v), address(p));
        // Cash for borrows
        p.mint(address(oliver), 1_000_000 ether);

        v.mint(address(this), 1_000_000 ether);
        stream = new DevFundStream(address(v), address(oliver), recipient);
        v.approve(address(stream), type(uint256).max);
        stream.fund(200_000 ether);
        stream.startStream();
    }

    function test_flat_unlock_settles_to_oliver_not_recipient() public {
        vm.warp(block.timestamp + 2 * YEAR);
        uint256 locked = stream.settle();
        assertEq(locked, 100_000 ether);
        assertEq(v.balanceOf(recipient), 0, "no V to recipient");
        assertEq(oliver.collatV(address(stream)), 100_000 ether);
        assertEq(stream.lockedInOliver(), 100_000 ether);
        assertEq(v.balanceOf(address(stream)), 100_000 ether, "unvested remains in stream");
    }

    function test_draw_pusd_only_path() public {
        vm.warp(block.timestamp + YEAR);
        vm.prank(recipient);
        uint256 paid = stream.drawPusd(1_000 ether);
        assertEq(paid, 1_000 ether);
        assertEq(p.balanceOf(recipient), 1_000 ether);
        assertEq(v.balanceOf(recipient), 0, "HARD LOCK: no V transfer");
        assertGt(oliver.collatV(address(stream)), 0);
    }

    function test_withdrawV_and_claimV_revert() public {
        vm.expectRevert(DevFundStream.NoMarketSell.selector);
        stream.withdrawV(recipient, 1 ether);
        vm.expectRevert(DevFundStream.NoMarketSell.selector);
        stream.claimV(recipient, 1 ether);
    }

    function test_expansion_slows_remaining_unlock() public {
        vm.warp(block.timestamp + YEAR);
        // Accrue year-1 at flat supply into Oliver before expansion tip.
        assertEq(stream.vested(), 50_000 ether);
        stream.settle();
        assertEq(stream.accrued(), 50_000 ether);

        v.mint(address(0xB0B0), v.totalSupply());
        assertEq(stream.expansionWad(), 2e18);

        // Another wall-year at 2x expansion unlocks +25k (half rate).
        vm.warp(block.timestamp + YEAR);
        assertEq(stream.vested(), 75_000 ether, "expansion halves unlock rate");
    }

    function test_recipient_frozen_after_start() public {
        vm.expectRevert(DevFundStream.Frozen.selector);
        stream.setRecipient(address(0xDE02));
    }

    function test_stranger_cannot_draw() public {
        vm.warp(block.timestamp + YEAR);
        vm.prank(address(0xBAD1));
        vm.expectRevert(DevFundStream.NotRecipient.selector);
        stream.drawPusd(1 ether);
    }

    function test_settle_does_not_mint() public {
        uint256 supply0 = v.totalSupply();
        vm.warp(block.timestamp + YEAR);
        stream.settle();
        assertEq(v.totalSupply(), supply0);
    }

    function test_constants() public view {
        assertEq(stream.GENESIS_AMOUNT(), 200_000 ether);
        assertEq(stream.BASE_DURATION(), 4 * YEAR);
    }
}
