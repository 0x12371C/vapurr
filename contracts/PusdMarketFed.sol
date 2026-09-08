// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {PusdToken} from "./PusdMarket.sol";
import "./Remittance.sol";

/// Canonical V seigniorage surface used by cutover Lithe.
/// Market must hold `marketMinter` on Fed V (set via `setMarketMinter` before policy handoff).
interface IVapurrSeigniorage {
    function balanceOf(address) external view returns (uint256);
    function totalSupply() external view returns (uint256);
    function mint(address to, uint256 amt) external;
    function burn(address from, uint256 amt) external;
}

/// Canonical Lithe PUSD market for the one-token cutover — Terra-style seigniorage.
///
/// Receives an already deployed Fed V address. Expand burns V / mints PUSD; redeem
/// burns PUSD / mints V. PUSD remains market-minted. Factory assigns this contract
/// as `marketMinter` on Fed V; gV remains policy minter for staker rebase.
contract PusdMarketFed {
    uint256 public constant DEC = 1e18;
    uint256 public constant BASE_POOL = 1_000_000 * DEC;
    uint256 public constant POOL_RECOVERY_PERIOD = 14400;
    uint256 public constant MIN_STABILITY_SPREAD = 2e16;
    uint256 public constant MAX_APY_BPS = 900;
    uint256 public constant YEAR = 365 days;
    uint256 public constant MAX_FEED_JUMP_WAD = 5e17;

    address public immutable owner;
    IVapurrSeigniorage public immutable vapurr;
    PusdToken public immutable pusd;

    uint256 public vapurrRate;
    uint256 public pendingRate;
    uint256 public liveBlock;
    uint256 public rateUpdatedAt;
    int256 public poolDelta;
    uint256 public lastReplenish;
    uint256 public yieldReserve;
    uint256 public lastAccrue;

    IRemittance public remittance;
    IRunwayView public runway;
    bool public remitOnAccrue;

    event Swap(address indexed trader, bool offerV, uint256 offer, uint256 ask, uint256 fee);
    event Feed(uint256 rate);
    event Accrue(uint256 pay, uint256 index);
    event RemittanceSet(address indexed sink, address indexed runway_, bool autoRemit);
    event Remitted(address indexed sink, uint256 assets);
    event VInventoryFunded(address indexed from, uint256 assets);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address vapurr_, uint256 vapurrRate_, address owner_) {
        require(vapurr_ != address(0) && vapurrRate_ > 0 && owner_ != address(0), "PRICE");
        owner = owner_;
        vapurr = IVapurrSeigniorage(vapurr_);
        pusd = new PusdToken();
        vapurrRate = vapurrRate_;
        pendingRate = vapurrRate_;
        liveBlock = block.number;
        rateUpdatedAt = block.timestamp;
        lastReplenish = block.number;
        lastAccrue = block.timestamp;
    }

    function feed(uint256 rate) external onlyOwner {
        require(rate > 0, "PRICE");
        uint256 hi = vapurrRate + (vapurrRate * MAX_FEED_JUMP_WAD) / DEC;
        uint256 lo = vapurrRate - (vapurrRate * MAX_FEED_JUMP_WAD) / DEC;
        require(rate <= hi && rate >= lo, "JUMP");
        pendingRate = rate;
        rateUpdatedAt = block.timestamp;
        emit Feed(rate);
    }

    function creditVapurrRate(uint256 maxAge) external view returns (uint256) {
        require(maxAge > 0, "AGE");
        require(rateUpdatedAt > 0 && block.timestamp >= rateUpdatedAt, "STALE");
        require(block.timestamp - rateUpdatedAt <= maxAge, "STALE");
        uint256 px = vapurrRate;
        if (pendingRate > 0 && pendingRate < px) px = pendingRate;
        require(px > 0, "PRICE");
        return px;
    }

    function setRemittance(address sink, address runway_, bool autoRemit) external onlyOwner {
        remittance = IRemittance(sink);
        runway = IRunwayView(runway_);
        remitOnAccrue = autoRemit;
        emit RemittanceSet(sink, runway_, autoRemit);
    }

    function remitSurplus(uint256 amount) public returns (uint256 sent) {
        accrue();
        sent = _remitSurplus(amount);
    }

    function _remitSurplus(uint256 amount) internal returns (uint256 sent) {
        require(address(remittance) != address(0), "REMIT");
        uint256 free = yieldReserve;
        sent = amount == 0 ? free : amount;
        if (sent > free) sent = free;
        uint256 cash = pusd.balanceOf(address(this));
        if (sent > cash) sent = cash;
        if (sent == 0) return 0;
        yieldReserve -= sent;
        require(pusd.approve(address(remittance), sent), "ALLOW");
        require(remittance.receiveRemittance(sent), "SINK");
        uint256 cashLeft = pusd.balanceOf(address(this));
        if (yieldReserve > cashLeft) yieldReserve = cashLeft;
        if (amount == 0 && cashLeft > 0 && cashLeft < 1e3) {
            pusd.burnAll(address(this));
            yieldReserve = 0;
        }
        emit Remitted(address(remittance), sent);
    }

    function _spot() internal {
        if (liveBlock != block.number) {
            if (pendingRate > 0) vapurrRate = pendingRate;
            liveBlock = block.number;
            // Do NOT refresh rateUpdatedAt here — only owner feed() heartbeats.
            // Swap applying pending must not launder a stale oracle past Oliver STALE.
        }
        require(vapurrRate > 0, "PRICE");
    }

    function getVapurrExchangeRate(bool isV) public view returns (uint256) {
        return isV ? DEC : vapurrRate;
    }

    function computeInternalSwap(uint256 offerAmt, bool offerV, bool askV) public view returns (uint256) {
        if (offerV == askV) return offerAmt;
        uint256 offerRate = getVapurrExchangeRate(offerV);
        uint256 askRate = getVapurrExchangeRate(askV);
        uint256 ret = (offerAmt * askRate) / offerRate;
        require(ret > 0, "TINY");
        return ret;
    }

    function replenishPools() internal {
        if (block.number <= lastReplenish) return;
        uint256 n = block.number - lastReplenish;
        lastReplenish = block.number;
        if (poolDelta == 0) return;
        if (n > 256) n = 256;
        int256 period = int256(POOL_RECOVERY_PERIOD);
        for (uint256 i = 0; i < n; i++) {
            poolDelta -= poolDelta / period;
        }
    }

    function computeSwap(uint256 offerAmt, bool offerV) public view returns (uint256 retAmt, uint256 spread) {
        require(offerAmt > 0, "TINY");
        uint256 baseOffer = computeInternalSwap(offerAmt, offerV, false);
        retAmt = computeInternalSwap(baseOffer, false, !offerV);
        uint256 cp = BASE_POOL * BASE_POOL;
        int256 stablePoolI = int256(BASE_POOL) + poolDelta;
        require(stablePoolI > 0, "THIN");
        uint256 stablePool = uint256(stablePoolI);
        uint256 vapurrPool = cp / stablePool;
        uint256 offerPool = offerV ? vapurrPool : stablePool;
        uint256 askPool = offerV ? stablePool : vapurrPool;
        uint256 askBaseAmount = askPool - (cp / (offerPool + baseOffer));
        if (baseOffer > askBaseAmount) spread = ((baseOffer - askBaseAmount) * DEC) / baseOffer;
        if (spread < MIN_STABILITY_SPREAD) spread = MIN_STABILITY_SPREAD;
    }

    function applySwapToPool(bool offerV, uint256 offerAmt, uint256 askAmtAfterFee) internal {
        if (offerV) {
            poolDelta -= int256(computeInternalSwap(askAmtAfterFee, false, false));
        } else {
            poolDelta += int256(computeInternalSwap(offerAmt, false, false));
        }
    }

    function accrue() public {
        replenishPools();
        uint256 dt = block.timestamp - lastAccrue;
        lastAccrue = block.timestamp;
        uint256 supply = pusd.totalSupply();
        if (dt == 0 || supply == 0 || yieldReserve == 0) return;
        uint256 maxPay = (supply * MAX_APY_BPS * dt) / 10_000 / YEAR;
        uint256 pay = yieldReserve < maxPay ? yieldReserve : maxPay;
        if (pay == 0) return;
        uint256 cash = pusd.balanceOf(address(this));
        if (cash > 0) {
            uint256 pull = cash < yieldReserve ? cash : yieldReserve;
            if (cash > yieldReserve) pull = cash;
            if (pull > 0) pusd.burn(address(this), pull);
            if (yieldReserve > cash) yieldReserve = cash;
        }
        if (pay > yieldReserve) pay = yieldReserve;
        if (pay == 0) return;
        pusd.drip(pay);
        yieldReserve -= pay;
        if (yieldReserve > 0) pusd.mint(address(this), yieldReserve);
        emit Accrue(pay, pusd.index());
        if (remitOnAccrue && address(remittance) != address(0) && yieldReserve > 0) _remitSurplus(0);
    }

    /// Residual V on market (should be ~0 under seigniorage; not redeem float).
    function vInventory() public view returns (uint256) {
        return vapurr.balanceOf(address(this));
    }

    /// Deprecated under seigniorage — redeem mints V. Retained for ABI compat only.
    function fundVInventory(uint256) external pure {
        revert("SEIGNIORAGE");
    }

    /// Seigniorage expand: burn V, mint PUSD at oracle minus spread.
    function swapVToPusd(uint256 offer) external returns (uint256 ask, uint256 fee) {
        _spot();
        accrue();
        (uint256 ret, uint256 spread) = computeSwap(offer, true);
        fee = (spread * ret) / DEC;
        ask = ret - fee;
        require(ask > 0, "TINY");
        applySwapToPool(true, offer, ask);
        vapurr.burn(msg.sender, offer);
        pusd.mint(msg.sender, ask);
        if (fee > 0) {
            pusd.mint(address(this), fee);
            yieldReserve += fee;
        }
        emit Swap(msg.sender, true, offer, ask, fee);
    }

    /// Seigniorage contract: burn PUSD, mint V at oracle minus spread.
    /// Requires this market to be Fed V `marketMinter`.
    function swapPusdToV(uint256 offer) external returns (uint256 ask, uint256 fee) {
        _spot();
        accrue();
        (uint256 ret, uint256 spread) = computeSwap(offer, false);
        fee = (spread * ret) / DEC;
        ask = ret - fee;
        require(ask > 0, "TINY");
        applySwapToPool(false, offer, ask);
        pusd.burn(msg.sender, offer);
        vapurr.mint(msg.sender, ask);
        emit Swap(msg.sender, false, offer, ask, fee);
    }

    struct Snap {
        uint256 vapurrBal;
        uint256 pusdBal;
        uint256 px;
        uint256 idx;
        uint256 vapurrSupply;
        uint256 pusdSupply;
        uint256 yieldRes;
        uint256 apy;
        address vapurrToken;
        address pusdToken;
        uint256 stablePool;
        uint256 minSpread;
    }

    function apyBps() public view returns (uint256) {
        uint256 supply = pusd.totalSupply();
        if (supply == 0 || yieldReserve == 0) return 0;
        uint256 raw = (yieldReserve * 10_000) / supply;
        return raw > MAX_APY_BPS ? MAX_APY_BPS : raw;
    }

    function snapshot(address a) external view returns (Snap memory s) {
        s.vapurrBal = vapurr.balanceOf(a);
        s.pusdBal = pusd.balanceOf(a);
        s.px = vapurrRate;
        s.idx = pusd.index();
        s.vapurrSupply = vapurr.totalSupply();
        s.pusdSupply = pusd.totalSupply();
        s.yieldRes = yieldReserve;
        s.apy = apyBps();
        s.vapurrToken = address(vapurr);
        s.pusdToken = address(pusd);
        int256 stablePoolI = int256(BASE_POOL) + poolDelta;
        s.stablePool = stablePoolI > 0 ? uint256(stablePoolI) : 0;
        s.minSpread = MIN_STABILITY_SPREAD;
    }
}
