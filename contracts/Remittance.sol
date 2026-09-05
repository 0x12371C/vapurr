// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Remittance pipe: branch realized surplus -> RemittanceSink (treasury RFV) ->
/// sink-level runway floor -> sPUSD (or hold).
///
/// INVARIANT (circular RFV): remittance may only move *realized* surplus
/// (collected interest/fees already in hand). Unpaid accrued interest and
/// depositor principal are NOT exogenous RFV — counting the same dollar as
/// RFV and as a user claim is circular.
///
/// INVARIANT (sink-level floor): the runway floor is enforced once on the
/// consolidated sink balance (accounted RFV cash), not as dual local pools on
/// Oliver/Lithe. Branches remit realized surplus into one RemittanceSink;
/// forward/drain above floor happens at the sink.
///
/// Tagged who-paid ledger: optional FeeAttribution proxy in front of this
/// sink (House/Lithe/Oliver). Attribution does not apply a second floor.

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

/// Optional runway floor view. Sink holds the SoT; branches may wire the same
/// address for observability but must not apply a second local floor on remit.
interface IRunwayView {
    function surplus(uint256 balance) external view returns (uint256);
    function floor() external view returns (uint256);
}

/// Shared treasury RFV runway floor — single source of truth.
/// Wire one instance into RemittanceSink (and optionally both branches for views).
/// Do not deploy a separate floor per branch.
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

    /// Alias: remittable / forwardable realized balance above floor.
    function remittable(uint256 realizedBalance) external view returns (uint256) {
        return surplus(realizedBalance);
    }
}

/// Thin treasury RFV view over consolidated remittance cash + shared floor.
/// RemittanceSink implements this; keep as a named surface for ROUTING/docs.
interface ITreasuryRfv {
    function accountedRfv() external view returns (uint256);
    function retainedFloor() external view returns (uint256);
    function surplus() external view returns (uint256);
    function runway() external view returns (RunwayFloor);
}

/// Holds remitted $PUSD for runway / later sPUSD forward. Implements IRemittance.
/// Floor is enforced here on consolidated cash — not on each branch pool.
contract RemittanceSink is IRemittance, ITreasuryRfv {
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

    /// Consolidate branch realized surplus. Floor is NOT applied on intake —
    /// retained runway lives in this sink once cash arrives.
    function receiveRemittance(uint256 amount) external returns (bool) {
        require(amount > 0, "TINY");
        require(asset.transferFrom(msg.sender, address(this), amount), "PULL");
        emit Remitted(msg.sender, amount);
        return true;
    }

    /// Consolidated RFV cash held by the sink (nominal $PUSD inventory).
    function accountedRfv() public view returns (uint256) {
        return asset.balanceOf(address(this));
    }

    /// Cash retained for runway (min of balance and floor).
    function retainedFloor() public view returns (uint256) {
        uint256 bal = accountedRfv();
        uint256 f = runway.floor();
        return bal < f ? bal : f;
    }

    /// Forwardable surplus above sink-level runway floor.
    function surplus() public view returns (uint256) {
        return runway.surplus(accountedRfv());
    }

    /// Forward only surplus above runway floor to configured sPUSD/receiver.
    /// Cannot drain consolidated RFV below the shared floor.
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
