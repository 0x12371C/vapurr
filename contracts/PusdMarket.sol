// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";

/// $VAPURR / $PUSD market on Robinhood Chain.
/// Burn offer, mint ask. No USDG in the swap.
/// Lithe: $PUSD index drips at 9%.

contract VapurrToken {
    string public constant name = "VAPURR";
    string public constant symbol = "VAPURR";
    uint8 public constant decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    address public immutable minter;
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor() { minter = msg.sender; }
    modifier onlyMinter() { require(msg.sender == minter, "MINTER"); _; }

    function mint(address to, uint256 amt) external onlyMinter {
        totalSupply += amt;
        balanceOf[to] += amt;
        emit Transfer(address(0), to, amt);
    }

    function burn(address from, uint256 amt) external onlyMinter {
        uint256 b = balanceOf[from];
        require(b >= amt, "VAPURR");
        unchecked { balanceOf[from] = b - amt; totalSupply -= amt; }
        emit Transfer(from, address(0), amt);
    }

    function take(address from, uint256 amt) external onlyMinter { _move(from, minter, amt); }
    function give(address to, uint256 amt) external onlyMinter { _move(minter, to, amt); }
    function transfer(address to, uint256 amt) external returns (bool) { _move(msg.sender, to, amt); return true; }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        emit Approval(msg.sender, spender, amt);
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) {
            require(a >= amt, "ALLOW");
            unchecked { allowance[from][msg.sender] = a - amt; }
        }
        _move(from, to, amt);
        return true;
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 b = balanceOf[from];
        require(b >= amt, "VAPURR");
        unchecked { balanceOf[from] = b - amt; balanceOf[to] += amt; }
        emit Transfer(from, to, amt);
    }
}

/// Lithe: shares * index. 9% cap.
contract PusdToken {
    string public constant name = "Purr USD";
    string public constant symbol = "PUSD";
    uint8 public constant decimals = 18;
    uint256 public constant DEC = 1e18;
    uint256 public index = DEC;
    uint256 public totalShares;
    mapping(address => uint256) public shares;
    mapping(address => mapping(address => uint256)) public allowance;
    address public immutable minter;
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor() { minter = msg.sender; }
    modifier onlyMinter() { require(msg.sender == minter, "MINTER"); _; }

    function totalSupply() public view returns (uint256) { return (totalShares * index) / DEC; }
    function balanceOf(address a) public view returns (uint256) { return (shares[a] * index) / DEC; }

    function mint(address to, uint256 amt) external onlyMinter {
        uint256 s = (amt * DEC) / index;
        require(s > 0, "TINY");
        shares[to] += s;
        totalShares += s;
        emit Transfer(address(0), to, amt);
    }

    function burn(address from, uint256 amt) external onlyMinter {
        uint256 s = (amt * DEC) / index;
        require(s > 0 && shares[from] >= s, "PUSD");
        unchecked { shares[from] -= s; totalShares -= s; }
        emit Transfer(from, address(0), amt);
    }

    function drip(uint256 pay) external onlyMinter {
        uint256 supply = totalSupply();
        if (pay == 0 || supply == 0) return;
        index = (index * (supply + pay)) / supply;
    }

    function transfer(address to, uint256 amt) external returns (bool) { _move(msg.sender, to, amt); return true; }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        emit Approval(msg.sender, spender, amt);
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) {
            require(a >= amt, "ALLOW");
            unchecked { allowance[from][msg.sender] = a - amt; }
        }
        _move(from, to, amt);
        return true;
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 s = (amt * DEC) / index;
        require(s > 0 && shares[from] >= s, "PUSD");
        unchecked { shares[from] -= s; shares[to] += s; }
        emit Transfer(from, to, amt);
    }
}


contract PusdMarket {
    uint256 public constant DEC = 1e18;
    /// stability-pool math (internal).
    uint256 public constant BASE_POOL = 1_000_000 * DEC;
    /// stability-pool math (internal).
    uint256 public constant POOL_RECOVERY_PERIOD = 14400;
    /// stability-pool math (internal).
    uint256 public constant MIN_STABILITY_SPREAD = 2e16;
    /// Lithe. 9% APY cap. Spread from mint and redeem funds it.
    uint256 public constant MAX_APY_BPS = 900;
    uint256 public constant YEAR = 365 days;
    uint256 public constant GENESIS = 1_000_000 * DEC;

    address public immutable owner;
    VapurrToken public immutable vapurr;
    PusdToken public immutable pusd;

    /// getVapurrExchangeRate(pusd): PUSD per 1 VAPURR, 18 dec. First spot of the block.
    uint256 public vapurrRate;
    uint256 public pendingRate;
    uint256 public liveBlock;

    /// stability-pool math (internal).
    int256 public poolDelta;
    uint256 public lastReplenish;

    uint256 public yieldReserve;
    uint256 public lastAccrue;

    IRemittance public remittance; // surplus sink (RemittanceSink / sPUSD)
    IRunwayView public runway; // optional floor gate before remit
    bool public remitOnAccrue; // when true, accrue best-effort pushes surplus above floor to sink

    event Swap(address indexed trader, bool offerV, uint256 offer, uint256 ask, uint256 fee);
    event Feed(uint256 rate);
    event Accrue(uint256 pay, uint256 index);
    event RemittanceSet(address indexed sink, address indexed runway_, bool autoRemit);
    event Remitted(address indexed sink, uint256 assets);

    modifier onlyOwner() { require(msg.sender == owner, "OWN"); _; }

    constructor(uint256 vapurrRate_) {
        require(vapurrRate_ > 0, "PRICE");
        owner = msg.sender;
        vapurr = new VapurrToken();
        pusd = new PusdToken();
        vapurrRate = vapurrRate_;
        pendingRate = vapurrRate_;
        liveBlock = block.number;
        lastReplenish = block.number;
        lastAccrue = block.timestamp;
        vapurr.mint(msg.sender, GENESIS);
    }

    /// Oracle vote. Live rate snapshots on first swap of the block (first-spot).
    function feed(uint256 rate) external onlyOwner {
        require(rate > 0, "PRICE");
        pendingRate = rate;
        emit Feed(rate);
    }

    function setRemittance(address sink, address runway_, bool autoRemit) external onlyOwner {
        remittance = IRemittance(sink);
        runway = IRunwayView(runway_);
        remitOnAccrue = autoRemit;
        emit RemittanceSet(sink, runway_, autoRemit);
    }

    /// Push yieldReserve surplus above runway floor to remittance sink (real $PUSD).
    /// amount==0 means remit all free surplus. Floor may start at 0.
    function remitSurplus(uint256 amount) public returns (uint256 sent) {
        accrue(); // settle holder drip first (Oliver-style)
        sent = _remitSurplus(amount);
    }

    function _remitSurplus(uint256 amount) internal returns (uint256 sent) {
        require(address(remittance) != address(0), "REMIT");
        uint256 free = yieldReserve;
        if (address(runway) != address(0)) {
            free = runway.surplus(yieldReserve);
        }
        sent = amount == 0 ? free : amount;
        if (sent > free) sent = free;
        if (sent > yieldReserve) sent = yieldReserve;
        uint256 cash = pusd.balanceOf(address(this));
        if (sent > cash) sent = cash;
        if (sent == 0) return 0;
        yieldReserve -= sent;
        require(pusd.approve(address(remittance), sent), "ALLOW");
        require(remittance.receiveRemittance(sent), "SINK");
        emit Remitted(address(remittance), sent);
    }

    function _spot() internal {
        if (liveBlock != block.number) {
            if (pendingRate > 0) vapurrRate = pendingRate;
            liveBlock = block.number;
        }
        require(vapurrRate > 0, "PRICE");
    }

    /// stability-pool math (internal).
    function getVapurrExchangeRate(bool isV) public view returns (uint256) {
        if (isV) return DEC;
        return vapurrRate;
    }

    /// stability-pool math (internal).
    /// retAmount = offer.Amount * askRate / offerRate
    function computeInternalSwap(uint256 offerAmt, bool offerV, bool askV) public view returns (uint256) {
        if (offerV == askV) return offerAmt;
        uint256 offerRate = getVapurrExchangeRate(offerV);
        uint256 askRate = getVapurrExchangeRate(askV);
        uint256 ret = (offerAmt * askRate) / offerRate;
        require(ret > 0, "TINY");
        return ret;
    }

    /// stability-pool math (internal).
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

    /// stability-pool math (internal).
    function computeSwap(uint256 offerAmt, bool offerV)
        public
        view
        returns (uint256 retAmt, uint256 spread)
    {
        require(offerAmt > 0, "TINY");
        // Swap offer to stable denom, then base to ask (stability-pool math).
        uint256 baseOffer = computeInternalSwap(offerAmt, offerV, false);
        retAmt = computeInternalSwap(baseOffer, false, !offerV);

        uint256 basePool = BASE_POOL;
        uint256 cp = basePool * basePool;
        int256 stablePoolI = int256(basePool) + poolDelta;
        require(stablePoolI > 0, "THIN");
        uint256 stablePool = uint256(stablePoolI);
        uint256 vapurrPool = cp / stablePool;

        uint256 offerPool = offerV ? vapurrPool : stablePool;
        uint256 askPool = offerV ? stablePool : vapurrPool;
        uint256 askBaseAmount = askPool - (cp / (offerPool + baseOffer));
        require(baseOffer >= askBaseAmount, "THIN");
        spread = ((baseOffer - askBaseAmount) * DEC) / baseOffer;
        if (spread < MIN_STABILITY_SPREAD) spread = MIN_STABILITY_SPREAD;
    }

    /// stability-pool math (internal).
    function applySwapToPool(bool offerV, uint256 offerAmt, uint256 askAmtAfterFee) internal {
        if (offerV) {
            // V -> PUSD: delta -= ask in PUSD
            uint256 askBase = computeInternalSwap(askAmtAfterFee, false, false);
            poolDelta -= int256(askBase);
        } else {
            // PUSD -> V: delta += offer in PUSD
            uint256 offerBase = computeInternalSwap(offerAmt, false, false);
            poolDelta += int256(offerBase);
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
        pusd.drip(pay);
        yieldReserve -= pay;
        emit Accrue(pay, pusd.index());
        // Remittance hook: best-effort push remaining surplus above runway to sink.
        if (remitOnAccrue && address(remittance) != address(0) && yieldReserve > 0) {
            _remitSurplus(0);
        }
    }

    /// V inventory held by this market (pre-funded + V locked on PUSD mint).
    /// ROUTING wall: market redeem MUST NOT mint V — only Fed/gV policy prints V.
    function vInventory() public view returns (uint256) {
        return vapurr.balanceOf(address(this));
    }

    /// Seed / top-up V float for redeem. Does not mint — pulls already-minted V.
    function fundVInventory(uint256 amt) external {
        require(amt > 0, "TINY");
        vapurr.take(msg.sender, amt);
    }

    /// stability-pool math (internal).
    /// Lock VAPURR into inventory (no burn), mint PUSD at oracle minus spread. Spread -> Lithe reserve.
    /// Semantics: burn-unwrap float — V stays in market so redeem can return inventory, not mint.
    function swapVToPusd(uint256 offer) external returns (uint256 ask, uint256 fee) {
        _spot();
        accrue();
        (uint256 ret, uint256 spread) = computeSwap(offer, true);
        fee = (spread * ret) / DEC;
        ask = ret - fee;
        require(ask > 0, "TINY");
        applySwapToPool(true, offer, ask);
        vapurr.take(msg.sender, offer);
        // V stays on market as redeem inventory (was: vapurr.burn). No V supply change.
        pusd.mint(msg.sender, ask);
        if (fee > 0) {
            pusd.mint(address(this), fee);
            yieldReserve += fee;
        }
        emit Swap(msg.sender, true, offer, ask, fee);
    }

    /// stability-pool math (internal).
    /// Burn PUSD, unlock VAPURR from pre-funded inventory at oracle minus spread.
    /// HARD FENCE: does NOT call vapurr.mint — redeem fails if inventory thin (INV).
    /// Fed/gV rebase remains the sole V inflation path; browse/earn cannot unbounded-mint via market.
    function swapPusdToV(uint256 offer) external returns (uint256 ask, uint256 fee) {
        _spot();
        accrue();
        uint256 inv = vapurr.balanceOf(address(this));
        require(inv > 0, "INV"); // empty inventory fails before CP; no mint fallback
        (uint256 ret, uint256 spread) = computeSwap(offer, false);
        fee = (spread * ret) / DEC;
        ask = ret - fee;
        require(ask > 0, "TINY");
        require(inv >= ask, "INV");
        applySwapToPool(false, offer, ask);
        pusd.burn(msg.sender, offer);
        vapurr.give(msg.sender, ask);
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
        int256 tp = int256(BASE_POOL) + poolDelta;
        s.stablePool = tp > 0 ? uint256(tp) : 0;
        s.minSpread = MIN_STABILITY_SPREAD;
    }
}
