// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Gated bond market skeleton — exogenous RFV in, gV/wgV out from inventory only.
/// HARD WALL: markets stay disabled (or capacity 0) until Fed enables after inventory,
/// vesting ownership, capacity, haircuts, and valuation are wired. No mint path.
/// See docs/econ/BONDS.md.

interface IERC20Bond {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function approve(address, uint256) external returns (bool);
}

/// Asset tabs on the Bonds surface (v1).
enum BondAssetTag {
    ETH,
    USDG,
    STOCKS
}

/// Fed-gated bond markets. Pays pre-funded equity inventory — never mints Fed supply.
contract BondMarket {
    uint256 public constant BPS = 10_000;
    uint256 public constant WAD = 1e18;

    address public owner; // Fed / policy
    /// Equity inventory token (gV or wgV). Must be pre-funded; bond() never mints.
    IERC20Bond public immutable payoutToken;
    /// Optional Fed V token for supply assertions — bond path must not mint it.
    address public immutable fedV;

    struct Market {
        bool enabled; // default false until Fed enables
        address asset; // exogenous ERC20 (WETH / USDG / stock wrapper)
        address treasury; // RFV sink for pulled assets
        uint16 discountBps; // bond sweetener vs credited RFV (face > credit)
        uint16 haircutBps; // conservative cut on asset valuation
        uint64 vestingSeconds;
        uint256 capacity; // remaining credited-RFV capacity (0 = closed)
        uint256 priceWad; // payout wei per 1 wei asset (Fed valuation; oracle later)
    }

    struct Position {
        address owner;
        BondAssetTag tag;
        uint256 payoutAmount;
        uint64 unlockAt;
        bool claimed;
    }

    mapping(BondAssetTag => Market) public markets;
    /// gV/wgV reserved for open vesting positions (not yet claimed).
    uint256 public reservedPayout;
    uint256 public nextId = 1;
    mapping(uint256 => Position) public positions;

    event OwnerUpdated(address indexed owner);
    event MarketSet(
        BondAssetTag indexed tag,
        address asset,
        address treasury,
        uint16 discountBps,
        uint16 haircutBps,
        uint64 vestingSeconds,
        uint256 capacity,
        uint256 priceWad,
        bool enabled
    );
    event MarketEnabled(BondAssetTag indexed tag, bool enabled);
    event CapacitySet(BondAssetTag indexed tag, uint256 capacity);
    event InventoryFunded(address indexed from, uint256 amount);
    event Bonded(
        uint256 indexed id,
        address indexed user,
        BondAssetTag indexed tag,
        uint256 assetIn,
        uint256 creditedRfv,
        uint256 payoutAmount,
        uint64 unlockAt
    );
    event Claimed(uint256 indexed id, address indexed user, uint256 payoutAmount);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address payoutToken_, address fedV_) {
        require(payoutToken_ != address(0), "TO");
        payoutToken = IERC20Bond(payoutToken_);
        fedV = fedV_;
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    /// Configure a tab. New markets should ship enabled=false and/or capacity=0.
    function setMarket(
        BondAssetTag tag,
        address asset,
        address treasury,
        uint16 discountBps,
        uint16 haircutBps,
        uint64 vestingSeconds,
        uint256 capacity,
        uint256 priceWad,
        bool enabled
    ) external onlyOwner {
        require(asset != address(0) && treasury != address(0), "TO");
        require(discountBps < BPS && haircutBps < BPS, "BPS");
        require(priceWad > 0, "PRICE");
        markets[tag] = Market({
            enabled: enabled,
            asset: asset,
            treasury: treasury,
            discountBps: discountBps,
            haircutBps: haircutBps,
            vestingSeconds: vestingSeconds,
            capacity: capacity,
            priceWad: priceWad
        });
        emit MarketSet(tag, asset, treasury, discountBps, haircutBps, vestingSeconds, capacity, priceWad, enabled);
    }

    function setEnabled(BondAssetTag tag, bool enabled) external onlyOwner {
        require(markets[tag].asset != address(0), "MKT");
        markets[tag].enabled = enabled;
        emit MarketEnabled(tag, enabled);
    }

    function setCapacity(BondAssetTag tag, uint256 capacity) external onlyOwner {
        require(markets[tag].asset != address(0), "MKT");
        markets[tag].capacity = capacity;
        emit CapacitySet(tag, capacity);
    }

    function setValuation(BondAssetTag tag, uint256 priceWad, uint16 haircutBps, uint16 discountBps) external onlyOwner {
        Market storage m = markets[tag];
        require(m.asset != address(0), "MKT");
        require(priceWad > 0 && discountBps < BPS && haircutBps < BPS, "PARAM");
        m.priceWad = priceWad;
        m.haircutBps = haircutBps;
        m.discountBps = discountBps;
        emit MarketSet(
            tag, m.asset, m.treasury, m.discountBps, m.haircutBps, m.vestingSeconds, m.capacity, m.priceWad, m.enabled
        );
    }

    /// Pull already-minted gV/wgV into bond inventory (treasury funds this). No mint.
    function fundInventory(uint256 amount) external onlyOwner {
        require(amount > 0, "TINY");
        require(payoutToken.transferFrom(msg.sender, address(this), amount), "PULL");
        emit InventoryFunded(msg.sender, amount);
    }

    /// Free inventory available for new bonds (balance minus reserved vesting).
    function availableInventory() public view returns (uint256) {
        uint256 bal = payoutToken.balanceOf(address(this));
        return bal > reservedPayout ? bal - reservedPayout : 0;
    }

    /// Quote discount + vesting for an asset amount. Reverts if market unset.
    function quote(BondAssetTag tag, uint256 assetAmount)
        public
        view
        returns (uint256 payoutAmount, uint256 creditedRfv, uint64 vestingSeconds, uint16 discountBps, uint16 haircutBps)
    {
        Market memory m = markets[tag];
        require(m.asset != address(0), "MKT");
        require(assetAmount > 0, "TINY");
        (creditedRfv, payoutAmount) = _quote(m, assetAmount);
        vestingSeconds = m.vestingSeconds;
        discountBps = m.discountBps;
        haircutBps = m.haircutBps;
    }

    /// Bond exogenous asset into treasury/RFV sink; vest payout from inventory only.
    function bond(BondAssetTag tag, uint256 assetAmount) external returns (uint256 id) {
        Market storage m = markets[tag];
        require(m.enabled, "DISABLED");
        require(m.asset != address(0) && m.capacity > 0, "CLOSED");
        require(assetAmount > 0, "TINY");

        (uint256 creditedRfv, uint256 payoutAmount) = _quote(m, assetAmount);
        require(creditedRfv > 0 && payoutAmount > 0, "TINY");
        require(creditedRfv <= m.capacity, "CAP");
        require(payoutAmount <= availableInventory(), "INV");

        // Pull exogenous asset into RFV sink.
        require(IERC20Bond(m.asset).transferFrom(msg.sender, m.treasury, assetAmount), "PULL");

        unchecked {
            m.capacity -= creditedRfv;
            reservedPayout += payoutAmount;
        }

        id = nextId++;
        uint64 unlockAt = uint64(block.timestamp + m.vestingSeconds);
        positions[id] = Position({
            owner: msg.sender,
            tag: tag,
            payoutAmount: payoutAmount,
            unlockAt: unlockAt,
            claimed: false
        });

        emit Bonded(id, msg.sender, tag, assetAmount, creditedRfv, payoutAmount, unlockAt);
    }

    /// Claim vested gV/wgV from reserved inventory. No mint; no early exit in v1.
    function claim(uint256 id) external returns (uint256 paid) {
        Position storage p = positions[id];
        require(p.owner == msg.sender && !p.claimed, "POS");
        require(block.timestamp >= p.unlockAt, "VEST");
        paid = p.payoutAmount;
        p.claimed = true;
        unchecked {
            reservedPayout -= paid;
        }
        require(payoutToken.transfer(msg.sender, paid), "PAY");
        emit Claimed(id, msg.sender, paid);
    }

    function _quote(Market memory m, uint256 assetAmount)
        internal
        pure
        returns (uint256 creditedRfv, uint256 payoutAmount)
    {
        // Gross valuation at Fed price, then haircut (conservative RFV credit).
        uint256 gross = (assetAmount * m.priceWad) / WAD;
        creditedRfv = (gross * (BPS - m.haircutBps)) / BPS;
        // Discount: face payout above credited RFV (bond sweetener), still inventory-capped.
        payoutAmount = (creditedRfv * BPS) / (BPS - m.discountBps);
    }
}