// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "forge-std/console2.sol";
import {VapurrToken, RebasePolicy, gVAPURR} from "../GvFed.sol";
import {PusdMarketFedUpgradeable} from "../PusdMarketFedUpgradeable.sol";
import {ERC1967Proxy} from "../proxy/ERC1967Proxy.sol";
import {PusdLoop} from "../PusdLoop.sol";
import {BondMarket, BondAssetTag} from "../BondMarket.sol";
import {RunwayFloor, RemittanceSink} from "../Remittance.sol";
import {LaunchBootstrap} from "../LaunchBootstrap.sol";
import {MockUsdg} from "../MockUsdg.sol";
import {SPUSD} from "../SPUSD.sol";
import {SpusdCd} from "../SpusdCd.sol";
import {SavingsRouter} from "../SavingsRouter.sol";
import {LegacyVConverter} from "../LegacyVConverter.sol";
import {ILegacyLitheMarket, LitheCutoverMigrator} from "../LitheCutoverMigrator.sol";

interface ILegacyVSupply {
    function totalSupply() external view returns (uint256);
}

/// Minimal exo ERC20 stand-in for dry-run / testnet when ENV legs are unset.
contract RolloutMockErc20 {
    string public name;
    string public symbol;
    uint8 public constant decimals = 18;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    constructor(string memory name_, string memory symbol_) {
        name = name_;
        symbol = symbol_;
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

/// Dry-run-only legacy Lithe surface so LitheCutoverMigrator can construct without live gen-4 addresses.
contract RolloutMockLegacyMarket {
    address public vapurr;
    address public pusd;

    constructor(address vapurr_, address pusd_) {
        require(vapurr_ != address(0) && pusd_ != address(0), "TO");
        vapurr = vapurr_;
        pusd = pusd_;
    }

    function swapPusdToV(uint256) external pure returns (uint256, uint256) {
        revert("MOCK");
    }
}

/// Non-script executor so custody can use address(this) (forge forbids that on Script).
/// Composes CanonicalLitheFactory order + LaunchBootstrap companion + savings forward + cutover inventory.
contract TestnetRolloutDeploy {
    uint256 public constant DEV_FUND_AMOUNT = 200_000 ether;
    uint256 public constant DRY_RUN_LEGACY_SUPPLY = 1_000_000 ether;

    VapurrToken public v;
    RebasePolicy public policy;
    gVAPURR public gV;
    PusdMarketFedUpgradeable public impl;
    PusdMarketFedUpgradeable public market;
    PusdLoop public oliver;
    BondMarket public bonds;
    RunwayFloor public runway;
    RemittanceSink public sink;
    LaunchBootstrap public boot;
    SPUSD public spusd;
    SpusdCd public spusdCd;
    SavingsRouter public savingsRouter;
    LegacyVConverter public converter;
    LitheCutoverMigrator public migrator;
    address public litheProxy;
    address public usdg;
    address public exoEth;
    address public exoNvda;
    address public exoAmd;
    address public legacyMarket;
    address public legacyV;
    uint256 public legacyVSupply;
    bool public cutoverWired;
    bool public cutoverIsDryRunMock;

    struct Env {
        address usdg;
        address eth;
        address nvda;
        address amd;
        address bondTreasury;
        address recipient;
        uint256 bondCapacity;
        uint256 runwayFloor;
        bool seedPol;
        bool autoRemit;
        address legacyMarket;
        address legacyV;
        uint256 legacyVSupply;
        uint256 cdCouponBps;
        uint256 cdBreakFeeBps;
        uint256 cdTerm;
        bool allowDryRunCutoverMock;
    }

    function execute(uint256 rate, address owner_, uint256 bootstrapV, Env calldata env) external {
        _deployFedLitheOliver(rate);
        _wireBondsRemit(owner_, env);
        _wireSavings(owner_, env);
        _wireCutover(env);
        _genesisBootstrapHandoff(owner_, bootstrapV, env);
    }

    function _deployFedLitheOliver(uint256 rate) internal {
        v = new VapurrToken();
        policy = new RebasePolicy();
        gV = new gVAPURR(address(v), address(policy));
        policy.bindGV(address(gV));

        impl = new PusdMarketFedUpgradeable();
        bytes memory initData =
            abi.encodeCall(PusdMarketFedUpgradeable.initialize, (address(v), rate, address(this)));
        litheProxy = address(new ERC1967Proxy(address(impl), initData));
        market = PusdMarketFedUpgradeable(litheProxy);
        oliver = new PusdLoop(litheProxy);
    }

    function _wireBondsRemit(address owner_, Env calldata env) internal {
        usdg = env.usdg;
        if (usdg == address(0)) usdg = address(new MockUsdg());

        address treasury = env.bondTreasury == address(0) ? owner_ : env.bondTreasury;
        uint256 cap = env.bondCapacity == 0 ? 100_000 ether : env.bondCapacity;
        bonds = new BondMarket(address(gV), address(v));
        bonds.setMarket(BondAssetTag.USDG, usdg, treasury, 1_000, 500, uint64(7 days), cap, 1 ether, true);
        policy.bindBondMarket(address(bonds));

        address pusd = address(market.pusd());
        runway = new RunwayFloor(env.runwayFloor);
        sink = new RemittanceSink(pusd, address(runway));
        market.setRemittance(address(sink), address(runway), env.autoRemit);
        oliver.setRemittance(address(sink), address(runway), env.autoRemit);
    }

    /// Savings path per EARNINGS_ENGINE / SPUSD.md: sink -> SavingsRouter -> SPUSD + SpusdCd.
    /// Starts DISABLED (cdBps=0) so empty liquid vault cannot receive first-depositor windfall.
    function _wireSavings(address owner_, Env calldata env) internal {
        address pusd = address(market.pusd());
        uint256 coupon = env.cdCouponBps == 0 ? 500 : env.cdCouponBps;
        uint256 breakFee = env.cdBreakFeeBps == 0 ? 200 : env.cdBreakFeeBps;
        uint256 term_ = env.cdTerm == 0 ? 30 days : env.cdTerm;
        spusd = new SPUSD(pusd);
        spusdCd = new SpusdCd(pusd, coupon, breakFee, term_);
        savingsRouter = new SavingsRouter(address(sink), address(spusd), address(spusdCd));
        // Contract constructor enables by default; safe rollout default is disabled until seeded.
        savingsRouter.setAllocation(false, 0);
        sink.setForward(address(savingsRouter));
        spusd.setOwner(owner_);
        spusdCd.setOwner(owner_);
        savingsRouter.setOwner(owner_);
    }

    /// Cutover inventory as CanonicalLitheFactory: LegacyVConverter + LitheCutoverMigrator.
    /// Live addresses only via LEGACY_* env (verified). Dry-run may use local mocks — never hardcoded gen-4.
    function _wireCutover(Env calldata env) internal {
        address legacyMarket_ = env.legacyMarket;
        address legacyV_ = env.legacyV;
        uint256 supply_ = env.legacyVSupply;

        if (legacyMarket_ != address(0) || legacyV_ != address(0) || supply_ != 0) {
            require(legacyMarket_ != address(0) && legacyV_ != address(0) && supply_ > 0, "LEGACY_ENV");
            require(ILegacyLitheMarket(legacyMarket_).vapurr() == legacyV_, "LEGACY");
            require(ILegacyVSupply(legacyV_).totalSupply() == supply_, "SUPPLY");
            cutoverIsDryRunMock = false;
        } else if (env.allowDryRunCutoverMock) {
            // Local composition only — not live gen-4 addresses.
            address mockLegacyV = address(new RolloutMockErc20("Legacy V (dry-run)", "lV"));
            address mockLegacyPusd = address(new RolloutMockErc20("Legacy PUSD (dry-run)", "lPUSD"));
            legacyMarket_ = address(new RolloutMockLegacyMarket(mockLegacyV, mockLegacyPusd));
            legacyV_ = mockLegacyV;
            supply_ = DRY_RUN_LEGACY_SUPPLY;
            cutoverIsDryRunMock = true;
        } else {
            // Live confirm path without LEGACY_* — leave cutover for CutoverDeploy / manual.
            cutoverWired = false;
            return;
        }

        converter = new LegacyVConverter(legacyV_, address(v));
        migrator = new LitheCutoverMigrator(legacyMarket_, litheProxy, address(converter));
        legacyMarket = legacyMarket_;
        legacyV = legacyV_;
        legacyVSupply = supply_;
        cutoverWired = true;
    }

    function _genesisBootstrapHandoff(address owner_, uint256 bootstrapV, Env calldata env) internal {
        uint256 cutoverInv = cutoverWired ? legacyVSupply : 0;
        v.mint(address(this), DEV_FUND_AMOUNT + bootstrapV + cutoverInv);

        if (cutoverWired && cutoverInv > 0) {
            require(v.approve(address(converter), cutoverInv), "ALLOW");
            converter.fund(cutoverInv);
        }

        exoEth = env.eth;
        exoNvda = env.nvda;
        exoAmd = env.amd;
        if (exoEth == address(0)) exoEth = address(new RolloutMockErc20("Exo ETH", "eETH"));
        if (exoNvda == address(0)) exoNvda = address(new RolloutMockErc20("Exo NVDA", "eNVDA"));
        if (exoAmd == address(0)) exoAmd = address(new RolloutMockErc20("Exo AMD", "eAMD"));

        address recipient = env.recipient == address(0) ? owner_ : env.recipient;
        address pusd = address(market.pusd());
        boot = new LaunchBootstrap(
            address(v), address(oliver), recipient, usdg, pusd, exoEth, exoNvda, exoAmd, env.seedPol
        );
        require(v.approve(address(boot), DEV_FUND_AMOUNT), "ALLOW");
        boot.fundAndStart();
        if (bootstrapV > 0) {
            require(v.transfer(owner_, bootstrapV), "BOOT");
        }

        v.setMarketMinter(litheProxy);
        v.setMinter(address(gV));

        market.transferOwnership(owner_);
        oliver.setOwner(owner_);
        bonds.setOwner(owner_);
        runway.setOwner(owner_);
        sink.setOwner(owner_);
        boot.registry().setOwner(owner_);
        policy.setOwner(owner_);
    }
}

/// Dry-runable gen-5 testnet rollout (chain 46630).
///
/// SAFETY: does NOT broadcast unless CONFIRM_TESTNET_DEPLOY=1 is set in the env.
/// Default path simulates the full ordered stack locally (no chain write).
///
/// House / wgV: follow-up script TestnetHouseFollowup.s.sol (not this factory).
contract TestnetRollout is Script {
    address constant VANITY = 0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2;
    address constant STATUS_DEPLOYER = 0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5;

    function run() external {
        bool confirm = vm.envOr("CONFIRM_TESTNET_DEPLOY", uint256(0)) == 1;
        uint256 rate = vm.envOr("LITHE_RATE_WAD", uint256(1 ether));
        address owner_ = vm.envOr("ROLLOUT_OWNER", address(0));
        uint256 bootstrapV = vm.envOr("BOOTSTRAP_V", uint256(200_000 ether));

        console2.log("chain planned: 46630 (testnet)");
        console2.log("CONFIRM_TESTNET_DEPLOY", confirm ? uint256(1) : uint256(0));
        console2.log("vanity target:", VANITY);
        console2.log("STATUS deployer:", STATUS_DEPLOYER);

        if (!confirm) {
            console2.log("DRY-RUN only - no broadcast. Set CONFIRM_TESTNET_DEPLOY=1 to enable live deploy.");
            if (owner_ == address(0)) owner_ = msg.sender;
            TestnetRolloutDeploy.Env memory env = _readEnv(owner_, true);
            _plan(rate, owner_, bootstrapV, env);
            TestnetRolloutDeploy dryHelper = new TestnetRolloutDeploy();
            dryHelper.execute(rate, owner_, bootstrapV, env);
            _logDeployed(dryHelper);
            return;
        }

        uint256 pk = vm.envUint("PRIVATE_KEY");
        if (owner_ == address(0)) owner_ = vm.addr(pk);
        // Live path: never invent gen-4 addresses; cutover only if LEGACY_* set.
        TestnetRolloutDeploy.Env memory envLive = _readEnv(owner_, false);

        vm.startBroadcast(pk);
        TestnetRolloutDeploy liveHelper = new TestnetRolloutDeploy();
        liveHelper.execute(rate, owner_, bootstrapV, envLive);
        vm.stopBroadcast();
        _logDeployed(liveHelper);
    }

    function _readEnv(address ownerFallback, bool allowDryRunCutoverMock)
        internal
        view
        returns (TestnetRolloutDeploy.Env memory env)
    {
        env.usdg = vm.envOr("USDG", address(0));
        env.eth = vm.envOr("EXO_ETH", address(0));
        env.nvda = vm.envOr("EXO_NVDA", address(0));
        env.amd = vm.envOr("EXO_AMD", address(0));
        env.bondTreasury = vm.envOr("BOND_TREASURY", ownerFallback);
        env.recipient = vm.envOr("DEVFUND_RECIPIENT", ownerFallback);
        env.bondCapacity = vm.envOr("BOND_USDG_CAPACITY", uint256(100_000 ether));
        env.runwayFloor = vm.envOr("RUNWAY_FLOOR", uint256(0));
        env.seedPol = vm.envOr("SEED_POL", uint256(0)) == 1;
        env.autoRemit = vm.envOr("AUTO_REMIT", uint256(0)) == 1;
        env.legacyMarket = vm.envOr("LEGACY_MARKET", address(0));
        env.legacyV = vm.envOr("LEGACY_V", address(0));
        env.legacyVSupply = vm.envOr("LEGACY_V_SUPPLY", uint256(0));
        env.cdCouponBps = vm.envOr("CD_COUPON_BPS", uint256(500));
        env.cdBreakFeeBps = vm.envOr("CD_BREAK_FEE_BPS", uint256(200));
        env.cdTerm = vm.envOr("CD_TERM", uint256(30 days));
        env.allowDryRunCutoverMock = allowDryRunCutoverMock;
    }

    function _plan(uint256 rate, address owner_, uint256 bootstrapV, TestnetRolloutDeploy.Env memory env)
        internal
        pure
    {
        console2.log("plan owner:", owner_);
        console2.log("plan lithe rate wad:", rate);
        console2.log("plan bootstrap V:", bootstrapV);
        console2.log("plan runway floor:", env.runwayFloor);
        console2.log("plan USDG bond capacity:", env.bondCapacity);
        console2.log("plan seedPol:", env.seedPol ? uint256(1) : uint256(0));
        console2.log("plan CD coupon bps:", env.cdCouponBps);
        console2.log("plan legacy market:", env.legacyMarket);
        console2.log("plan legacy V:", env.legacyV);
        console2.log("plan legacy V supply:", env.legacyVSupply);
        console2.log("ordered steps (IN SCRIPT vs MANUAL) - see docs/econ/TESTNET_ROLLOUT.md:");
        console2.log("  1 [IN SCRIPT] Fed V + RebasePolicy + gV (dynamic 1-9%)");
        console2.log("  2 [IN SCRIPT] Lithe impl + ERC1967Proxy (UUPS) - prefer vanity land");
        console2.log("  3 [IN SCRIPT] Oliver (PusdLoop) behind market proxy");
        console2.log("  4 [IN SCRIPT] BondMarket(gV,V) + USDG BondAssetTag only + policy.bindBondMarket");
        console2.log("  5 [IN SCRIPT] RunwayFloor + RemittanceSink + market/oliver.setRemittance");
        console2.log("     [IN SCRIPT] SavingsRouter + SPUSD + SpusdCd; sink.setForward; starts DISABLED");
        console2.log("  6 [IN SCRIPT] Genesis mint bootstrap + DevFund 200k BEFORE setMinter(gV)");
        console2.log("     [IN SCRIPT] Cutover inventory: LegacyVConverter.fund + LitheCutoverMigrator");
        console2.log("                 dry-run mock if LEGACY_* unset; live requires LEGACY_* (no invented addrs)");
        console2.log("  7 [IN SCRIPT] LaunchBootstrap: DevFundStream start + V/ETH+V/NVDA+V/AMD");
        console2.log("  8 [IN SCRIPT] Dual-minter: setMarketMinter(Lithe) then setMinter(gV)");
        console2.log("     (no Lithe redeem inventory fund; seigniorage mint on swapPusdToV)");
        console2.log("  9 [FOLLOW-UP] House / wgV -> script/TestnetHouseFollowup.s.sol after core");
        console2.log(" 10 [MANUAL]   CutoverDeploy gate + UI address book + migrator fork verify");
        console2.log("HONEST: gen-4 remains live on 46630 until approved cutover.");
    }

    function _logDeployed(TestnetRolloutDeploy h) internal view {
        console2.log("deployed V", address(h.v()));
        console2.log("deployed policy", address(h.policy()));
        console2.log("deployed gV", address(h.gV()));
        console2.log("deployed lithe impl", address(h.impl()));
        console2.log("deployed lithe proxy", h.litheProxy());
        console2.log("deployed oliver", address(h.oliver()));
        console2.log("deployed bonds", address(h.bonds()));
        console2.log("deployed runway", address(h.runway()));
        console2.log("deployed remit sink", address(h.sink()));
        console2.log("deployed SPUSD", address(h.spusd()));
        console2.log("deployed SpusdCd", address(h.spusdCd()));
        console2.log("deployed SavingsRouter", address(h.savingsRouter()));
        console2.log("savings enabled", h.savingsRouter().enabled() ? uint256(1) : uint256(0));
        console2.log("savings cdBps", h.savingsRouter().cdBps());
        console2.log("deployed launch bootstrap", address(h.boot()));
        console2.log("deployed DevFundStream", address(h.boot().devFund()));
        console2.log("deployed exo registry", address(h.boot().registry()));
        console2.log("USDG (bond-only)", h.usdg());
        console2.log("EXO ETH", h.exoEth());
        console2.log("EXO NVDA", h.exoNvda());
        console2.log("EXO AMD", h.exoAmd());
        if (h.cutoverWired()) {
            console2.log("deployed LegacyVConverter", address(h.converter()));
            console2.log("deployed LitheCutoverMigrator", address(h.migrator()));
            console2.log("cutover legacy market", h.legacyMarket());
            console2.log("cutover legacy V", h.legacyV());
            console2.log("cutover inventory", h.legacyVSupply());
            console2.log("cutover dry-run mock", h.cutoverIsDryRunMock() ? uint256(1) : uint256(0));
        } else {
            console2.log("cutover SKIPPED (set LEGACY_MARKET/LEGACY_V/LEGACY_V_SUPPLY for live)");
        }
        console2.log("FOLLOW-UP not deployed: House / wgV (see TestnetHouseFollowup.s.sol)");
        if (h.litheProxy() == VANITY) {
            console2.log("vanity MATCH");
        } else {
            console2.log("vanity MISS - use STATUS deployer nonce-0 CREATE or CREATE2 salt hunt");
        }
    }
}
