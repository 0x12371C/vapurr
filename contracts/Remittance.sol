// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Remittance pipe: branch surplus -> runway floor gate -> sPUSD (or hold).
/// Stub-grade but wired so Oliver/Lithe/House can call later.
///
/// INVARIANT (circular RFV): remittance may only move *realized* surplus
/// (collected interest/fees already in hand) above the shared runway floor.
/// Unpaid accrued interest and depositor principal are NOT exogenous RFV —
/// counting the same dollar as RFV and as a user claim is circular.

interface IERC20Remit {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function approve(address, uint256) external returns (bool);
}

/// Minimal remittance receiver. Sink or sPUSD implement this.
interface IRemittance {
    /// Caller transfers `amount` of the configured asset to the sink (pull via transferFrom).
    function receiveRemittance(uint256 amount) external returns (bool);
}

/// Optional runway floor view used by Oliver/Lithe before remitting surplus.
/// Both branches MUST point at the same RunwayFloor instance (one treasury runway).
interface IRunwayView {
    function surplus(uint256 balance) external view returns (uint256);
    function floor() external view returns (uint256);
}

/// Shared treasury RFV runway floor — single source of truth for Oliver + Lithe.
/// Remittance surplus is only the *realized* balance above `floor`.
/// Do not deploy a separate floor per branch; wire one instance into both.
contract RunwayFloor {
    address public owner;
    uint256 public floor;

    event FloorUpdated(uint256 floor);
    event OwnerUpdated(address indexed owner);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(uint256 floor_) {
        owner = msg.sender;
        floor = floor_;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    function setFloor(uint256 f) external onlyOwner {
        floor = f;
        emit FloorUpdated(f);
    }

    /// Balance above runway floor (0 if at/under floor).
    function surplus(uint256 balance) public view returns (uint256) {
        return balance > floor ? balance - floor : 0;
    }

    /// Alias: remittable realized balance above floor (same as surplus).
    function remittable(uint256 realizedBalance) external view returns (uint256) {
        return surplus(realizedBalance);
    }
}

/// Holds remitted $PUSD for runway / later sPUSD forward. Implements IRemittance.
contract RemittanceSink is IRemittance {
    IERC20Remit public immutable asset;
    RunwayFloor public immutable runway;
    address public owner;
    address public forward; // optional sPUSD / vault

    event Remitted(address indexed from, uint256 amount);
    event Forwarded(address indexed to, uint256 amount);
    event ForwardUpdated(address indexed forward);
    event OwnerUpdated(address indexed owner);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWN");
        _;
    }

    constructor(address asset_, address runway_) {
        require(asset_ != address(0) && runway_ != address(0), "TO");
        asset = IERC20Remit(asset_);
        runway = RunwayFloor(runway_);
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        require(o != address(0), "TO");
        owner = o;
        emit OwnerUpdated(o);
    }

    function setForward(address f) external onlyOwner {
        forward = f;
        emit ForwardUpdated(f);
    }

    function receiveRemittance(uint256 amount) external returns (bool) {
        require(amount > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        emit Remitted(msg.sender, amount);
        return true;
    }

    function surplus() public view returns (uint256) {
        return runway.surplus(asset.balanceOf(address(this)));
    }

    /// Forward only surplus above runway floor to configured sPUSD/receiver.
    function forwardSurplus(uint256 amount) external onlyOwner returns (uint256 sent) {
        require(forward != address(0), "FWD");
        uint256 free = surplus();
        sent = amount == 0 ? free : amount;
        require(sent > 0 && sent <= free, "FLOOR");
        require(asset.approve(forward, sent), "ALLOW");
        require(IRemittance(forward).receiveRemittance(sent), "REMIT");
        emit Forwarded(forward, sent);
    }
}
