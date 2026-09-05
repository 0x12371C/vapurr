// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {VapurrToken} from "../GvFed.sol";
import {ExogenousPairRegistry, ExogenousSeedMarket, EXO_TAG_ETH, EXO_TAG_NVDA, EXO_TAG_AMD} from "../ExogenousPairRegistry.sol";

/// Minimal ERC20 for exogenous legs (ETH/NVDA/AMD/USDG stand-ins).
contract MockExo {
    string public name;
    string public symbol;
    uint8 public constant decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    constructor(string memory n, string memory s) {
        name = n;
        symbol = s;
    }

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

contract ExogenousPairRegistryTest is Test {
    VapurrToken internal v;
    MockExo internal eth;
    MockExo internal nvda;
    MockExo internal amd;
    MockExo internal usdg;
    address internal pusd;
    ExogenousPairRegistry internal registry;

    function setUp() public {
        v = new VapurrToken();
        eth = new MockExo("Wrapped Ether", "WETH");
        nvda = new MockExo("NVIDIA", "NVDA");
        amd = new MockExo("AMD", "AMD");
        usdg = new MockExo("USDG", "USDG");
        // Use a concrete non-zero pusd stand-in (0xPUSD is invalid hex literal in older solc â€” use plain).
        pusd = address(0xBEEF);
        registry = new ExogenousPairRegistry(address(v), address(usdg), pusd);

        registry.registerPair(EXO_TAG_ETH, address(eth), true);
        registry.registerPair(EXO_TAG_NVDA, address(nvda), true);
        registry.registerPair(EXO_TAG_AMD, address(amd), true);
    }

    function test_genesis_tags_accept_v_exo_pairs() public {
        registry.requireExogenousPair(EXO_TAG_ETH, address(v), address(eth));
        registry.requireExogenousPair(EXO_TAG_NVDA, address(nvda), address(v));
        registry.requireExogenousPair(EXO_TAG_AMD, address(v), address(amd));

        bytes32 id = registry.validateAndMark(EXO_TAG_ETH, address(v), address(eth));
        assertEq(id, keccak256(abi.encode(EXO_TAG_ETH, address(v), address(eth))));
        assertEq(registry.tagCount(), 3);
    }

    function test_usdg_cannot_be_registered_as_pair() public {
        vm.expectRevert(ExogenousPairRegistry.UsdgNotPairAsset.selector);
        registry.registerPair("USDG", address(usdg), true);
    }

    function test_pusd_cannot_be_registered_as_exo_leg() public {
        vm.expectRevert(ExogenousPairRegistry.PusdNotExogenousPair.selector);
        registry.registerPair("PUSD", pusd, true);
    }

    function test_wrong_assets_rejected() public {
        vm.expectRevert(ExogenousPairRegistry.BadExogenousPair.selector);
        registry.requireExogenousPair(EXO_TAG_ETH, address(v), address(nvda));
    }

    function test_disabled_tag_unknown() public {
        registry.setEnabled(EXO_TAG_ETH, false);
        vm.expectRevert(ExogenousPairRegistry.UnknownTag.selector);
        registry.requireExogenousPair(EXO_TAG_ETH, address(v), address(eth));
    }

    function test_seed_market_pulls_inventory_no_mint() public {
        v.mint(address(this), 10_000 ether);
        eth.mint(address(this), 5 ether);
        uint256 supply0 = v.totalSupply();

        ExogenousSeedMarket seed =
            new ExogenousSeedMarket(address(registry), EXO_TAG_ETH, address(v), address(eth));
        registry.bindPool(EXO_TAG_ETH, address(seed));

        v.approve(address(seed), type(uint256).max);
        eth.approve(address(seed), type(uint256).max);
        seed.seed(1_000 ether, 1 ether);

        (uint256 r0, uint256 r1) = seed.reserves();
        assertEq(r0, 1_000 ether);
        assertEq(r1, 1 ether);
        assertEq(v.totalSupply(), supply0, "seed does not mint V");

        (address exo, address pool, bool enabled) = registry.pairOf(EXO_TAG_ETH);
        assertEq(exo, address(eth));
        assertEq(pool, address(seed));
        assertTrue(enabled);
    }
}
