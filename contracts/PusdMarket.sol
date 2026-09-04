// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

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
    /// terra-fork/params.go DefaultBasePool = 1_000_000 SDR
    uint256 public constant BASE_POOL = 1_000_000 * DEC;
    /// terra-fork/params.go DefaultPoolRecoveryPeriod = BlocksPerDay = 14400
    uint256 public constant POOL_RECOVERY_PERIOD = 14400;
    /// terra-fork/params.go DefaultMinStabilitySpread = 2%
    uint256 public constant MIN_STABILITY_SPREAD = 2e16;
    /// Lithe. 9% APY cap. Spread from mint and redeem funds it.
    uint256 public constant MAX_APY_BPS = 900;
    uint256 public constant YEAR = 365 days;
    uint256 public constant GENESIS = 1_000_000 * DEC;

    address public immutable owner;
    VapurrToken public immutable vapurr;
    PusdToken public immutable pusd;

    /// GetLunaExchangeRate(uusd): PUSD per 1 VAPURR, 18 dec. First spot of the block.
    uint256 public lunaRate;
    uint256 public pendingRate;
    uint256 public liveBlock;

    /// terra-fork/keeper GetTerraPoolDelta — signed, SDR/UST units
    int256 public terraPoolDelta;
    uint256 public lastReplenish;

    uint256 public yieldReserve;
    uint256 public lastAccrue;

    event Swap(address indexed trader, bool offerLuna, uint256 offer, uint256 ask, uint256 fee);
    event Feed(uint256 rate);
    event Accrue(uint256 pay, uint256 index);

    modifier onlyOwner() { require(msg.sender == owner, "OWN"); _; }

    constructor(uint256 lunaRate_) {
        require(lunaRate_ > 0, "PRICE");
        owner = msg.sender;
        vapurr = new VapurrToken();
        pusd = new PusdToken();
        lunaRate = lunaRate_;
        pendingRate = lunaRate_;
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

    function _spot() internal {
        if (liveBlock != block.number) {
            if (pendingRate > 0) lunaRate = pendingRate;
            liveBlock = block.number;
        }
        require(lunaRate > 0, "PRICE");
    }

    /// terra-fork/oracle keeper.go GetLunaExchangeRate
    function getLunaExchangeRate(bool luna) public view returns (uint256) {
        if (luna) return DEC;
        return lunaRate;
    }

    /// terra-fork/swap.go ComputeInternalSwap
    /// retAmount = offer.Amount * askRate / offerRate
    function computeInternalSwap(uint256 offerAmt, bool offerLuna, bool askLuna) public view returns (uint256) {
        if (offerLuna == askLuna) return offerAmt;
        uint256 offerRate = getLunaExchangeRate(offerLuna);
        uint256 askRate = getLunaExchangeRate(askLuna);
        uint256 ret = (offerAmt * askRate) / offerRate;
        require(ret > 0, "TINY");
        return ret;
    }

    /// terra-fork/keeper.go ReplenishPools — one EndBlocker step per missed block, capped.
    function replenishPools() internal {
        if (block.number <= lastReplenish) return;
        uint256 n = block.number - lastReplenish;
        lastReplenish = block.number;
        if (terraPoolDelta == 0) return;
        if (n > 256) n = 256;
        int256 period = int256(POOL_RECOVERY_PERIOD);
        for (uint256 i = 0; i < n; i++) {
            terraPoolDelta -= terraPoolDelta / period;
        }
    }

    /// terra-fork/swap.go ComputeSwap (Luna<>Terra branch; one stable so SDR = UST)
    function computeSwap(uint256 offerAmt, bool offerLuna)
        public
        view
        returns (uint256 retAmt, uint256 spread)
    {
        require(offerAmt > 0, "TINY");
        // Swap offer to base denom (UST), then base to ask — swap.go ComputeSwap
        uint256 baseOffer = computeInternalSwap(offerAmt, offerLuna, false);
        retAmt = computeInternalSwap(baseOffer, false, !offerLuna);

        uint256 basePool = BASE_POOL;
        uint256 cp = basePool * basePool;
        int256 terraPoolI = int256(basePool) + terraPoolDelta;
        require(terraPoolI > 0, "THIN");
        uint256 terraPool = uint256(terraPoolI);
        uint256 lunaPool = cp / terraPool;

        uint256 offerPool = offerLuna ? lunaPool : terraPool;
        uint256 askPool = offerLuna ? terraPool : lunaPool;
        uint256 askBaseAmount = askPool - (cp / (offerPool + baseOffer));
        require(baseOffer >= askBaseAmount, "THIN");
        spread = ((baseOffer - askBaseAmount) * DEC) / baseOffer;
        if (spread < MIN_STABILITY_SPREAD) spread = MIN_STABILITY_SPREAD;
    }

    /// terra-fork/swap.go ApplySwapToPool
    function applySwapToPool(bool offerLuna, uint256 offerAmt, uint256 askAmtAfterFee) internal {
        if (offerLuna) {
            // Luna -> Terra: delta -= ask in UST
            uint256 askBase = computeInternalSwap(askAmtAfterFee, false, false);
            terraPoolDelta -= int256(askBase);
        } else {
            // Terra -> Luna: delta += offer in UST
            uint256 offerBase = computeInternalSwap(offerAmt, false, false);
            terraPoolDelta += int256(offerBase);
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
    }

    /// terra-fork/msg_server.go handleSwapRequest — Luna -> UST
    /// Burn VAPURR, mint PUSD at oracle minus spread. Spread -> Lithe reserve.
    function swapLunaToUst(uint256 offer) external returns (uint256 ask, uint256 fee) {
        _spot();
        accrue();
        (uint256 ret, uint256 spread) = computeSwap(offer, true);
        fee = (spread * ret) / DEC;
        ask = ret - fee;
        require(ask > 0, "TINY");
        applySwapToPool(true, offer, ask);
        vapurr.take(msg.sender, offer);
        vapurr.burn(address(this), offer);
        pusd.mint(msg.sender, ask);
        if (fee > 0) {
            pusd.mint(address(this), fee);
            yieldReserve += fee;
        }
        emit Swap(msg.sender, true, offer, ask, fee);
    }

    /// terra-fork/msg_server.go handleSwapRequest — UST -> Luna
    /// Burn PUSD, mint VAPURR at oracle minus spread. V fee is not minted (burned).
    function swapUstToLuna(uint256 offer) external returns (uint256 ask, uint256 fee) {
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
        uint256 terraPool;
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
        s.px = lunaRate;
        s.idx = pusd.index();
        s.vapurrSupply = vapurr.totalSupply();
        s.pusdSupply = pusd.totalSupply();
        s.yieldRes = yieldReserve;
        s.apy = apyBps();
        s.vapurrToken = address(vapurr);
        s.pusdToken = address(pusd);
        int256 tp = int256(BASE_POOL) + terraPoolDelta;
        s.terraPool = tp > 0 ? uint256(tp) : 0;
        s.minSpread = MIN_STABILITY_SPREAD;
    }
}
