// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../PusdLoop.sol";
import "../HouseFeeRemit.sol";
import "../FeeAttribution.sol";
import "../SavingsRouter.sol";

/// Runs the real mint / loan / fee / savings contracts together at a flat V price.
/// No Fed mint, BrowserStream refill, new bond buyers, or mocked branch remittances.
contract EarningsEngineTest is Test {
    PusdMarket market;
    PusdLoop oliver;
    HouseFeeRemit house;
    RunwayFloor runway;
    RemittanceSink sink;
    FeeAttribution attribution;
    SPUSD liquid;
    SpusdCd cd;
    SavingsRouter router;
    PusdToken pusd;
    address supplier = address(0x5110);
    address borrower = address(0xB02B);

    function setUp() public {
        market = new PusdMarket(1 ether);
        pusd = market.pusd();
        oliver = new PusdLoop(address(market));
        runway = new RunwayFloor(100 ether);
        sink = new RemittanceSink(address(pusd), address(runway));
        attribution = new FeeAttribution(address(pusd), address(sink));
        house = new HouseFeeRemit(address(pusd));
        liquid = new SPUSD(address(pusd));
        cd = new SpusdCd(address(pusd), 500, 200, 30 days);
        router = new SavingsRouter(address(sink), address(liquid), address(cd));
        sink.setForward(address(router));
        router.setAllocation(true, 2500);
        market.setRemittance(address(attribution), address(runway), false);
        oliver.setRemittance(address(attribution), address(runway), false);
        house.setRemittance(address(attribution));
        attribution.register(address(market), FeeAttribution.Source.Lithe);
        attribution.register(address(oliver), FeeAttribution.Source.Oliver);
        attribution.register(address(house), FeeAttribution.Source.House);

        market.vapurr().transfer(supplier, 200_000 ether);
        market.vapurr().transfer(borrower, 30_000 ether);
        vm.startPrank(supplier);
        market.swapVToPusd(100_000 ether);
        pusd.approve(address(liquid), type(uint256).max);
        liquid.deposit(1_000 ether, supplier);
        pusd.approve(address(cd), type(uint256).max);
        cd.open(1_000 ether);
        pusd.approve(address(oliver), type(uint256).max);
        oliver.supply(20_000 ether);
        pusd.approve(address(house), type(uint256).max);
        house.creditFees(300 ether);
        vm.stopPrank();
        vm.startPrank(borrower);
        market.vapurr().approve(address(oliver), type(uint256).max);
        oliver.depositV(30_000 ether);
        oliver.borrow(5_000 ether);
        pusd.approve(address(oliver), type(uint256).max);
        vm.stopPrank();
    }

    function test_flat_price_realized_branches_fund_both_savings_legs() public {
        uint256 vSupply = market.vapurr().totalSupply();
        uint256 housePaid = house.remitSurplus(0);
        uint256 lithePaid = market.remitSurplus(0);
        // No Lithe reserve remains to subsidize the following loan accrual interval.
        assertEq(market.yieldReserve(), 0);
        vm.warp(block.timestamp + 30 days);
        oliver.accrue();
        assertGt(oliver.pendingReserve(), 0);
        assertEq(oliver.realizedRemittable(), 0);
        vm.prank(borrower);
        oliver.repay(200 ether);
        uint256 claimBefore = oliver.snapshot(supplier).supplied;
        uint256 oliverPaid = oliver.remitReserve(type(uint256).max / 4);
        assertGt(oliverPaid, 0);
        assertApproxEqAbs(oliver.snapshot(supplier).supplied, claimBefore, 2);
        (uint256 h, uint256 l, uint256 o, uint256 total) = attribution.breakdown();
        assertEq(h, housePaid);
        assertEq(l, lithePaid);
        assertEq(o, oliverPaid);
        assertEq(total, h + l + o);
        uint256 pSupply = pusd.totalSupply();
        uint256 shares = liquid.totalSupply();
        uint256 sent = sink.forwardSurplus(0);
        assertEq(sink.accountedRfv(), 100 ether);
        assertEq(router.totalReceived(), sent);
        assertEq(router.totalLiquid() + router.totalCd(), sent);
        assertEq(liquid.totalSupply(), shares);
        assertGt(liquid.convertToAssets(liquid.balanceOf(supplier)), 1_000 ether);
        assertEq(cd.totalPrincipal(), 1_000 ether);
        vm.prank(supplier);
        (uint256 principal, uint256 coupon,) = cd.close(1);
        assertEq(principal, 1_000 ether);
        assertEq(coupon, 50 ether);
        assertEq(pusd.totalSupply(), pSupply);
        assertEq(market.vapurr().totalSupply(), vSupply);
    }

    function test_rebased_pusd_received_balances_bound_cd_claims() public {
        vm.warp(block.timestamp + 1 days);
        market.accrue();
        assertGt(pusd.index(), 1 ether);
        vm.startPrank(supplier);
        uint256 beforeCash = pusd.balanceOf(address(cd));
        uint256 id = cd.open(1_000 ether + 17);
        (, uint256 principal,,) = cd.positions(id);
        assertEq(principal, pusd.balanceOf(address(cd)) - beforeCash);
        beforeCash = pusd.balanceOf(address(cd));
        uint256 beforeSurplus = cd.surplus();
        pusd.approve(address(cd), type(uint256).max);
        cd.receiveRemittance(100 ether + 17);
        assertEq(cd.surplus() - beforeSurplus, pusd.balanceOf(address(cd)) - beforeCash);
        vm.stopPrank();
        assertGe(pusd.balanceOf(address(cd)), cd.totalPrincipal() + cd.availableSurplus());
    }
}
