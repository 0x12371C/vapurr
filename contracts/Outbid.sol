// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// Pay-to-rank board. Rank is $PUSD paid — nothing else.
/// Whole PUSD. First listing 10. Taking #1 costs +5 over the current top.
/// Raise only charges the difference. Same URL stays with the first bidder.
/// Bids never expire. Nothing is refunded. The pot sits in this contract.

interface IERC20 {
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
}

contract Outbid {
    uint256 public constant DEC = 1e18;
    uint256 public constant MIN_BID = 10 * DEC;
    uint256 public constant MIN_OUTBID = 5 * DEC;
    uint256 public constant MIN_RAISE = 1 * DEC;
    uint256 public constant MAX_BID = 999_999 * DEC;
    uint256 public constant MAX_LISTINGS = 256;
    uint256 public constant MAX_URL = 256;
    uint256 public constant MAX_TITLE = 64;

    IERC20 public immutable pusd;

    struct Listing {
        address bidder;
        uint256 paid;
        uint256 firstAt;
        uint256 lastAt;
        string url;
        string title;
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

    event Bid(bytes32 indexed key, address indexed bidder, uint256 paid, uint256 total, string url);

    constructor(address pusd_) {
        require(pusd_ != address(0), "PUSD");
        pusd = IERC20(pusd_);
    }

    function stats()
        external
        view
        returns (uint256 n, uint256 pot_, uint256 top, uint256 minBid, uint256 minOut)
    {
        return (keys.length, pot, topPaid, MIN_BID, MIN_OUTBID);
    }

    function row(uint256 i)
        external
        view
        returns (
            bytes32 key,
            address bidder,
            uint256 paid,
            uint256 firstAt,
            uint256 lastAt,
            string memory url,
            string memory title
        )
    {
        require(i < keys.length, "OOB");
        key = keys[i];
        Listing storage L = listings[key];
        return (key, L.bidder, L.paid, L.firstAt, L.lastAt, L.url, L.title);
    }

    function bid(string calldata url, string calldata title, uint256 amt) external lock {
        require(bytes(url).length > 0 && bytes(url).length <= MAX_URL, "URL");
        require(bytes(title).length <= MAX_TITLE, "TITLE");
        require(amt >= MIN_RAISE && amt <= MAX_BID && amt % DEC == 0, "TINY");

        bytes32 key = keccak256(bytes(url));
        Listing storage L = listings[key];
        uint256 pull;
        bool fresh = L.firstAt == 0;
        if (fresh) {
            require(amt >= MIN_BID, "TINY");
            require(keys.length < MAX_LISTINGS, "FULL");
            if (amt > topPaid && topPaid != 0) {
                require(amt >= topPaid + MIN_OUTBID, "TOP");
            }
            pull = amt;
        } else {
            require(msg.sender == L.bidder, "OWNER");
            require(amt >= L.paid + MIN_RAISE, "TINY");
            if (amt > topPaid) {
                require(amt >= topPaid + MIN_OUTBID, "TOP");
            }
            pull = amt - L.paid;
        }
        require(pull > 0, "TINY");
        require(pusd.transferFrom(msg.sender, address(this), pull), "PUSD");
        if (fresh) {
            L.bidder = msg.sender;
            L.firstAt = block.timestamp;
            L.url = url;
            keys.push(key);
        }
        L.paid = amt;
        L.lastAt = block.timestamp;
        if (bytes(title).length > 0) {
            L.title = title;
        }
        pot += pull;
        if (L.paid > topPaid) {
            topPaid = L.paid;
        }
        emit Bid(key, msg.sender, pull, L.paid, L.url);
    }
}
