// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";
import "../HousePairConfig.sol";
import "../HouseSwap.sol";

contract MockPoolManager {
    function unlock(bytes calldata) external pure returns (bytes memory) {
        return abi.encode(uint256(0));
    }
    function swap(PoolKey memory, SwapParams memory, bytes calldata) external pure returns (int256) {
        return 0;
    }
    function sync(address) external pure {}
    function settle() external payable returns (uint256) {
        return 0;
    }
    function take(address, address, uint256) external pure {}
}

/// HouseSwap equity wiring via PairConfig (no live Uni v4 e2e).
contract HouseSwapWiringTest is Test {
    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    wgVAPURR internal wgV;
    address internal pusd;

    HousePairConfig internal cfg;
    MockPoolManager internal pm;

    uint24 internal constant FEE = 3000;
    int24 internal constant SPACING = 60;

    function setUp() public {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        wgV = new wgVAPURR(address(gV));
        pusd = address(0xBEEF);
        cfg = new HousePairConfig(address(wgV), pusd, address(gV));
        pm = new MockPoolManager();
    }

    function test_house_swap_wires_wgV_from_pair_config() public {
        HouseSwap hs = new HouseSwap(address(cfg), address(pm), FEE, SPACING);
        assertEq(address(hs.wgV()), address(wgV));
        assertEq(address(hs.pusd()), pusd);
        assertEq(address(hs.pairConfig()), address(cfg));
        assertTrue(address(hs.wgV()) != address(gV));
        assertTrue(address(hs.wgV()) != address(v));
    }

    function test_house_swap_rejects_zero_pair_config() public {
        vm.expectRevert(bytes("MKT"));
        new HouseSwap(address(0), address(pm), FEE, SPACING);
    }

    function test_house_swap_rejects_zero_fee() public {
        vm.expectRevert(bytes("FEE"));
        new HouseSwap(address(cfg), address(pm), 0, SPACING);
    }
}
