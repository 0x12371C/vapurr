// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./IVapurrMinter.sol";

/// Fed staking slice: V mint policy, gV rebase, wgV wrapper, BrowserStream earmark.
/// HARD WALL: browse/earn MUST NOT receive or trigger the Fed policy-rate staker mint.
/// Policy rate is dynamic from BondMarket capacity utilization, clamped [1%, 9%]/yr.
/// Rebase model: **index** (shares fixed; balanceOf = shares * index / DEC), matching PusdToken.

/// Fed-side $VAPURR with dual mint authority (`IVapurrMinter`).
/// Policy minter (`minter`): gV rebase inflate to stakers (additional printer).
/// Market minter (`marketMinter`): Lithe seigniorage — mint on PUSD redeem, burn on PUSD expand.
/// Deploy sets policy minter = msg.sender; factory sets marketMinter = Lithe then hands policy to gV.
/// See docs/econ/MINT_AUTHORITY.md.
contract VapurrToken is IVapurrMinter {
    string public constant name = "VAPURR";
    string public constant symbol = "VAPURR";
    uint8 public constant decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    address public minter;
    address public marketMinter;
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event MinterUpdated(address indexed minter);
    event MarketMinterUpdated(address indexed marketMinter);

    constructor() {
        minter = msg.sender;
    }

    modifier onlyPolicyMinter() {
        require(msg.sender == minter, "MINTER");
        _;
    }

    modifier onlyAuthorizedMinter() {
        require(msg.sender == minter || msg.sender == marketMinter, "MINTER");
        _;
    }

    /// Policy mint-role transfer. `m == address(0)` revokes policy minting.
    function setMinter(address m) external onlyPolicyMinter {
        minter = m;
        emit MinterUpdated(m);
    }

    /// Lithe seigniorage mint/burn role. Set before handing policy minter to gV.
    function setMarketMinter(address m) external onlyPolicyMinter {
        marketMinter = m;
        emit MarketMinterUpdated(m);
    }

    function mint(address to, uint256 amt) external onlyAuthorizedMinter {
        require(to != address(0), "TO");
        totalSupply += amt;
        balanceOf[to] += amt;
        emit Transfer(address(0), to, amt);
    }

    function burn(address from, uint256 amt) external onlyAuthorizedMinter {
        uint256 b = balanceOf[from];
        require(b >= amt, "VAPURR");
        unchecked {
            balanceOf[from] = b - amt;
            totalSupply -= amt;
        }
        emit Transfer(from, address(0), amt);
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        _move(msg.sender, to, amt);
        return true;
    }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        emit Approval(msg.sender, spender, amt);
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) {
            require(a >= amt, "ALLOW");
            unchecked {
                allowance[from][msg.sender] = a - amt;
            }
        }
        _move(from, to, amt);
        return true;
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 b = balanceOf[from];
        require(b >= amt, "VAPURR");
        unchecked {
            balanceOf[from] = b - amt;
            balanceOf[to] += amt;
        }
        emit Transfer(from, to, amt);
    }
}

/// Transfer + mint surface used by gV / BrowserStream (stream never holds minter).
/// Authority: policy minter (gV) + optional marketMinter (Lithe seigniorage).
interface IVapurrMint {
    function mint(address to, uint256 amt) external;
    function transfer(address to, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function totalSupply() external view returns (uint256);
    function approve(address spender, uint256 amt) external returns (bool);
}

/// Annualized rebase rate in bps (100 = 1%/yr). Implemented by RebasePolicy.
interface IPolicyRate {
    function policyRateBps() external view returns (uint256);
}

/// Bond-market signal used by RebasePolicy (capacity utilization / idle).
interface IBondMarketSignal {
    function capacityUtilizationWad() external view returns (uint256);
    function hasBondBookSignal() external view returns (bool);
}

/// Staked V with dynamic Fed policy-rate index rebase. Sole V inflation path to stakers.
contract gVAPURR {
    string public constant name = "gVAPURR";
    string public constant symbol = "gVAPURR";
    uint8 public constant decimals = 18;
    uint256 public constant DEC = 1e18;
    /// Clamp band for annualized policy rate (bps). Mid/default when bonds unbound: 350.
    uint256 public constant MIN_REBASE_BPS = 100;
    uint256 public constant MAX_REBASE_BPS = 900;
    uint256 public constant MID_REBASE_BPS = 350;
    uint256 public constant YEAR = 365 days;

    IVapurrMint public immutable vapurr;
    address public policy; // only policy may rebase-mint
    uint256 public index = DEC;
    uint256 public totalShares;
    uint256 public lastRebase;
    mapping(address => uint256) public shares;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Stake(address indexed user, uint256 assets, uint256 sharesOut);
    event Unstake(address indexed user, uint256 assets, uint256 sharesIn);
    event Rebase(uint256 minted, uint256 newIndex, uint256 timestamp);
    event PolicyUpdated(address indexed policy);

    modifier onlyPolicy() {
        require(msg.sender == policy, "POLICY");
        _;
    }

    constructor(address vapurr_, address policy_) {
        require(vapurr_ != address(0) && policy_ != address(0), "TO");
        vapurr = IVapurrMint(vapurr_);
        policy = policy_;
        lastRebase = block.timestamp;
    }

    function setPolicy(address p) external onlyPolicy {
        require(p != address(0), "TO");
        policy = p;
        emit PolicyUpdated(p);
    }

    /// Live annualized rate from RebasePolicy, clamped to [1%, 9%].
    function currentRebaseBps() public view returns (uint256 bps) {
        bps = IPolicyRate(policy).policyRateBps();
        if (bps < MIN_REBASE_BPS) return MIN_REBASE_BPS;
        if (bps > MAX_REBASE_BPS) return MAX_REBASE_BPS;
    }

    function totalSupply() public view returns (uint256) {
        return (totalShares * index) / DEC;
    }

    function balanceOf(address a) public view returns (uint256) {
        return (shares[a] * index) / DEC;
    }

    function stake(uint256 assets) external returns (uint256 sharesOut) {
        require(assets > 0, "TINY");
        // Settle elapsed rebase before minting shares (blocks late-stake / empty-pool theft).
        accrue();
        sharesOut = (assets * DEC) / index;
        require(sharesOut > 0, "TINY");
        require(vapurr.transferFrom(msg.sender, address(this), assets), "PULL");
        shares[msg.sender] += sharesOut;
        totalShares += sharesOut;
        emit Stake(msg.sender, assets, sharesOut);
        emit Transfer(address(0), msg.sender, assets);
    }

    function unstake(uint256 assets) external returns (uint256 sharesIn) {
        require(assets > 0, "TINY");
        accrue();
        sharesIn = (assets * DEC) / index;
        require(sharesIn > 0 && shares[msg.sender] >= sharesIn, "gV");
        unchecked {
            shares[msg.sender] -= sharesIn;
            totalShares -= sharesIn;
        }
        require(vapurr.transfer(msg.sender, assets), "VAPURR");
        emit Unstake(msg.sender, assets, sharesIn);
        emit Transfer(msg.sender, address(0), assets);
    }

    /// Permissionless settle: policy-rate mint into this contract and lift the index.
    /// Rate is dynamic from BondMarket via RebasePolicy (clamp [1%, 9%]/yr).
    /// Called before stake/unstake so new shares never capture unpaid intervals.
    function accrue() public returns (uint256 minted) {
        uint256 dt = block.timestamp - lastRebase;
        lastRebase = block.timestamp;
        uint256 supply = totalSupply();
        if (dt == 0 || supply == 0) {
            emit Rebase(0, index, block.timestamp);
            return 0;
        }
        minted = (supply * currentRebaseBps() * dt) / 10_000 / YEAR;
        if (minted == 0) {
            emit Rebase(0, index, block.timestamp);
            return 0;
        }
        vapurr.mint(address(this), minted);
        index = (index * (supply + minted)) / supply;
        emit Rebase(minted, index, block.timestamp);
    }

    /// Policy-facing alias; browse distributors must never be policy.
    function rebase() external onlyPolicy returns (uint256 minted) {
        return accrue();
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        _move(msg.sender, to, amt);
        return true;
    }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        emit Approval(msg.sender, spender, amt);
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) {
            require(a >= amt, "ALLOW");
            unchecked {
                allowance[from][msg.sender] = a - amt;
            }
        }
        _move(from, to, amt);
        return true;
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 s = (amt * DEC) / index;
        require(s > 0 && shares[from] >= s, "gV");
        unchecked {
            shares[from] -= s;
            shares[to] += s;
        }
        emit Transfer(from, to, amt);
    }
}

/// Non-rebasing wrapper (wstETH pattern) for House AMM pairing.
contract wgVAPURR {
    string public constant name = "Wrapped gVAPURR";
    string public constant symbol = "wgVAPURR";
    uint8 public constant decimals = 18;

    gVAPURR public immutable gV;
    /// Virtual offset (OZ-style) + dead shares on first wrap — donation / inflation resistant.
    uint256 public constant VIRTUAL_SHARES = 1e6;
    uint256 public constant VIRTUAL_ASSETS = 1;
    uint256 public constant DEAD_SHARES = 1_000;
    uint256 public constant MIN_WRAP = 1_000;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Wrap(address indexed user, uint256 gvIn, uint256 sharesOut);
    event Unwrap(address indexed user, uint256 sharesIn, uint256 gvOut);

    constructor(address gV_) {
        require(gV_ != address(0), "TO");
        gV = gVAPURR(gV_);
    }

    function convertToShares(uint256 gvAssets, uint256 pooled) public view returns (uint256) {
        return (gvAssets * (totalSupply + VIRTUAL_SHARES)) / (pooled + VIRTUAL_ASSETS);
    }

    function convertToAssets(uint256 sharesIn, uint256 pooled) public view returns (uint256) {
        return (sharesIn * (pooled + VIRTUAL_ASSETS)) / (totalSupply + VIRTUAL_SHARES);
    }

    function wrap(uint256 gvAssets) external returns (uint256 sharesOut) {
        require(gvAssets >= MIN_WRAP, "TINY");
        uint256 pooled = gV.balanceOf(address(this));
        require(gV.transferFrom(msg.sender, address(this), gvAssets), "PULL");
        if (totalSupply == 0) {
            // First wrap: lock dead shares permanently (address(0)).
            require(gvAssets > DEAD_SHARES, "TINY");
            sharesOut = gvAssets - DEAD_SHARES;
            totalSupply = gvAssets;
            balanceOf[address(0)] = DEAD_SHARES;
            balanceOf[msg.sender] = sharesOut;
            emit Transfer(address(0), address(0), DEAD_SHARES);
            emit Wrap(msg.sender, gvAssets, sharesOut);
            emit Transfer(address(0), msg.sender, sharesOut);
            return sharesOut;
        }
        sharesOut = convertToShares(gvAssets, pooled);
        require(sharesOut > 0, "TINY");
        totalSupply += sharesOut;
        balanceOf[msg.sender] += sharesOut;
        emit Wrap(msg.sender, gvAssets, sharesOut);
        emit Transfer(address(0), msg.sender, sharesOut);
    }

    function unwrap(uint256 sharesIn) external returns (uint256 gvOut) {
        require(sharesIn > 0 && balanceOf[msg.sender] >= sharesIn, "wgV");
        uint256 pooled = gV.balanceOf(address(this));
        // Floor assets out (favor remaining holders / dead+virtual).
        gvOut = convertToAssets(sharesIn, pooled);
        require(gvOut > 0, "TINY");
        unchecked {
            balanceOf[msg.sender] -= sharesIn;
            totalSupply -= sharesIn;
        }
        require(gV.transfer(msg.sender, gvOut), "gV");
        emit Unwrap(msg.sender, sharesIn, gvOut);
        emit Transfer(msg.sender, address(0), sharesIn);
    }

    function gvPerShare() external view returns (uint256) {
        if (totalSupply == 0) return 1e18;
        uint256 pooled = gV.balanceOf(address(this));
        return ((pooled + VIRTUAL_ASSETS) * 1e18) / (totalSupply + VIRTUAL_SHARES);
    }

    function transfer(address to, uint256 amt) external returns (bool) {
        _move(msg.sender, to, amt);
        return true;
    }

    function approve(address spender, uint256 amt) external returns (bool) {
        allowance[msg.sender][spender] = amt;
        emit Approval(msg.sender, spender, amt);
        return true;
    }

    function transferFrom(address from, address to, uint256 amt) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        if (a != type(uint256).max) {
            require(a >= amt, "ALLOW");
            unchecked {
                allowance[from][msg.sender] = a - amt;
            }
        }
        _move(from, to, amt);
        return true;
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 b = balanceOf[from];
        require(b >= amt, "wgV");
        unchecked {
            balanceOf[from] = b - amt;
            balanceOf[to] += amt;
        }
        emit Transfer(from, to, amt);
    }
}

/// 50k V / 3y from already-minted treasury. Transfers only â€” never mints.
contract BrowserStream {
    uint256 public constant CAP = 50_000 ether;
    uint256 public constant DURATION = 3 * 365 days;

    IVapurrMint public immutable vapurr;
    address public owner;
    address public distributor; // browse/earn path â€” cannot be gV policy
    uint256 public start;
    uint256 public released;
    bool public started;

    event Funded(uint256 amt);
    event Started(uint256 start);
    event Drip(address indexed to, uint256 amt);
    event DistributorUpdated(address indexed distributor);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address vapurr_) {
        require(vapurr_ != address(0), "TO");
        vapurr = IVapurrMint(vapurr_);
        owner = msg.sender;
    }

    function setDistributor(address d) external onlyOwner {
        distributor = d;
        emit DistributorUpdated(d);
    }

    /// Pull already-minted V into the earmark (treasury funds this).
    function fund(uint256 amt) external onlyOwner {
        require(amt > 0, "TINY");
        require(vapurr.transferFrom(msg.sender, address(this), amt), "PULL");
        emit Funded(amt);
    }

    function startStream() external onlyOwner {
        require(!started, "LIVE");
        require(vapurr.balanceOf(address(this)) > 0, "EMPTY");
        started = true;
        start = block.timestamp;
        emit Started(start);
    }

    function vested() public view returns (uint256) {
        if (!started) return 0;
        uint256 bal = vapurr.balanceOf(address(this)) + released;
        uint256 cap = bal < CAP ? bal : CAP;
        uint256 dt = block.timestamp - start;
        if (dt >= DURATION) return cap;
        return (cap * dt) / DURATION;
    }

    function releasable() public view returns (uint256) {
        uint256 v = vested();
        if (v <= released) return 0;
        uint256 due = v - released;
        uint256 bal = vapurr.balanceOf(address(this));
        return due < bal ? due : bal;
    }

    /// Browse path claims earmarked V. No mint. Supply unchanged.
    function drip(address to, uint256 amt) external returns (uint256 paid) {
        require(msg.sender == distributor || msg.sender == owner, "DIST");
        require(to != address(0), "TO");
        paid = amt == 0 ? releasable() : amt;
        require(paid > 0 && paid <= releasable(), "VEST");
        released += paid;
        require(vapurr.transfer(to, paid), "VAPURR");
        emit Drip(to, paid);
    }
}

/// Fed policy holder: sole caller of gV.rebase. Separates mint authority from browse.
/// Annualized gV rate is a function of BondMarket capacity utilization:
///   hot offtake / high util -> toward 1%/yr (suppress V print);
///   cold bond book / low util -> toward 9%/yr;
///   unbound or empty signal -> ~3.5%/yr mid default.
contract RebasePolicy {
    uint256 public constant WAD = 1e18;
    uint256 public constant MIN_RATE_BPS = 100;
    uint256 public constant MAX_RATE_BPS = 900;
    uint256 public constant MID_RATE_BPS = 350;

    address public owner;
    gVAPURR public gV;
    IBondMarketSignal public bondMarket;

    event OwnerUpdated(address indexed owner);
    event GVBound(address indexed gV);
    event BondMarketBound(address indexed bondMarket);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    function bindGV(address gV_) external onlyOwner {
        require(gV_ != address(0), "TO");
        gV = gVAPURR(gV_);
        emit GVBound(gV_);
    }

    function bindBondMarket(address bondMarket_) external onlyOwner {
        bondMarket = IBondMarketSignal(bondMarket_);
        emit BondMarketBound(bondMarket_);
    }

    /// Dynamic annualized policy rate in bps.
    /// rate = MAX - util * (MAX - MIN); util=0 => 900, util=1e18 => 100.
    function policyRateBps() public view returns (uint256) {
        if (address(bondMarket) == address(0) || !bondMarket.hasBondBookSignal()) {
            return MID_RATE_BPS;
        }
        uint256 util = bondMarket.capacityUtilizationWad();
        if (util > WAD) util = WAD;
        uint256 rate = MAX_RATE_BPS - (util * (MAX_RATE_BPS - MIN_RATE_BPS)) / WAD;
        if (rate < MIN_RATE_BPS) return MIN_RATE_BPS;
        if (rate > MAX_RATE_BPS) return MAX_RATE_BPS;
        return rate;
    }

    function rebase() external onlyOwner returns (uint256) {
        require(address(gV) != address(0), "gV");
        return gV.rebase();
    }
}
