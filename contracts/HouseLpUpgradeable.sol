// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Initializable} from "./proxy/Initializable.sol";
import {UUPSUpgradeable} from "./proxy/UUPSUpgradeable.sol";

/// Upgradeable House book — UUPS behind ERC1967Proxy.
/// Uniswap v4 concentrated $VAPURR / $PUSD. NFT position to the owner
/// (never held by this contract). No USDG. No ETH. No WETH. No hooks.
/// Matches the shape actually live on 46630 (pre-wgV/HousePairConfig
/// refactor, commit e3b3565) — the live TESTNET_HOUSE predates `ef87976`'s
/// wgV switch, so this carries that live behavior forward under a proxy
/// rather than silently adopting the newer wgV/HousePairConfig design,
/// which docs/econ/TESTNET_ROLLOUT.md §9 still marks "still open".
/// `adopt()` records the already-existing live position (owner-held NFT
/// #2273, minted by the old bare-impl HouseLp) on this proxy's bookkeeping
/// without re-minting or moving any capital — `seed()` stays available for
/// a genuinely new position later, but is not called on deploy here.

interface IERC20 {
    function approve(address, uint256) external returns (bool);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

interface IMarket {
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
    function vapurrRate() external view returns (uint256);
}

interface IPermit2 {
    function approve(address token, address spender, uint160 amount, uint48 expiration) external;
}

interface IPositionManager {
    function initializePool(PoolKey calldata key, uint160 sqrtPriceX96) external payable returns (int24);
    function modifyLiquidities(bytes calldata unlockData, uint256 deadline) external payable;
    function multicall(bytes[] calldata data) external payable returns (bytes[] memory);
    function nextTokenId() external view returns (uint256);
}

struct PoolKey {
    address currency0;
    address currency1;
    uint24 fee;
    int24 tickSpacing;
    address hooks;
}

contract HouseLpUpgradeable is Initializable, UUPSUpgradeable {
    uint8 internal constant MINT_POSITION = 0x02;
    uint8 internal constant SETTLE_PAIR = 0x0d;

    /// Storage layout v1 — do not reorder; append only before __gap.
    address public owner;
    IMarket public market;
    IPositionManager public posm;
    IPermit2 public permit2;
    IERC20 public vapurr;
    IERC20 public pusd;
    uint24 public fee;
    int24 public tickSpacing;

    uint256 public tokenId;
    int24 public tickLower;
    int24 public tickUpper;
    uint128 public liquidity;
    bytes32 public poolId;

    uint256 private _locked;

    /// Reserved for future storage (upgrade-safe).
    uint256[40] private __gap;

    event Seeded(uint256 tokenId, bytes32 poolId, uint256 vapurrAmt, uint256 pusdAmt, uint128 liq);
    event Adopted(uint256 tokenId, bytes32 poolId, uint128 liq);
    event OwnerUpdated(address indexed owner);

    modifier lock() {
        require(_locked == 1, "LOCK");
        _locked = 2;
        _;
        _locked = 1;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address market_,
        address posm_,
        address permit2_,
        uint24 fee_,
        int24 tickSpacing_,
        address owner_
    ) external initializer {
        require(
            market_ != address(0) && posm_ != address(0) && permit2_ != address(0) && owner_ != address(0),
            "MKT"
        );
        require(fee_ > 0 && tickSpacing_ > 0, "FEE");
        owner = owner_;
        market = IMarket(market_);
        posm = IPositionManager(posm_);
        permit2 = IPermit2(permit2_);
        address v = IMarket(market_).vapurr();
        address p = IMarket(market_).pusd();
        require(v != address(0) && p != address(0), "MKT");
        vapurr = IERC20(v);
        pusd = IERC20(p);
        fee = fee_;
        tickSpacing = tickSpacing_;
        _locked = 1;
    }

    function _authorizeUpgrade(address) internal view override {
        require(msg.sender == owner, "OWN");
    }

    /// Surface marker for upgrade proofs (v1).
    function houseLpVersion() external pure returns (uint256) {
        return 1;
    }

    function setOwner(address owner_) external {
        require(msg.sender == owner && owner_ != address(0), "OWN");
        owner = owner_;
        emit OwnerUpdated(owner_);
    }

    /// Record an already-existing owner-held position (e.g. the one minted by
    /// the old bare-impl HouseLp) on this proxy's bookkeeping. Moves no funds,
    /// mints nothing — the NFT stays exactly where it already is (with owner).
    /// Only callable while this proxy has never seeded/adopted a position.
    function adopt(uint256 tokenId_, int24 tickLower_, int24 tickUpper_, uint128 liquidity_, bytes32 poolId_)
        external
        onlyOwner
    {
        require(tokenId == 0 && liquidity == 0, "SET");
        require(liquidity_ > 0, "TINY");
        tokenId = tokenId_;
        tickLower = tickLower_;
        tickUpper = tickUpper_;
        liquidity = liquidity_;
        poolId = poolId_;
        emit Adopted(tokenId_, poolId_, liquidity_);
    }

    function seed(
        uint256 vapurrAmt,
        uint256 pusdAmt,
        int24 tickLower_,
        int24 tickUpper_,
        uint128 liquidity_,
        uint160 sqrtPriceX96
    ) external onlyOwner lock {
        require(vapurrAmt > 0 && pusdAmt > 0 && liquidity_ > 0 && sqrtPriceX96 > 0, "TINY");
        require(tickLower_ < tickUpper_, "TICK");
        require(tickLower_ % tickSpacing == 0 && tickUpper_ % tickSpacing == 0, "TICK");
        _pull(vapurr, vapurrAmt);
        _pull(pusd, pusdAmt);
        _allow(vapurr);
        _allow(pusd);

        PoolKey memory key = _key();
        uint256 a0;
        uint256 a1;
        if (key.currency0 == address(vapurr)) {
            a0 = vapurrAmt;
            a1 = pusdAmt;
        } else {
            a0 = pusdAmt;
            a1 = vapurrAmt;
        }
        require(a0 <= type(uint128).max && a1 <= type(uint128).max, "TINY");

        uint256 idBefore = posm.nextTokenId();
        bytes[] memory calls = new bytes[](2);
        calls[0] = abi.encodeWithSelector(IPositionManager.initializePool.selector, key, sqrtPriceX96);
        bytes memory actions = abi.encodePacked(MINT_POSITION, SETTLE_PAIR);
        bytes[] memory params = new bytes[](2);
        params[0] = abi.encode(
            key,
            tickLower_,
            tickUpper_,
            uint256(liquidity_),
            uint128(a0),
            uint128(a1),
            owner,
            bytes("")
        );
        params[1] = abi.encode(key.currency0, key.currency1);
        calls[1] = abi.encodeWithSelector(
            IPositionManager.modifyLiquidities.selector,
            abi.encode(actions, params),
            block.timestamp + 600
        );
        posm.multicall(calls);

        tokenId = idBefore;
        tickLower = tickLower_;
        tickUpper = tickUpper_;
        liquidity = liquidity_;
        poolId = keccak256(abi.encode(key));
        _sweep(vapurr);
        _sweep(pusd);
        emit Seeded(tokenId, poolId, vapurrAmt, pusdAmt, liquidity_);
    }

    struct Snap {
        uint256 tokenId_;
        bytes32 poolId_;
        int24 tickLower_;
        int24 tickUpper_;
        uint128 liquidity_;
        uint256 vapurrBal;
        uint256 pusdBal;
        uint256 px;
        address vapurrToken;
        address pusdToken;
        address posm_;
        uint24 fee_;
        int24 spacing;
        address owner_;
    }

    function snapshot() external view returns (Snap memory s) {
        s.tokenId_ = tokenId;
        s.poolId_ = poolId;
        s.tickLower_ = tickLower;
        s.tickUpper_ = tickUpper;
        s.liquidity_ = liquidity;
        s.vapurrBal = vapurr.balanceOf(owner);
        s.pusdBal = pusd.balanceOf(owner);
        s.px = market.vapurrRate();
        s.vapurrToken = address(vapurr);
        s.pusdToken = address(pusd);
        s.posm_ = address(posm);
        s.fee_ = fee;
        s.spacing = tickSpacing;
        s.owner_ = owner;
    }

    function _key() internal view returns (PoolKey memory key) {
        address a = address(vapurr);
        address b = address(pusd);
        if (uint160(a) < uint160(b)) {
            key.currency0 = a;
            key.currency1 = b;
        } else {
            key.currency0 = b;
            key.currency1 = a;
        }
        key.fee = fee;
        key.tickSpacing = tickSpacing;
        key.hooks = address(0);
    }

    function _pull(IERC20 t, uint256 amt) internal {
        require(t.transferFrom(msg.sender, address(this), amt), "PULL");
    }

    function _allow(IERC20 t) internal {
        require(t.approve(address(permit2), type(uint256).max), "ALLOW");
        permit2.approve(address(t), address(posm), type(uint160).max, type(uint48).max);
    }

    function _sweep(IERC20 t) internal {
        uint256 b = t.balanceOf(address(this));
        if (b > 0) require(t.transfer(owner, b), "PUSD");
    }
}
