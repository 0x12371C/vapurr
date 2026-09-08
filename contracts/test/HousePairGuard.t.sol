// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";
import "../HousePairConfig.sol";

/// Proof: raw rebasing gV is rejected as a House pool leg; wgV/$PUSD accepted.
contract HousePairGuardTest is Test {
    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    wgVAPURR internal wgV;
    /// Stand-in cash leg (address-only; PairConfig does not call into PUSD).
    address internal pusd;

    HousePairConfig internal cfg;
    HousePairFactory internal factory;

    function setUp() public {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        wgV = new wgVAPURR(address(gV));
        // Distinct non-zero cash address for pair checks (no PUSD deploy needed).
        pusd = address(0xBEEF);
        cfg = new HousePairConfig(address(wgV), pusd, address(gV));
        factory = new HousePairFactory(address(cfg));
    }

    function test_raw_gV_not_accepted_as_house_equity() public {
        vm.expectRevert(HousePairConfig.RawGvNotHouseEquity.selector);
        cfg.requireHouseEquity(address(gV));
    }

    function test_raw_gV_not_accepted_in_house_pair() public {
        vm.expectRevert(HousePairConfig.RawGvNotHouseEquity.selector);
        cfg.requireHousePair(address(gV), pusd);

        vm.expectRevert(HousePairConfig.RawGvNotHouseEquity.selector);
        cfg.requireHousePair(pusd, address(gV));
    }

    function test_factory_rejects_raw_gV_pool() public {
        vm.expectRevert(HousePairConfig.RawGvNotHouseEquity.selector);
        factory.validateAndMark(address(gV), pusd);
    }

    function test_wgV_pusd_accepted_as_house_pair() public {
        cfg.requireHouseEquity(address(wgV));
        cfg.requireHousePair(address(wgV), pusd);
        cfg.requireHousePair(pusd, address(wgV));
        assertTrue(cfg.isHousePair(address(wgV), pusd));
        assertFalse(cfg.isHousePair(address(gV), pusd));

        bytes32 id = factory.validateAndMark(address(wgV), pusd);
        assertEq(id, keccak256(abi.encode(address(wgV), pusd)));
    }

    function test_vapurr_or_stranger_rejected_as_equity() public {
        vm.expectRevert(bytes("EQUITY"));
        cfg.requireHouseEquity(address(v));

        vm.expectRevert(HousePairConfig.BadHousePair.selector);
        cfg.requireHousePair(address(v), pusd);
    }
}