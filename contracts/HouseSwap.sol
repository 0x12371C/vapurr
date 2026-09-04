// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Exact-in swap on the house Uniswap v4 $VAPURR / $PUSD pool only.

interface IERC20 {
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

interface IMarket {
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
}

struct PoolKey {
    address currency0;
    address currency1;
    uint24 fee;
    int24 tickSpacing;
    address hooks;
}

struct SwapParams {
    bool zeroForOne;
    int256 amountSpecified;
    uint160 sqrtPriceLimitX96;
}

interface IPoolManager {
    function unlock(bytes calldata data) external returns (bytes memory);
    function swap(PoolKey memory key, SwapParams memory params, bytes calldata hookData) external returns (int256 delta);
    function sync(address currency) external;
    function settle() external payable returns (uint256);
    function take(address currency, address to, uint256 amount) external;
}

contract HouseSwap {
    uint160 internal constant MIN_SQRT = 4295128740;
    uint160 internal constant MAX_SQRT = 1461446703485210103287273052203988822378723970341;

    address public immutable owner;
    IPoolManager public immutable pm;
    IERC20 public immutable vapurr;
    IERC20 public immutable pusd;
    uint24 public immutable fee;
    int24 public immutable tickSpacing;

    uint256 private _locked = 1;

    event Swap(address indexed trader, bool sellV, uint256 amountIn, uint256 amountOut);

    modifier lock() {
        require(_locked == 1, "LOCK");
        _locked = 2;
        _;
        _locked = 1;
    }

    constructor(address market_, address pm_, uint24 fee_, int24 tickSpacing_) {
        require(market_ != address(0) && pm_ != address(0), "MKT");
        owner = msg.sender;
        pm = IPoolManager(pm_);
        vapurr = IERC20(IMarket(market_).vapurr());
        pusd = IERC20(IMarket(market_).pusd());
        fee = fee_;
        tickSpacing = tickSpacing_;
    }

    function swapExact(bool sellV, uint256 amountIn, uint256 minOut) external lock returns (uint256 outAmt) {
        require(amountIn > 0, "TINY");
        IERC20 inn = sellV ? vapurr : pusd;
        IERC20 outt = sellV ? pusd : vapurr;
        require(inn.transferFrom(msg.sender, address(this), amountIn), "PULL");
        bytes memory raw = pm.unlock(abi.encode(msg.sender, sellV, amountIn, minOut));
        outAmt = abi.decode(raw, (uint256));
        emit Swap(msg.sender, sellV, amountIn, outAmt);
        uint256 dust = inn.balanceOf(address(this));
        if (dust > 0) require(inn.transfer(msg.sender, dust), "PUSD");
        dust = outt.balanceOf(address(this));
        if (dust > 0) require(outt.transfer(msg.sender, dust), "PUSD");
    }

    function unlockCallback(bytes calldata data) external returns (bytes memory) {
        require(msg.sender == address(pm), "PM");
        (address trader, bool sellV, uint256 amountIn, uint256 minOut) = abi.decode(
            data,
            (address, bool, uint256, uint256)
        );
        PoolKey memory key = _key();
        bool vIs0 = address(vapurr) == key.currency0;
        bool zeroForOne = sellV ? vIs0 : !vIs0;
        SwapParams memory sp = SwapParams({
            zeroForOne: zeroForOne,
            amountSpecified: -int256(amountIn),
            sqrtPriceLimitX96: zeroForOne ? MIN_SQRT : MAX_SQRT
        });
        int256 delta = pm.swap(key, sp, "");
        (int128 d0, int128 d1) = _split(delta);
        uint256 outAmt = _settle(key, d0, d1, trader, zeroForOne);
        require(outAmt >= minOut, "SLIP");
        return abi.encode(outAmt);
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

    function _split(int256 d) internal pure returns (int128 a0, int128 a1) {
        // RHC PoolManager packs amount0 in the high 128 bits.
        assembly {
            a0 := sar(128, d)
            a1 := signextend(15, d)
        }
    }

    function _settle(
        PoolKey memory key,
        int128 d0,
        int128 d1,
        address trader,
        bool zeroForOne
    ) internal returns (uint256 outAmt) {
        if (d0 < 0) {
            uint256 owe = uint256(uint128(-d0));
            pm.sync(key.currency0);
            require(IERC20(key.currency0).transfer(address(pm), owe), "PUSD");
            pm.settle();
        } else if (d0 > 0) {
            uint256 got = uint256(uint128(d0));
            pm.take(key.currency0, trader, got);
            if (!zeroForOne) outAmt = got;
        }
        if (d1 < 0) {
            uint256 owe = uint256(uint128(-d1));
            pm.sync(key.currency1);
            require(IERC20(key.currency1).transfer(address(pm), owe), "PUSD");
            pm.settle();
        } else if (d1 > 0) {
            uint256 got = uint256(uint128(d1));
            pm.take(key.currency1, trader, got);
            if (zeroForOne) outAmt = got;
        }
    }
}
