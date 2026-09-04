// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// Ketcharts listing board. Rank is $PUSD paid — nothing else.
/// One listing per token. First listing 50. Taking #1 costs +25 over the current top.
/// Raise only charges the difference. Same token stays with the first lister.
/// Bids never expire. Nothing is refunded. The pot sits in this contract.

interface IERC20 {
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
}

contract KetList {
    uint256 public constant DEC = 1e18;
    uint256 public constant MIN_LIST = 50 * DEC;
    uint256 public constant MIN_OUTBID = 25 * DEC;
    uint256 public constant MIN_RAISE = 10 * DEC;
    uint256 public constant MAX_LIST = 999_999 * DEC;
    uint256 public constant MAX_LISTINGS = 256;
    uint256 public constant MAX_SYM = 16;
    uint256 public constant MAX_NAME = 64;
    uint256 public constant MAX_META = 512;

    IERC20 public immutable pusd;

    struct Listing {
        address lister;
        address token;
        address pool;
        uint256 paid;
        uint256 firstAt;
        uint256 lastAt;
        string symbol;
        string name;
        string meta;
    }

    mapping(bytes32 => Listing) public listings;
    bytes32[] public keys;
    uint256 public pot;
    uint256 public topPaid;
    uint256 private _locked = 1;

    modifier lock() {
        require(_locked == 1, "LOCK");
        _locked = 2;
        _;
        _locked = 1;
    }

    event List(
        bytes32 indexed key,
        address indexed token,
        address indexed lister,
        uint256 paid,
        uint256 total,
        address pool
    );

    constructor(address pusd_) {
        require(pusd_ != address(0), "PUSD");
        pusd = IERC20(pusd_);
    }

    function stats()
        external
        view
        returns (uint256 n, uint256 pot_, uint256 top, uint256 minList, uint256 minOut)
    {
        return (keys.length, pot, topPaid, MIN_LIST, MIN_OUTBID);
    }

    function row(uint256 i)
        external
        view
        returns (
            bytes32 key,
            address lister,
            address token,
            address pool,
            uint256 paid,
            uint256 firstAt,
            uint256 lastAt,
            string memory symbol,
            string memory name,
            string memory meta
        )
    {
        require(i < keys.length, "OOB");
        key = keys[i];
        Listing storage L = listings[key];
        return (key, L.lister, L.token, L.pool, L.paid, L.firstAt, L.lastAt, L.symbol, L.name, L.meta);
    }

    function list(
        address token,
        address pool,
        string calldata symbol,
        string calldata name,
        string calldata meta,
        uint256 amt
    ) external lock {
        require(token != address(0), "TOKEN");
        require(pool != address(0) && pool != token, "POOL");
        require(bytes(symbol).length > 0 && bytes(symbol).length <= MAX_SYM, "SYM");
        require(bytes(name).length > 0 && bytes(name).length <= MAX_NAME, "NAME");
        require(bytes(meta).length <= MAX_META, "META");
        require(amt >= MIN_RAISE && amt <= MAX_LIST && amt % DEC == 0, "TINY");

        bytes32 key = keccak256(abi.encodePacked(token));
        Listing storage L = listings[key];
        uint256 pull;
        bool fresh = L.firstAt == 0;
        if (fresh) {
            require(amt >= MIN_LIST, "TINY");
            require(keys.length < MAX_LISTINGS, "FULL");
            if (amt > topPaid && topPaid != 0) {
                require(amt >= topPaid + MIN_OUTBID, "TOP");
            }
            pull = amt;
        } else {
            require(msg.sender == L.lister, "OWNER");
            require(amt >= L.paid + MIN_RAISE, "TINY");
            if (amt > topPaid) {
                require(amt >= topPaid + MIN_OUTBID, "TOP");
            }
            pull = amt - L.paid;
        }
        require(pull > 0, "TINY");
        require(pusd.transferFrom(msg.sender, address(this), pull), "PUSD");
        if (fresh) {
            L.lister = msg.sender;
            L.token = token;
            L.firstAt = block.timestamp;
            keys.push(key);
        }
        L.pool = pool;
        L.paid = amt;
        L.lastAt = block.timestamp;
        L.symbol = symbol;
        L.name = name;
        if (bytes(meta).length > 0) {
            L.meta = meta;
        }
        pot += pull;
        if (L.paid > topPaid) {
            topPaid = L.paid;
        }
        emit List(key, token, msg.sender, pull, L.paid, pool);
    }
}
