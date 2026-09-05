// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../BondMarket.sol";
import "../GvFed.sol";

contract MockBondAsset is IERC20Bond {
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

/// Inventory payout token that is NOT Fed V — bond pays from this balance only.
contract MockPayoutToken is IERC20Bond {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    uint256 public totalSupply;

    function mint(address to, uint256 amt) external {
        balanceOf[to] += amt;
        totalSupply += amt;
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

contract BondMarketTest is Test {
    VapurrToken internal fedV;
    MockPayoutToken internal payout; // stands in for gV inventory
    MockBondAsset internal usdg;
    address internal treasury = address(0x71E45);
    address internal user = address(0xB0D1);
    BondMarket internal bonds;

    uint256 internal constant PRICE = 1e18; // 1 payout per 1 asset (pre-haircut)

    function setUp() public {
        fedV = new VapurrToken();
        payout = new MockPayoutToken();
        usdg = new MockBondAsset();
        bonds = new BondMarket(address(payout), address(fedV));

        // Seed Fed V and hand minter away — bond must not mint Fed supply.
        fedV.mint(address(this), 1_000_000 ether);
        uint256 supply = fedV.totalSupply();
        fedV.setMinter(address(0));
        assertEq(fedV.totalSupply(), supply);

        // Default market: LIVE — enabled with non-zero capacity (product posture).
        bonds.setMarket(
            BondAssetTag.USDG,
            address(usdg),
            treasury,
            1_000, // 10% discount
            500, // 5% haircut
            7 days,
            100_000 ether, // live capacity
            PRICE,
            true // enabled live-by-default
        );

        usdg.mint(user, 1_000_000 ether);
        vm.prank(user);
        usdg.approve(address(bonds), type(uint256).max);

        // Pre-fund payout inventory (already-minted equity; not Fed mint).
        payout.mint(address(this), 500_000 ether);
        payout.approve(address(bonds), type(uint256).max);
        bonds.fundInventory(200_000 ether);
    }

    function test_owner_disable_killswitch_reverts() public {
        // Happy path is live; owner setEnabled(false) remains the safety killswitch.
        bonds.setEnabled(BondAssetTag.USDG, false);
        vm.prank(user);
        vm.expectRevert(bytes("DISABLED"));
        bonds.bond(BondAssetTag.USDG, 1_000 ether);
    }

    function test_enabled_zero_capacity_reverts() public {
        bonds.setCapacity(BondAssetTag.USDG, 0);
        vm.prank(user);
        vm.expectRevert(bytes("CLOSED"));
        bonds.bond(BondAssetTag.USDG, 1_000 ether);
    }

    function test_enabled_with_inventory_succeeds_without_minting_fed_supply() public {
        uint256 fedSupply0 = fedV.totalSupply();
        uint256 payoutSupply0 = payout.totalSupply();

        // setUp already enabled USDG with capacity — happy path is live.

        // asset 1000, price 1e18 -> gross 1000; haircut 5% -> credit 950;
        // discount 10% -> payout = 950 * 10000 / 9000 = 1055.555... 
        uint256 assetIn = 1_000 ether;
        (uint256 quotedPayout, uint256 credited, uint64 vestSecs, uint16 disc, uint16 hair) =
            bonds.quote(BondAssetTag.USDG, assetIn);
        assertEq(credited, 950 ether);
        uint256 expectedPayout = uint256(950 ether) * 10_000 / 9_000;
        assertEq(quotedPayout, expectedPayout);
        assertEq(uint256(vestSecs), 7 days);
        assertEq(uint256(disc), 1_000);
        assertEq(uint256(hair), 500);

        vm.prank(user);
        uint256 id = bonds.bond(BondAssetTag.USDG, assetIn);

        assertEq(usdg.balanceOf(treasury), assetIn, "asset to RFV sink");
        assertEq(fedV.totalSupply(), fedSupply0, "Fed supply unchanged");
        assertEq(payout.totalSupply(), payoutSupply0, "payout inventory not minted");
        assertEq(bonds.reservedPayout(), quotedPayout);

        vm.warp(block.timestamp + 7 days);
        vm.prank(user);
        uint256 paid = bonds.claim(id);
        assertEq(paid, quotedPayout);
        assertEq(payout.balanceOf(user), quotedPayout);
        assertEq(fedV.totalSupply(), fedSupply0, "claim does not mint Fed V");
    }

    function test_capacity_respected() public {
        // credit(1000) = 950. Leave a positive remainder so the next bond hits CAP (not CLOSED).
        bonds.setCapacity(BondAssetTag.USDG, 950 ether + 100 ether);

        vm.prank(user);
        bonds.bond(BondAssetTag.USDG, 1_000 ether);

        vm.prank(user);
        vm.expectRevert(bytes("CAP"));
        bonds.bond(BondAssetTag.USDG, 1_000 ether);
    }

    function test_haircut_reduces_credited_and_payout() public {
        bonds.setCapacity(BondAssetTag.USDG, 1_000_000 ether);

        (uint256 payoutWithHaircut, uint256 creditWithHaircut,,,) = bonds.quote(BondAssetTag.USDG, 10_000 ether);

        // Remove haircut, keep discount.
        bonds.setValuation(BondAssetTag.USDG, PRICE, 0, 1_000);
        (uint256 payoutNoHaircut, uint256 creditNoHaircut,,,) = bonds.quote(BondAssetTag.USDG, 10_000 ether);

        assertEq(creditNoHaircut, 10_000 ether);
        assertEq(creditWithHaircut, 9_500 ether);
        assertLt(payoutWithHaircut, payoutNoHaircut);
        assertEq(payoutWithHaircut, uint256(9_500 ether) * 10_000 / 9_000);
        assertEq(payoutNoHaircut, uint256(10_000 ether) * 10_000 / 9_000);
    }

    function test_insufficient_inventory_reverts() public {
        // Drain free inventory by setting a huge capacity but tiny funded balance remaining.
        // available = 200k funded; quote for 1M asset ~ payout > 200k after discount.
        bonds.setCapacity(BondAssetTag.USDG, type(uint256).max);

        uint256 huge = 500_000 ether; // credit 475k, payout ~527k > 200k inventory
        vm.prank(user);
        vm.expectRevert(bytes("INV"));
        bonds.bond(BondAssetTag.USDG, huge);
    }

    function test_eth_and_stocks_tags_live_by_default_config() public {
        MockBondAsset weth = new MockBondAsset();
        MockBondAsset stock = new MockBondAsset();
        // Sensible live config for ETH / STOCKS tags (non-zero capacity, enabled).
        bonds.setMarket(BondAssetTag.ETH, address(weth), treasury, 500, 200, 14 days, 50 ether, PRICE, true);
        bonds.setMarket(BondAssetTag.STOCKS, address(stock), treasury, 500, 1_000, 30 days, 50 ether, PRICE, true);

        weth.mint(user, 10 ether);
        stock.mint(user, 10 ether);
        vm.startPrank(user);
        weth.approve(address(bonds), type(uint256).max);
        stock.approve(address(bonds), type(uint256).max);
        uint256 ethId = bonds.bond(BondAssetTag.ETH, 1 ether);
        uint256 stockId = bonds.bond(BondAssetTag.STOCKS, 1 ether);
        vm.stopPrank();
        assertEq(ethId, 1);
        assertEq(stockId, 2);

        // Killswitch still works per-tab.
        bonds.setEnabled(BondAssetTag.ETH, false);
        vm.prank(user);
        vm.expectRevert(bytes("DISABLED"));
        bonds.bond(BondAssetTag.ETH, 1 ether);
    }
}