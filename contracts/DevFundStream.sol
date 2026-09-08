// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title DevFundStream - Sablier-style lockup; V auto-locks as Oliver collateral
/// @notice Relic HARD LOCK: genesis 200_000 $VAPURR unlocks on an expansion-aware
/// schedule, then AUTOMATICALLY deposits as Oliver (PusdLoop) collateral.
/// Recipient may ONLY draw $PUSD against that collateral. There is NO path that
/// transfers $VAPURR to the recipient wallet, AMM, or open market.
/// Distinct from BrowserStream (50k/3y treasury float, browse drip).
/// Formula: docs/econ/DEV_FUND.md

interface IVapurrTokenView {
    function transfer(address to, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function approve(address spender, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function totalSupply() external view returns (uint256);
}

interface IPusdToken {
    function transfer(address to, uint256 amt) external returns (bool);
    function transferFrom(address from, address to, uint256 amt) external returns (bool);
    function balanceOf(address) external view returns (uint256);
    function approve(address spender, uint256 amt) external returns (bool);
}

/// Minimal Oliver surface used by DevFund. withdrawV is intentionally NOT called.
interface IOliverVault {
    function depositV(uint256 amt) external;
    function borrow(uint256 amt) external;
    function repay(uint256 amt) external;
    function collatV(address user) external view returns (uint256);
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
}

contract DevFundStream {
    uint256 public constant WAD = 1e18;
    uint256 public constant GENESIS_AMOUNT = 200_000 ether;
    uint256 public constant BASE_DURATION = 4 * 365 days;

    IVapurrTokenView public immutable vapurr;
    IOliverVault public immutable oliver;
    IPusdToken public immutable pusd;

    address public recipient;
    address public owner;

    uint256 public startSupply;
    uint256 public start;
    uint256 public deposited;
    uint256 public accrued;
    /// Unlocked V already pushed into Oliver as collatV[address(this)].
    uint256 public lockedInOliver;
    uint256 public lastAccrual;
    bool public started;
    bool public recipientFrozen;

    event OwnerUpdated(address indexed owner);
    event RecipientUpdated(address indexed recipient);
    event Funded(uint256 amount);
    event Started(uint256 start, uint256 startSupply, uint256 deposited);
    event SettledToOliver(uint256 amount, uint256 totalLocked);
    event DrewPusd(address indexed to, uint256 amount);
    event Repaid(uint256 amount);

    error NotOwner();
    error NotRecipient();
    error ZeroAddr();
    error Tiny();
    error Live();
    error Frozen();
    error Vest();
    error BadOliver();
    error NoMarketSell();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    /// @param vapurr_ canonical $VAPURR
    /// @param oliver_ Oliver vault (PusdLoop). Immutable - cannot redirect V out.
    /// @param recipient_ sole $PUSD drawer (frozen at startStream)
    constructor(address vapurr_, address oliver_, address recipient_) {
        if (vapurr_ == address(0) || oliver_ == address(0) || recipient_ == address(0)) revert ZeroAddr();
        IOliverVault o = IOliverVault(oliver_);
        if (o.vapurr() != vapurr_) revert BadOliver();
        vapurr = IVapurrTokenView(vapurr_);
        oliver = o;
        pusd = IPusdToken(o.pusd());
        recipient = recipient_;
        owner = msg.sender;
    }

    function setOwner(address o) external onlyOwner {
        if (o == address(0)) revert ZeroAddr();
        owner = o;
        emit OwnerUpdated(o);
    }

    /// Recipient may change only before the stream starts (then soulbound).
    function setRecipient(address r) external onlyOwner {
        if (recipientFrozen) revert Frozen();
        if (r == address(0)) revert ZeroAddr();
        recipient = r;
        emit RecipientUpdated(r);
    }

    /// Pull from caller and/or account inventory already sent to this contract.
    function fund(uint256 amount) external onlyOwner {
        if (amount == 0) revert Tiny();
        uint256 have = vapurr.balanceOf(address(this));
        // Inventory already counted as deposited stays reserved; need `amount` more free.
        uint256 free = have > deposited ? have - deposited : 0;
        if (free < amount) {
            require(vapurr.transferFrom(msg.sender, address(this), amount - free), "PULL");
        }
        deposited += amount;
        emit Funded(amount);
    }

    function startStream() external onlyOwner {
        if (started) revert Live();
        if (deposited == 0) revert Tiny();
        uint256 supply = vapurr.totalSupply();
        if (supply == 0) revert Tiny();
        started = true;
        recipientFrozen = true;
        start = block.timestamp;
        lastAccrual = block.timestamp;
        startSupply = supply;
        emit Started(start, startSupply, deposited);
    }

    function expansionWad() public view returns (uint256) {
        if (!started || startSupply == 0) return WAD;
        uint256 supply = vapurr.totalSupply();
        if (supply <= startSupply) return WAD;
        return (supply * WAD) / startSupply;
    }

    function unlockRatePerSecond() public view returns (uint256) {
        if (!started || deposited == 0) return 0;
        uint256 exp = expansionWad();
        return (deposited * WAD) / (BASE_DURATION * exp);
    }

    function _pendingUnlock() internal view returns (uint256) {
        if (!started) return 0;
        if (accrued >= deposited) return 0;
        uint256 dt = block.timestamp - lastAccrual;
        if (dt == 0) return 0;
        uint256 exp = expansionWad();
        uint256 unlock = (deposited * dt * WAD) / (BASE_DURATION * exp);
        uint256 room = deposited - accrued;
        return unlock > room ? room : unlock;
    }

    function _accrue() internal {
        if (!started) return;
        uint256 pending = _pendingUnlock();
        if (pending > 0) accrued += pending;
        lastAccrual = block.timestamp;
    }

    function vested() public view returns (uint256) {
        uint256 v = accrued + _pendingUnlock();
        return v > deposited ? deposited : v;
    }

    /// Unlocked V not yet pushed to Oliver.
    function unsettleable() public view returns (uint256) {
        uint256 v = vested();
        return v > lockedInOliver ? v - lockedInOliver : 0;
    }

    /// HARD LOCK: push unlocked V into Oliver as collatV[this]. Never to recipient.
    function settle() public returns (uint256 locked) {
        _accrue();
        locked = unsettleable();
        if (locked == 0) return 0;
        // Cap by actual balance (inventory must be present).
        uint256 bal = vapurr.balanceOf(address(this));
        if (locked > bal) locked = bal;
        if (locked == 0) return 0;
        require(vapurr.approve(address(oliver), locked), "ALLOW");
        oliver.depositV(locked);
        lockedInOliver += locked;
        emit SettledToOliver(locked, lockedInOliver);
    }

    /// Sole claim path: borrow $PUSD against stream-owned Oliver collateral -> recipient.
    /// No $VAPURR leaves to wallet/AMM.
    function drawPusd(uint256 amount) external returns (uint256 paid) {
        if (msg.sender != recipient && msg.sender != owner) revert NotRecipient();
        settle();
        paid = amount;
        if (paid == 0) revert Tiny();
        uint256 before = pusd.balanceOf(address(this));
        oliver.borrow(paid);
        uint256 got = pusd.balanceOf(address(this)) - before;
        if (got < paid) paid = got;
        if (paid == 0) revert Tiny();
        require(pusd.transfer(recipient, paid), "PUSD");
        emit DrewPusd(recipient, paid);
    }

    /// Recipient/owner may repay Oliver debt with $PUSD (does not unlock V to market).
    function repayPusd(uint256 amount) external returns (uint256 repaid) {
        if (msg.sender != recipient && msg.sender != owner) revert NotRecipient();
        if (amount == 0) revert Tiny();
        require(pusd.transferFrom(msg.sender, address(this), amount), "PULL");
        require(pusd.approve(address(oliver), amount), "ALLOW");
        oliver.repay(amount);
        repaid = amount;
        emit Repaid(repaid);
    }

    /// Explicit ban: any attempt to pull V to an EOA/market reverts.
    function withdrawV(address, uint256) external pure {
        revert NoMarketSell();
    }

    function claimV(address, uint256) external pure {
        revert NoMarketSell();
    }

    function remaining() external view returns (uint256) {
        uint256 v = vested();
        return deposited > v ? deposited - v : 0;
    }

    function oliverCollateral() external view returns (uint256) {
        return oliver.collatV(address(this));
    }
}
