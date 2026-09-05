// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../SPUSD.sol";
import "../GvFed.sol";

/// Minimal ERC20 for sPUSD vault tests.
contract MockPusd {
    string public constant name = "Mock PUSD";
    string public constant symbol = "mPUSD";
    uint8 public constant decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    function mint(address to, uint256 amt) external {
        totalSupply += amt;
        balanceOf[to] += amt;
        emit Transfer(address(0), to, amt);
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        _move(msg.sender, to, amt);
        return true;
    }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        emit Approval(msg.sender, spender, amt);
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) {
            require(a >= amt, "ALLOW");
            unchecked { allowance[from][msg.sender] = a - amt; }
        }
        _move(from, to, amt);
        return true;
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 b = balanceOf[from];
        require(b >= amt, "BAL");
        unchecked { balanceOf[from] = b - amt; balanceOf[to] += amt; }
        emit Transfer(from, to, amt);
    }
}

/// sPUSD / wgV donation + inflation proofs.
/// FAIL before fix / PASS after: first-depositor donation theft; remittance dust skim.
contract SpusdDonationGuardsTest is Test {
    MockPusd internal asset;
    SPUSD internal vault;

    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    wgVAPURR internal wgV;

    address internal attacker = address(0xA77AC);
    address internal victim = address(0xB1C71);
    address internal staker = address(0x57A);

    function setUp() public {
        asset = new MockPusd();
        vault = new SPUSD(address(asset));

        asset.mint(attacker, 1_000_000 ether);
        asset.mint(victim, 1_000_000 ether);

        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        wgV = new wgVAPURR(address(gV));

        v.mint(staker, 100_000 ether);
        v.mint(attacker, 100_000 ether);
        v.mint(victim, 100_000 ether);
        v.setMinter(address(gV));
    }

    /// Classic ERC-4626 inflation: seed shares, donate huge, next depositor loses NAV.
    function test_donation_cannot_steal_next_depositor() public {
        vm.startPrank(attacker);
        asset.approve(address(vault), type(uint256).max);
        vault.deposit(1 ether, attacker);
        asset.transfer(address(vault), 10_000 ether);
        vm.stopPrank();

        uint256 victimAssets = 10_000 ether;
        vm.startPrank(victim);
        asset.approve(address(vault), type(uint256).max);
        uint256 shares = vault.deposit(victimAssets, victim);
        vm.stopPrank();

        assertGt(shares, 0, "victim must receive shares");
        uint256 victimValue = vault.convertToAssets(shares);
        assertGe(victimValue, (victimAssets * 99) / 100, "victim NAV not stolen");

        // Attacker may reclaim their own donation; must not also drain the victim deposit.
        uint256 attackerShares = vault.balanceOf(attacker);
        vm.prank(attacker);
        uint256 attackerOut = vault.redeem(attackerShares, attacker, attacker);
        uint256 victimLeft = vault.convertToAssets(vault.balanceOf(victim));
        assertGe(victimLeft, (victimAssets * 99) / 100, "victim intact after attacker exit");
        assertLt(attackerOut, 10_000 ether + 1 ether + victimAssets / 2, "attacker cannot take half of victim");
    }

    /// Remittance skim: dust / min deposit then redeem after yield must not capture bulk credit.
    function test_dust_deposit_cannot_skim_remittance() public {
        vm.startPrank(victim);
        asset.approve(address(vault), type(uint256).max);
        vault.deposit(10_000 ether, victim);
        vm.stopPrank();

        vm.startPrank(attacker);
        asset.approve(address(vault), type(uint256).max);
        vm.expectRevert(bytes("TINY"));
        vault.deposit(1, attacker);

        uint256 minIn = vault.MIN_DEPOSIT();
        uint256 shares = vault.deposit(minIn, attacker);
        vm.stopPrank();

        asset.mint(address(this), 5_000 ether);
        asset.approve(address(vault), 5_000 ether);
        vault.receiveRemittance(5_000 ether);

        vm.prank(attacker);
        uint256 out = vault.redeem(shares, attacker, attacker);
        assertLt(out, minIn + 100 ether, "no remittance skim via dust round-trip");
        assertGt(vault.convertToAssets(vault.balanceOf(victim)), 10_000 ether, "victim keeps remittance NAV");
    }

    /// wgV same-pattern donation guard.
    function test_wgV_donation_cannot_steal_next_wrapper() public {
        vm.startPrank(staker);
        v.approve(address(gV), type(uint256).max);
        gV.stake(20_000 ether);
        gV.approve(address(wgV), type(uint256).max);
        wgV.wrap(10_000 ether);
        vm.stopPrank();

        vm.startPrank(attacker);
        v.approve(address(gV), type(uint256).max);
        gV.stake(20_000 ether);
        gV.approve(address(wgV), type(uint256).max);
        wgV.wrap(1 ether);
        gV.transfer(address(wgV), 10_000 ether);
        vm.stopPrank();

        vm.startPrank(victim);
        v.approve(address(gV), type(uint256).max);
        gV.stake(20_000 ether);
        gV.approve(address(wgV), type(uint256).max);
        uint256 shares = wgV.wrap(10_000 ether);
        vm.stopPrank();

        assertGt(shares, 0, "victim wgV shares");
        uint256 pooled = gV.balanceOf(address(wgV));
        uint256 victimGv = (shares * pooled) / wgV.totalSupply();
        assertGe(victimGv, (10_000 ether * 99) / 100, "victim gV claim not stolen");
    }
}
