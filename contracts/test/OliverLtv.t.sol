// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../PusdMarket.sol";
import "../PusdLoop.sol";
import "../Remittance.sol";

/// Reverting remittance sink — proves auto-remit must not freeze repay/withdraw/liq.
contract RevertingRemit is IRemittance {
    function receiveRemittance(uint256) external pure returns (bool) {
        revert("SINK_DOWN");
    }
}

/// Oliver (PusdLoop) LTV trust proofs.
/// FAIL before fix / PASS after: mid-transfer collateral inflation, cash drain past LTV,
/// remitReserve without remaining-LTV, auto-remit revert isolation.
contract OliverLtvTest is Test {
    PusdMarket internal market;
    VapurrToken internal vapurr;
    PusdToken internal pusd;
    PusdLoop internal loop;

    address internal supplier = address(0x5110);

    uint256 internal constant PRICE = 1e18;

    function setUp() public {
        market = new PusdMarket(PRICE);
        vapurr = market.vapurr();
        pusd = market.pusd();
        loop = new PusdLoop(address(market));

        // Fund supplier with PUSD via market mint path
        vapurr.transfer(supplier, 200_000 ether);
        vm.startPrank(supplier);
        vapurr.approve(address(market), type(uint256).max);
        market.swapVToPusd(100_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vapurr.approve(address(loop), type(uint256).max);
        vm.stopPrank();
    }

    /// Attack 1+2: sole supplier borrows ~all cash via mid-transfer LTV inflation.
    /// Post-fix: borrow of full cash reverts LTV.
    function test_sole_supplier_cannot_borrow_past_ltv_cash() public {
        uint256 supplied = 10_000 ether;
        vm.startPrank(supplier);
        loop.supply(supplied);

        uint256 cash = pusd.balanceOf(address(loop));
        assertEq(cash, supplied, "vault cash = supply");

        // Pre-fix this succeeds (~100% util, undercollateralized after transfer).
        // Post-fix must revert LTV.
        vm.expectRevert(bytes("LTV"));
        loop.borrow(cash);
        vm.stopPrank();
    }

    function test_sole_supplier_max_borrow_respects_85_ltv() public {
        uint256 supplied = 10_000 ether;
        vm.startPrank(supplier);
        loop.supply(supplied);

        // 85% of supplied is the policy max when supply is sole collateral.
        uint256 maxOk = (supplied * 8500) / 10_000;
        loop.borrow(maxOk);

        PusdLoop.Snap memory s = loop.snapshot(supplier);
        assertLe(s.debt * 10_000, s.collatValue * loop.LTV_BPS(), "debt within LTV");
        assertGe(s.cash, supplied - maxOk, "cash remains");
        assertGt(s.cash, 0, "cash not zero");
        // Utilization must not exceed LTV policy for sole-supply book
        assertLe(s.totalBorrowAssets_ * 10_000, s.totalSupplyAssets * loop.LTV_BPS(), "util within LTV");
        vm.stopPrank();
    }

    /// Attack: withdraw checks LTV before cash leaves, inflating remaining supply collateral.
    function test_withdraw_ltv_uses_post_cash_state() public {
        uint256 supplied = 10_000 ether;
        vm.startPrank(supplier);
        loop.supply(supplied);
        // Borrow near limit against supply collateral
        uint256 debt = (supplied * 8000) / 10_000; // 80% — room under 85%
        loop.borrow(debt);

        // Withdraw that would leave debt > 85% of remaining collateral must revert.
        vm.expectRevert(bytes("LTV"));
        loop.withdraw(1_000 ether);
        vm.stopPrank();
    }

    /// Attack 3: remitReserve burns owner supply without remaining-LTV check.
    function test_remit_reserve_respects_owner_ltv() public {
        // Owner is address(this). Seed owner supply + debt against it.
        vapurr.approve(address(market), type(uint256).max);
        (uint256 ask,) = market.swapVToPusd(20_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        loop.supply(ask);

        uint256 ownerSupply = ask;
        uint256 debt = (ownerSupply * 8000) / 10_000;
        loop.borrow(debt);

        RunwayFloor runway = new RunwayFloor(0);
        RemittanceSink sink = new RemittanceSink(address(pusd), address(runway));
        loop.setRemittance(address(sink), address(runway), false);

        // Remitting a large slice of owner supply would drop collat under debt*LTV.
        // Pre-fix: succeeds and leaves owner undercollateralized.
        // Post-fix: reverts LTV.
        vm.expectRevert(bytes("LTV"));
        loop.remitReserve(ownerSupply / 2);
    }

    /// Prefer: auto-remit revert must not freeze repay.
    function test_auto_remit_revert_does_not_freeze_repay() public {
        vm.startPrank(supplier);
        loop.supply(20_000 ether);
        vm.stopPrank();

        address borrower = address(0xB02B);
        vapurr.transfer(borrower, 10_000 ether);
        vm.startPrank(borrower);
        vapurr.approve(address(loop), type(uint256).max);
        loop.depositV(10_000 ether);
        loop.borrow(1_000 ether);
        pusd.approve(address(loop), type(uint256).max);
        vm.stopPrank();

        RevertingRemit bad = new RevertingRemit();
        RunwayFloor runway = new RunwayFloor(0);
        loop.setRemittance(address(bad), address(runway), true);

        vm.warp(block.timestamp + 30 days);
        // Accrue must succeed even though auto-remit sink reverts
        loop.accrue();

        // Repay must still work
        vm.prank(borrower);
        loop.repay(100 ether);
    }
}
