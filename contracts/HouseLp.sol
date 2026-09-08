// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./HousePairConfig.sol";

/// House book: Uniswap v4 concentrated liquidity.
/// CANON pair = wgV / $PUSD (see docs/econ/HOUSE_PAIR.md + HousePairConfig).
/// Equity immutable is wgV (not market.vapurr / raw gV). Cash leg remains $PUSD.
/// Call HousePairConfig.requireHousePair before initializePool/seed.
/// NFT position to the owner. No USDG. No ETH. No WETH. No hooks.
/// LIVE GAP: full Uni v4 PM/Permit2 e2e + pool-held $PUSD rebase settlement remain P1.

interface IERC20 {
    function approve(address, uint256) external returns (bool);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

interface IMarket {
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

contract HouseLp {
    uint8 internal constant MINT_POSITION = 0x02;
    uint8 internal constant SETTLE_PAIR = 0x0d;

    address public immutable owner;
    IHousePairConfig public immutable pairConfig;
    IMarket public immutable market;
    IPositionManager public immutable posm;
    IPermit2 public immutable permit2;
    /// Equity leg: non-rebasing wgV (never raw gV / market.vapurr).
    IERC20 public immutable wgV;
    /// Cash leg: $PUSD (still rebasing via Lithe index — pool rebase accounting is P1).
    IERC20 public immutable pusd;
    uint24 public immutable fee;
    int24 public immutable tickSpacing;

    uint256 public tokenId;
    int24 public tickLower;
    int24 public tickUpper;
    uint128 public liquidity;
    bytes32 public poolId;

    uint256 private _locked = 1;

    event Seeded(uint256 tokenId, bytes32 poolId, uint256 wgVAmt, uint256 pusdAmt, uint128 liq);

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

    constructor(
        address pairConfig_,
        address market_,
        address posm_,
        address permit2_,
        uint24 fee_,
        int24 tickSpacing_
    ) {
        require(
            pairConfig_ != address(0) && market_ != address(0) && posm_ != address(0) && permit2_ != address(0),
            "MKT"
        );
        require(fee_ > 0 && tickSpacing_ > 0, "FEE");
        owner = msg.sender;
        pairConfig = IHousePairConfig(pairConfig_);
        market = IMarket(market_);
        posm = IPositionManager(posm_);
        permit2 = IPermit2(permit2_);
        address equity = IHousePairConfig(pairConfig_).wgV();
        address cash = IHousePairConfig(pairConfig_).pusd();
        IHousePairConfig(pairConfig_).requireHousePair(equity, cash);
        wgV = IERC20(equity);
        pusd = IERC20(cash);
        fee = fee_;
        tickSpacing = tickSpacing_;
    }

    function seed(
        uint256 wgVAmt,
        uint256 pusdAmt,
        int24 tickLower_,
        int24 tickUpper_,
        uint128 liquidity_,
        uint160 sqrtPriceX96
    ) external onlyOwner lock {
        require(wgVAmt > 0 && pusdAmt > 0 && liquidity_ > 0 && sqrtPriceX96 > 0, "TINY");
        require(tickLower_ < tickUpper_, "TICK");
        require(tickLower_ % tickSpacing == 0 && tickUpper_ % tickSpacing == 0, "TICK");

        PoolKey memory key = _key();
        // Gate before any PositionManager initializePool / mint.
        pairConfig.requireHousePair(key.currency0, key.currency1);

        _pull(wgV, wgVAmt);
        _pull(pusd, pusdAmt);
        _allow(wgV);
        _allow(pusd);

        uint256 a0;
        uint256 a1;
        if (key.currency0 == address(wgV)) {
            a0 = wgVAmt;
            a1 = pusdAmt;
        } else {
            a0 = pusdAmt;
            a1 = wgVAmt;
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
        _sweep(wgV);
        _sweep(pusd);
        emit Seeded(tokenId, poolId, wgVAmt, pusdAmt, liquidity_);
    }

    struct Snap {
        uint256 tokenId_;
        bytes32 poolId_;
        int24 tickLower_;
        int24 tickUpper_;
        uint128 liquidity_;
        uint256 wgVBal;
        uint256 pusdBal;
        uint256 px;
        address wgVToken;
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
        s.wgVBal = wgV.balanceOf(owner);
        s.pusdBal = pusd.balanceOf(owner);
        s.px = market.vapurrRate();
        s.wgVToken = address(wgV);
        s.pusdToken = address(pusd);
        s.posm_ = address(posm);
        s.fee_ = fee;
        s.spacing = tickSpacing;
        s.owner_ = owner;
    }

    function _key() internal view returns (PoolKey memory key) {
        address a = address(wgV);
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
