// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

bytes32 constant EXO_TAG_ETH = "ETH";
bytes32 constant EXO_TAG_NVDA = "NVDA";
bytes32 constant EXO_TAG_AMD = "AMD";

interface IExogenousPairRegistry {
    function vapurr() external view returns (address);
    function usdgBanned() external view returns (address);
    function pusdBanned() external view returns (address);
    function pairOf(bytes32 tag) external view returns (address exogenous, address pool, bool enabled);
    function requireExogenousPair(bytes32 tag, address currency0, address currency1) external view;
    function isExogenousPair(bytes32 tag, address a, address b) external view returns (bool);
}

interface IERC20Seed {
    function transferFrom(address, address, uint256) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

/// @title ExogenousPairRegistry - V/exogenous POL and trading books at genesis
/// @notice Distinct from bond purchase (exogenous in -> gV out). USDG is bond intake ONLY.
contract ExogenousPairRegistry is IExogenousPairRegistry {
    bytes32 public constant TAG_ETH = EXO_TAG_ETH;
    bytes32 public constant TAG_NVDA = EXO_TAG_NVDA;
    bytes32 public constant TAG_AMD = EXO_TAG_AMD;

    address public immutable override vapurr;
    /// Bond-intake-only; cannot be a pair leg.
    address public immutable override usdgBanned;
    /// Product cash; not an exogenous POL leg here (House is wgV/PUSD; Lithe is V/PUSD mint).
    address public immutable override pusdBanned;

    address public owner;

    struct Pair {
        address exogenous;
        address pool; // optional seeded / Uni book address (0 until live)
        bool enabled;
        bool set;
    }

    mapping(bytes32 => Pair) internal pairs;
    bytes32[] public tags;

    event OwnerUpdated(address indexed owner);
    event PairRegistered(bytes32 indexed tag, address exogenous, bool enabled);
    event PairEnabled(bytes32 indexed tag, bool enabled);
    event PoolBound(bytes32 indexed tag, address pool);
    event PoolValidated(bytes32 indexed tag, address currency0, address currency1, bytes32 poolId);

    error ZeroAddr();
    error UsdgNotPairAsset();
    error PusdNotExogenousPair();
    error BadExogenousPair();
    error UnknownTag();
    error NotOwner();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    /// @param vapurr_ canonical $VAPURR
    /// @param usdg_ USDG token (banned as pair asset; address(0) if unset on testnet)
    /// @param pusd_ $PUSD (banned as exogenous POL leg in this registry)
    constructor(address vapurr_, address usdg_, address pusd_) {
        if (vapurr_ == address(0)) revert ZeroAddr();
        vapurr = vapurr_;
        usdgBanned = usdg_;
        pusdBanned = pusd_;
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        if (o == address(0)) revert ZeroAddr();
        owner = o;
        emit OwnerUpdated(o);
    }

    /// Register or replace a V/exogenous trading book tag.
    function registerPair(bytes32 tag, address exogenous, bool enabled) external onlyOwner {
        if (tag == bytes32(0) || exogenous == address(0)) revert ZeroAddr();
        _assertAllowedExogenous(exogenous);
        bool first = !pairs[tag].set;
        pairs[tag] = Pair({exogenous: exogenous, pool: pairs[tag].pool, enabled: enabled, set: true});
        if (first) tags.push(tag);
        emit PairRegistered(tag, exogenous, enabled);
    }

    function setEnabled(bytes32 tag, bool enabled) external onlyOwner {
        if (!pairs[tag].set) revert UnknownTag();
        pairs[tag].enabled = enabled;
        emit PairEnabled(tag, enabled);
    }

    /// Bind a live pool / seed-market address after deploy (honest empty until then).
    function bindPool(bytes32 tag, address pool) external onlyOwner {
        if (!pairs[tag].set) revert UnknownTag();
        if (pool == address(0)) revert ZeroAddr();
        pairs[tag].pool = pool;
        emit PoolBound(tag, pool);
    }

    function pairOf(bytes32 tag) external view override returns (address exogenous, address pool, bool enabled) {
        Pair memory p = pairs[tag];
        return (p.exogenous, p.pool, p.enabled);
    }

    function tagCount() external view returns (uint256) {
        return tags.length;
    }

    function _assertAllowedExogenous(address asset) internal view {
        if (usdgBanned != address(0) && asset == usdgBanned) revert UsdgNotPairAsset();
        if (pusdBanned != address(0) && asset == pusdBanned) revert PusdNotExogenousPair();
        if (asset == vapurr) revert BadExogenousPair();
    }

    /// Pool currencies must be exactly {vapurr, exogenousForTag} (either order).
    function requireExogenousPair(bytes32 tag, address currency0, address currency1) public view override {
        Pair memory p = pairs[tag];
        if (!p.set || !p.enabled) revert UnknownTag();
        bool ok = (currency0 == vapurr && currency1 == p.exogenous)
            || (currency0 == p.exogenous && currency1 == vapurr);
        if (!ok) revert BadExogenousPair();
        _assertAllowedExogenous(p.exogenous);
    }

    function isExogenousPair(bytes32 tag, address a, address b) external view override returns (bool) {
        Pair memory p = pairs[tag];
        if (!p.set || !p.enabled || a == address(0) || b == address(0)) return false;
        return (a == vapurr && b == p.exogenous) || (a == p.exogenous && b == vapurr);
    }

    /// Gate before Uni v4 / CPMM initialize - same role as HousePairFactory.validateAndMark.
    function validateAndMark(bytes32 tag, address currency0, address currency1) external returns (bytes32 poolId) {
        requireExogenousPair(tag, currency0, currency1);
        poolId = keccak256(abi.encode(tag, currency0, currency1));
        emit PoolValidated(tag, currency0, currency1, poolId);
    }
}

/// Thin constant-product seed book for forge / bootstrap proofs.
/// Not a production AMM - records reserves so launch can stand up V/ETH etc. inventory.
contract ExogenousSeedMarket {
    address public immutable token0; // canonical V
    address public immutable token1; // exogenous
    bytes32 public immutable tag;
    ExogenousPairRegistry public immutable registry;

    uint256 public reserve0;
    uint256 public reserve1;
    address public owner;

    event Seeded(uint256 amount0, uint256 amount1);
    event OwnerUpdated(address indexed owner);

    error NotOwner();
    error ZeroAddr();
    error Tiny();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(address registry_, bytes32 tag_, address vapurr_, address exogenous_) {
        if (registry_ == address(0) || vapurr_ == address(0) || exogenous_ == address(0)) revert ZeroAddr();
        registry = ExogenousPairRegistry(registry_);
        registry.requireExogenousPair(tag_, vapurr_, exogenous_);
        tag = tag_;
        token0 = vapurr_;
        token1 = exogenous_;
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        if (o == address(0)) revert ZeroAddr();
        owner = o;
        emit OwnerUpdated(o);
    }

    /// Pull already-minted V + exogenous into the seed book (POL bootstrap). No mint.
    function seed(uint256 amountV, uint256 amountExo) external onlyOwner {
        if (amountV == 0 || amountExo == 0) revert Tiny();
        require(IERC20Seed(token0).transferFrom(msg.sender, address(this), amountV), "PULL0");
        require(IERC20Seed(token1).transferFrom(msg.sender, address(this), amountExo), "PULL1");
        reserve0 += amountV;
        reserve1 += amountExo;
        emit Seeded(amountV, amountExo);
    }

    function reserves() external view returns (uint256, uint256) {
        return (reserve0, reserve1);
    }
}
