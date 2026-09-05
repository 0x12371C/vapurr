// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../GvFed.sol";
import "../HousePairConfig.sol";
import "../HouseLp.sol";

contract MockMarketRate {
    uint256 public rate = 1e18;
    function vapurrRate() external view returns (uint256) {
        return rate;
    }
}

contract MockPermit2 {
    function approve(address, address, uint160, uint48) external {}
}

contract MockPosm {
    uint256 public id = 1;
    bool public initialized;
    function nextTokenId() external view returns (uint256) {
        return id;
    }
    function initializePool(PoolKey calldata, uint160) external payable returns (int24) {
        initialized = true;
        return 0;
    }
    function modifyLiquidities(bytes calldata, uint256) external payable {}
    function multicall(bytes[] calldata) external payable returns (bytes[] memory out) {
        out = new bytes[](0);
        initialized = true;
        id += 1;
    }
}

/// Minimal ERC20 so HouseLp snapshot/pull hit real balanceOf/transferFrom.
contract MockCash {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    function mint(address to, uint256 amt) external {
        balanceOf[to] += amt;
    }
    function approve(address sp, uint256 amt) external returns (bool) {
        allowance[msg.sender][sp] = amt;
        return true;
    }
    function transfer(address to, uint256 amt) external returns (bool) {
        require(balanceOf[msg.sender] >= amt, "BAL");
        balanceOf[msg.sender] -= amt;
        balanceOf[to] += amt;
        return true;
    }
    function transferFrom(address fr, address to, uint256 amt) external returns (bool) {
        require(balanceOf[fr] >= amt, "BAL");
        uint256 a = allowance[fr][msg.sender];
        require(a >= amt, "ALLOW");
        allowance[fr][msg.sender] = a - amt;
        balanceOf[fr] -= amt;
        balanceOf[to] += amt;
        return true;
    }
}

/// HouseLp equity wiring + PairConfig gate (no live Uni v4 e2e).
contract HouseLpWiringTest is Test {
    VapurrToken internal v;
    RebasePolicy internal policy;
    gVAPURR internal gV;
    wgVAPURR internal wgV;
    MockCash internal pusd;

    HousePairConfig internal cfg;
    MockMarketRate internal market;
    MockPosm internal posm;
    MockPermit2 internal permit2;

    uint24 internal constant FEE = 3000;
    int24 internal constant SPACING = 60;

    function setUp() public {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));
        wgV = new wgVAPURR(address(gV));
        pusd = new MockCash();
        cfg = new HousePairConfig(address(wgV), address(pusd), address(gV));
        market = new MockMarketRate();
        posm = new MockPosm();
        permit2 = new MockPermit2();
    }

    function test_house_lp_wires_wgV_from_pair_config() public {
        HouseLp lp = new HouseLp(address(cfg), address(market), address(posm), address(permit2), FEE, SPACING);
        assertEq(address(lp.wgV()), address(wgV), "equity is wgV");
        assertEq(address(lp.wgV()), cfg.wgV(), "matches config");
        assertTrue(address(lp.wgV()) != address(gV), "not raw gV");
        assertTrue(address(lp.wgV()) != address(v), "not raw V");
        assertEq(address(lp.pusd()), address(pusd), "cash is PUSD");
        assertEq(address(lp.pairConfig()), address(cfg));
    }

    function test_house_lp_snapshot_reports_wgV_token() public {
        HouseLp lp = new HouseLp(address(cfg), address(market), address(posm), address(permit2), FEE, SPACING);
        HouseLp.Snap memory s = lp.snapshot();
        assertEq(s.wgVToken, address(wgV));
        assertEq(s.pusdToken, address(pusd));
        assertEq(s.px, 1e18);
        assertEq(s.owner_, address(this));
    }

    function test_house_lp_rejects_zero_pair_config() public {
        vm.expectRevert(bytes("MKT"));
        new HouseLp(address(0), address(market), address(posm), address(permit2), FEE, SPACING);
    }

    function test_seed_pair_config_gate_before_posm() public {
        HouseLp lp = new HouseLp(address(cfg), address(market), address(posm), address(permit2), FEE, SPACING);
        cfg.requireHousePair(address(wgV), address(pusd));
        // No allowance/balance — transferFrom reverts BAL on MockCash or wgV path.
        // Either way PairConfig already passed; posm must stay cold.
        vm.expectRevert();
        lp.seed(1e18, 1e18, -60, 60, 1, 1 << 96);
        assertFalse(posm.initialized(), "posm not reached on failed pull");
    }

    function test_config_cannot_list_raw_gV_as_wgV() public {
        vm.expectRevert(bytes("WRAP"));
        new HousePairConfig(address(gV), address(pusd), address(gV));
    }
}
