// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import "./Remittance.sol";

/// Liquid sPUSD — minimal ERC-4626-style vault over $PUSD.
/// Receives branch remittance (yield credit) without minting new shares (NAV rises).
/// Shared surplus allocation via SavingsRouter; term savings in SpusdCd. See docs/econ/SPUSD.md.
/// Donation guards: virtual shares + dead shares + min deposit.

contract SPUSD is IRemittance {
    string public constant name = "Savings PUSD";
    string public constant symbol = "sPUSD";
    uint8 public constant decimals = 18;

    /// Virtual offset (OZ ERC-4626 style) resists donation / inflation rounding theft.
    uint256 public constant VIRTUAL_SHARES = 1e6;
    uint256 public constant VIRTUAL_ASSETS = 1;
    /// Permanently locked on first deposit (dead shares).
    uint256 public constant DEAD_SHARES = 1_000;
    /// Minimum deposit assets (blocks dust remittance skim + inflation seed).
    uint256 public constant MIN_DEPOSIT = 1_000;

    IERC20Remit public immutable asset; // $PUSD
    address public owner;

    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Deposit(address indexed caller, address indexed owner, uint256 assets, uint256 shares);
    event Withdraw(
        address indexed caller, address indexed receiver, address indexed owner, uint256 assets, uint256 shares
    );
    event YieldCredit(address indexed from, uint256 assets);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address asset_) {
        require(asset_ != address(0), "TO");
        asset = IERC20Remit(asset_);
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
    }

    function totalAssets() public view returns (uint256) {
        return asset.balanceOf(address(this));
    }

    function convertToShares(uint256 assets) public view returns (uint256) {
        return (assets * (totalSupply + VIRTUAL_SHARES)) / (totalAssets() + VIRTUAL_ASSETS);
    }

    function convertToAssets(uint256 shares) public view returns (uint256) {
        return (shares * (totalAssets() + VIRTUAL_ASSETS)) / (totalSupply + VIRTUAL_SHARES);
    }

    function deposit(uint256 assets, address receiver) external returns (uint256 shares) {
        require(assets >= MIN_DEPOSIT && receiver != address(0), "TINY");
        if (totalSupply == 0) {
            // First depositor: lock dead shares to address(0); remainder to receiver.
            require(assets > DEAD_SHARES, "TINY");
            require(asset.transferFrom(msg.sender, address(this), assets), "PULL");
            shares = assets - DEAD_SHARES;
            totalSupply = assets;
            balanceOf[address(0)] = DEAD_SHARES;
            balanceOf[receiver] = shares;
            emit Transfer(address(0), address(0), DEAD_SHARES);
            emit Deposit(msg.sender, receiver, assets, shares);
            emit Transfer(address(0), receiver, shares);
            return shares;
        }
        // Price against pre-transfer balances (donated assets already in totalAssets).
        shares = convertToShares(assets);
        require(shares > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), assets), "PULL");
        totalSupply += shares;
        balanceOf[receiver] += shares;
        emit Deposit(msg.sender, receiver, assets, shares);
        emit Transfer(address(0), receiver, shares);
    }

    function withdraw(uint256 assets, address receiver, address owner_) external returns (uint256 shares) {
        require(assets > 0 && receiver != address(0), "TINY");
        uint256 ta = totalAssets();
        // Ceil shares burned (favor vault / remaining holders).
        shares = (assets * (totalSupply + VIRTUAL_SHARES) + (ta + VIRTUAL_ASSETS) - 1) / (ta + VIRTUAL_ASSETS);
        _burnFrom(owner_, shares, msg.sender);
        require(asset.transfer(receiver, assets), "PUSD");
        emit Withdraw(msg.sender, receiver, owner_, assets, shares);
    }

    function redeem(uint256 shares, address receiver, address owner_) external returns (uint256 assets) {
        require(shares > 0 && receiver != address(0), "TINY");
        // Floor assets out (favor vault).
        assets = convertToAssets(shares);
        require(assets > 0, "TINY");
        _burnFrom(owner_, shares, msg.sender);
        require(asset.transfer(receiver, assets), "PUSD");
        emit Withdraw(msg.sender, receiver, owner_, assets, shares);
    }

    /// Remittance / yield credit: assets in, no new shares → existing share NAV rises.
    function receiveRemittance(uint256 amount) public returns (bool) {
        require(amount > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        emit YieldCredit(msg.sender, amount);
        return true;
    }

    /// Explicit yield credit (same semantics; owner or remitter may call after transfer).
    function creditYield(uint256 amount) external returns (bool) {
        return receiveRemittance(amount);
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

    function _burnFrom(address owner_, uint256 shares, address spender) internal {
        require(balanceOf[owner_] >= shares, "sPUSD");
        if (spender != owner_) {
            uint256 a = allowance[owner_][spender];
            if (a != type(uint256).max) {
                require(a >= shares, "ALLOW");
                unchecked {
                    allowance[owner_][spender] = a - shares;
                }
            }
        }
        unchecked {
            balanceOf[owner_] -= shares;
            totalSupply -= shares;
        }
        emit Transfer(owner_, address(0), shares);
    }

    function _move(address from, address to, uint256 amt) internal {
        require(to != address(0), "TO");
        uint256 b = balanceOf[from];
        require(b >= amt, "sPUSD");
        unchecked {
            balanceOf[from] = b - amt;
            balanceOf[to] += amt;
        }
        emit Transfer(from, to, amt);
    }
}