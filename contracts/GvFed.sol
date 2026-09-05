// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Fed staking slice: V mint policy, gV rebase, wgV wrapper, BrowserStream earmark.
/// HARD WALL: browse/earn MUST NOT receive or trigger the 3.5%/yr staker mint.
/// Rebase model: **index** (shares fixed; balanceOf = shares * index / DEC), matching PusdToken.

/// Standalone mintable $VAPURR for Fed modules / tests (market embeds its own copy).
contract VapurrToken {
    string public constant name = "VAPURR";
    string public constant symbol = "VAPURR";
    uint8 public constant decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    address public minter;
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event MinterUpdated(address indexed minter);

    constructor() {
        minter = msg.sender;
    }

    modifier onlyMinter() {
        require(msg.sender == minter, "MINTER");
        _;
    }

    function setMinter(address m) external onlyMinter {
        require(m != address(0), "TO");
        minter = m;
        emit MinterUpdated(m);
    }

    function mint(address to, uint256 amt) external onlyMinter {
        require(to != address(0), "TO");
        totalSupply += amt;
        balanceOf[to] += amt;
        emit Transfer(address(0), to, amt);
    }

    function burn(address from, uint256 amt) external onlyMinter {
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

interface IVapurrMint {
    function mint(address to, uint256 amt) external;
    function transfer(address to, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function totalSupply() external view returns (uint256);
    function approve(address spender, uint256 amt) external returns (bool);
}

/// Staked V with flat 3.5%/yr index rebase. Sole V inflation path to stakers.
contract gVAPURR {
    string public constant name = "gVAPURR";
    string public constant symbol = "gVAPURR";
    uint8 public constant decimals = 18;
    uint256 public constant DEC = 1e18;
    /// 3.5% = 350 bps flat (linear in time; not compounded continuously).
    uint256 public constant REBASE_BPS = 350;
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

    function totalSupply() public view returns (uint256) {
        return (totalShares * index) / DEC;
    }

    function balanceOf(address a) public view returns (uint256) {
        return (shares[a] * index) / DEC;
    }

    function stake(uint256 assets) external returns (uint256 sharesOut) {
        require(assets > 0, "TINY");
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

    /// Accrue flat 3.5%/yr by minting V into this contract and lifting the index.
    /// Only `policy` (Fed/treasury). Browse distributors must never be policy.
    function rebase() external onlyPolicy returns (uint256 minted) {
        uint256 dt = block.timestamp - lastRebase;
        lastRebase = block.timestamp;
        uint256 supply = totalSupply();
        if (dt == 0 || supply == 0) {
            emit Rebase(0, index, block.timestamp);
            return 0;
        }
        minted = (supply * REBASE_BPS * dt) / 10_000 / YEAR;
        if (minted == 0) {
            emit Rebase(0, index, block.timestamp);
            return 0;
        }
        vapurr.mint(address(this), minted);
        index = (index * (supply + minted)) / supply;
        emit Rebase(minted, index, block.timestamp);
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

    function wrap(uint256 gvAssets) external returns (uint256 sharesOut) {
        require(gvAssets > 0, "TINY");
        uint256 pooled = gV.balanceOf(address(this));
        require(gV.transferFrom(msg.sender, address(this), gvAssets), "PULL");
        if (totalSupply == 0 || pooled == 0) {
            sharesOut = gvAssets;
        } else {
            sharesOut = (gvAssets * totalSupply) / pooled;
        }
        require(sharesOut > 0, "TINY");
        totalSupply += sharesOut;
        balanceOf[msg.sender] += sharesOut;
        emit Wrap(msg.sender, gvAssets, sharesOut);
        emit Transfer(address(0), msg.sender, sharesOut);
    }

    function unwrap(uint256 sharesIn) external returns (uint256 gvOut) {
        require(sharesIn > 0 && balanceOf[msg.sender] >= sharesIn, "wgV");
        uint256 pooled = gV.balanceOf(address(this));
        gvOut = (sharesIn * pooled) / totalSupply;
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
        return (gV.balanceOf(address(this)) * 1e18) / totalSupply;
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

/// 50k V / 3y from already-minted treasury. Transfers only — never mints.
contract BrowserStream {
    uint256 public constant CAP = 50_000 ether;
    uint256 public constant DURATION = 3 * 365 days;

    IVapurrMint public immutable vapurr;
    address public owner;
    address public distributor; // browse/earn path — cannot be gV policy
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
contract RebasePolicy {
    address public owner;
    gVAPURR public gV;

    event OwnerUpdated(address indexed owner);
    event GVBound(address indexed gV);

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

    function rebase() external onlyOwner returns (uint256) {
        require(address(gV) != address(0), "gV");
        return gV.rebase();
    }
}
