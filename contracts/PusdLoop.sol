// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Isolated $PUSD credit vault. Euler-shaped, not an Euler fork.
/// Credit asset is $PUSD only. Collateral is $VAPURR plus supplied $PUSD.
/// Supply P, borrow P, loop under LTV. Utilization IRM. Liquidations.
/// No USDG. No ETH. No WETH. Lithe still drips on vault-held $PUSD.
/// Loop is recursive supply/borrow in one tx — virtual share mint, capped by vault cash (no invented depth).

interface IERC20 {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function approve(address, uint256) external returns (bool);
}

interface IMarket {
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
    function lunaRate() external view returns (uint256);
}

interface IRemittanceHook {
    function receiveRemittance(uint256 amount) external returns (bool);
}

interface IRunwayView {
    function surplus(uint256 balance) external view returns (uint256);
    function floor() external view returns (uint256);
}

contract PusdLoop {
    uint256 public constant DEC = 1e18;
    uint256 public constant YEAR = 365 days;
    uint256 public constant VIRTUAL = 1e6;
    uint256 public constant LTV_BPS = 8500;
    uint256 public constant LLTV_BPS = 9000;
    uint256 public constant LIQ_BONUS_BPS = 500;
    uint256 public constant RESERVE_BPS = 1000;
    uint256 public constant KINK = 9e17;
    /// Steady kink borrow APY once real $PUSD cash has filled the book.
    uint256 public constant BASE_SLOPE1 = 6e16;
    /// Cold-start kink borrow APY. Fades as exogenous cash arrives.
    uint256 public constant BOOT_SLOPE1 = 15e17;
    uint256 public constant SLOPE2 = 1e18;
    /// $PUSD cash at which boot slope has fully faded to BASE_SLOPE1.
    /// Looping does not raise cash â€” only real supply / Lithe drip does.
    uint256 public constant BOOT_CASH = 100_000 * DEC;
    uint256 public constant MAX_STEPS = 16;

    address public immutable owner;
    IMarket public immutable market;
    IERC20 public immutable vapurr;
    IERC20 public immutable pusd;

    uint256 public totalSupplyShares;
    uint256 public totalBorrowShares;
    uint256 public totalBorrowAssets;
    uint256 public lastAccrue;
    uint256 private _locked = 1;

    IRemittanceHook public remittance; // surplus sink (RemittanceSink / sPUSD)
    IRunwayView public runway; // optional floor gate before remit
    bool public remitOnAccrue; // when true, _accrue best-effort pushes reserve cash to sink

    mapping(address => uint256) public supplyShares;
    mapping(address => uint256) public borrowShares;
    mapping(address => uint256) public collatV;

    event Supply(address indexed user, uint256 assets, uint256 shares);
    event Withdraw(address indexed user, uint256 assets, uint256 shares);
    event DepositV(address indexed user, uint256 amt);
    event WithdrawV(address indexed user, uint256 amt);
    event Borrow(address indexed user, uint256 assets, uint256 shares);
    event Repay(address indexed user, uint256 assets, uint256 shares);
    event Loop(address indexed user, uint256 pusdIn, uint256 steps, uint256 supplied, uint256 debt);
    event Unwind(address indexed user, uint256 steps, uint256 repaid);
    event Liquidate(address indexed user, address indexed keeper, uint256 repay, uint256 vOut, uint256 pOut);
    event Accrue(uint256 interest, uint256 borrows);
    event RemittanceSet(address indexed sink, address indexed runway_, bool autoRemit);
    event Remitted(address indexed sink, uint256 assets, uint256 shares);

    modifier lock() {
        require(_locked == 1, "LOCK");
        _locked = 2;
        _;
        _locked = 1;
    }

    constructor(address market_) {
        require(market_ != address(0), "MKT");
        owner = msg.sender;
        market = IMarket(market_);
        address v = IMarket(market_).vapurr();
        address p = IMarket(market_).pusd();
        require(v != address(0) && p != address(0), "MKT");
        vapurr = IERC20(v);
        pusd = IERC20(p);
        lastAccrue = block.timestamp;
    }

    function accrue() external lock { _accrue(); }

    function setRemittance(address sink, address runway_, bool autoRemit) external {
        require(msg.sender == owner, "OWN");
        remittance = IRemittanceHook(sink);
        runway = IRunwayView(runway_);
        remitOnAccrue = autoRemit;
        emit RemittanceSet(sink, runway_, autoRemit);
    }

    /// Push owner reserve supply (RESERVE_BPS share claim) out as $PUSD to remittance sink.
    /// Runway floor (if set): only remits owner assets above floor. Floor may start at 0.
    function remitReserve(uint256 assets) public lock returns (uint256 sent) {
        _accrue();
        sent = _remitReserve(assets);
    }

    function _remitReserve(uint256 assets) internal returns (uint256 sent) {
        require(address(remittance) != address(0), "REMIT");
        require(assets > 0, "TINY");
        uint256 ownerAssets = _assetsOf(owner);
        if (address(runway) != address(0)) {
            uint256 fl = runway.floor();
            if (ownerAssets <= fl) return 0;
            uint256 above = ownerAssets - fl;
            if (assets > above) assets = above;
        }
        if (assets > ownerAssets) assets = ownerAssets;
        uint256 cash = pusd.balanceOf(address(this));
        if (assets > cash) assets = cash;
        if (assets == 0) return 0;
        uint256 sh = _supplySharesForAssets(assets);
        uint256 have = supplyShares[owner];
        if (sh > have) {
            sh = have;
            assets = _assetsFromSupplyShares(sh);
        }
        require(sh > 0 && assets > 0, "TINY");
        require(pusd.balanceOf(address(this)) >= assets, "CASH");
        supplyShares[owner] = have - sh;
        totalSupplyShares -= sh;
        require(pusd.approve(address(remittance), assets), "ALLOW");
        require(remittance.receiveRemittance(assets), "SINK");
        emit Remitted(address(remittance), assets, sh);
        return assets;
    }

    function supply(uint256 amt) external lock {
        _accrue();
        uint256 got = _pull(pusd, amt);
        _mintSupply(msg.sender, got, true);
    }

    function withdraw(uint256 amt) external lock {
        _accrue();
        require(amt > 0, "TINY");
        uint256 sh = _supplySharesForAssets(amt);
        uint256 have = supplyShares[msg.sender];
        if (sh > have) {
            sh = have;
            amt = _assetsFromSupplyShares(sh);
        }
        require(sh > 0 && amt > 0, "TINY");
        require(pusd.balanceOf(address(this)) >= amt, "CASH");
        supplyShares[msg.sender] = have - sh;
        totalSupplyShares -= sh;
        _requireLtv(msg.sender);
        require(pusd.transfer(msg.sender, amt), "PUSD");
        emit Withdraw(msg.sender, amt, sh);
    }

    function depositV(uint256 amt) external lock {
        _accrue();
        uint256 got = _pull(vapurr, amt);
        collatV[msg.sender] += got;
        emit DepositV(msg.sender, got);
    }

    function withdrawV(uint256 amt) external lock {
        _accrue();
        require(amt > 0, "TINY");
        uint256 have = collatV[msg.sender];
        require(have >= amt, "VAPURR");
        collatV[msg.sender] = have - amt;
        _requireLtv(msg.sender);
        require(vapurr.transfer(msg.sender, amt), "VAPURR");
        emit WithdrawV(msg.sender, amt);
    }

    function borrow(uint256 amt) external lock {
        _accrue();
        require(amt > 0, "TINY");
        require(pusd.balanceOf(address(this)) >= amt, "CASH");
        uint256 sh = _debtSharesFromAssets(amt);
        require(sh > 0, "TINY");
        borrowShares[msg.sender] += sh;
        totalBorrowShares += sh;
        totalBorrowAssets += amt;
        _requireLtv(msg.sender);
        require(pusd.transfer(msg.sender, amt), "PUSD");
        emit Borrow(msg.sender, amt, sh);
    }

    function repay(uint256 amt) external lock {
        _accrue();
        uint256 debt = _debtOf(msg.sender);
        require(debt > 0, "DEBT");
        if (amt > debt) amt = debt;
        uint256 got = _pull(pusd, amt);
        if (got > debt) got = debt;
        uint256 sh = _debtSharesForAssets(got);
        uint256 have = borrowShares[msg.sender];
        if (sh > have || got == debt) {
            sh = have;
            totalBorrowAssets -= debt;
            borrowShares[msg.sender] = 0;
            totalBorrowShares -= have;
            emit Repay(msg.sender, debt, have);
            return;
        }
        require(sh > 0, "TINY");
        borrowShares[msg.sender] = have - sh;
        totalBorrowShares -= sh;
        totalBorrowAssets -= got;
        emit Repay(msg.sender, got, sh);
    }

    /// Recursive supply/borrow. Tokens never leave; utilization rises.
    /// Looping still virtual share mint but cannot invent depth past cash.
    function loop(uint256 pusdIn, uint256 steps) external lock {
        _accrue();
        require(steps <= MAX_STEPS, "STEP");
        if (pusdIn > 0) {
            uint256 got = _pull(pusd, pusdIn);
            _mintSupply(msg.sender, got, true);
        }
        for (uint256 i = 0; i < steps; i++) {
            uint256 room = _room(msg.sender);
            uint256 cash = pusd.balanceOf(address(this));
            if (room > cash) room = cash;
            room = (room * 999) / 1000;
            if (room < 1e12) break;
            uint256 dSh = _debtSharesFromAssets(room);
            uint256 sSh = _supplySharesFromAssets(room);
            if (dSh == 0 || sSh == 0) break;
            borrowShares[msg.sender] += dSh;
            totalBorrowShares += dSh;
            totalBorrowAssets += room;
            supplyShares[msg.sender] += sSh;
            totalSupplyShares += sSh;
        }
        _requireLtv(msg.sender);
        emit Loop(msg.sender, pusdIn, steps, _assetsOf(msg.sender), _debtOf(msg.sender));
    }

    /// Burn supplied PUSD against debt. Cash stays. Deleverage.
    function unwind(uint256 steps) external lock {
        _accrue();
        require(steps > 0 && steps <= MAX_STEPS, "STEP");
        uint256 repaid;
        for (uint256 i = 0; i < steps; i++) {
            uint256 debt = _debtOf(msg.sender);
            uint256 supplied = _assetsOf(msg.sender);
            uint256 pay = debt < supplied ? debt : supplied;
            if (pay < 1e12) break;
            uint256 sSh = _supplySharesForAssets(pay);
            uint256 dSh = _debtSharesForAssets(pay);
            uint256 sHave = supplyShares[msg.sender];
            uint256 dHave = borrowShares[msg.sender];
            if (sSh > sHave) sSh = sHave;
            if (dSh > dHave || pay == debt) dSh = dHave;
            if (sSh == 0 || dSh == 0) break;
            supplyShares[msg.sender] = sHave - sSh;
            totalSupplyShares -= sSh;
            if (dSh == dHave) {
                totalBorrowAssets -= debt;
                borrowShares[msg.sender] = 0;
                totalBorrowShares -= dHave;
                repaid += debt;
                break;
            }
            borrowShares[msg.sender] = dHave - dSh;
            totalBorrowShares -= dSh;
            totalBorrowAssets -= pay;
            repaid += pay;
        }
        emit Unwind(msg.sender, steps, repaid);
    }

    function liquidate(address user, uint256 repayAmt) external lock {
        require(user != address(0) && user != msg.sender, "USER");
        _accrue();
        require(_liquidatable(user), "LIQ");
        uint256 debt = _debtOf(user);
        require(debt > 0 && repayAmt > 0, "DEBT");
        uint256 maxRepay = (_collatValue(user) * 10_000) / (10_000 + LIQ_BONUS_BPS);
        if (repayAmt > maxRepay) repayAmt = maxRepay;
        if (repayAmt > debt) repayAmt = debt;
        require(repayAmt > 0, "TINY");
        uint256 got = _pull(pusd, repayAmt);
        if (got > debt) got = debt;
        _burnDebt(user, got, debt);
        (uint256 vOut, uint256 pOut) = _seize(user, (got * (10_000 + LIQ_BONUS_BPS)) / 10_000);
        if (vOut > 0) require(vapurr.transfer(msg.sender, vOut), "VAPURR");
        if (pOut > 0) {
            require(pusd.balanceOf(address(this)) >= pOut, "CASH");
            require(pusd.transfer(msg.sender, pOut), "PUSD");
        }
        emit Liquidate(user, msg.sender, got, vOut, pOut);
    }

    function _burnDebt(address u, uint256 got, uint256 debt) internal {
        uint256 dSh = _debtSharesForAssets(got);
        uint256 dHave = borrowShares[u];
        if (dSh > dHave || got == debt) {
            totalBorrowAssets -= debt;
            borrowShares[u] = 0;
            totalBorrowShares -= dHave;
            return;
        }
        require(dSh > 0, "TINY");
        borrowShares[u] = dHave - dSh;
        totalBorrowShares -= dSh;
        totalBorrowAssets -= got;
    }

    function _seize(address u, uint256 seizeP) internal returns (uint256 vOut, uint256 pOut) {
        uint256 px = _px();
        uint256 vHave = collatV[u];
        uint256 vVal = px > 0 ? (vHave * px) / DEC : 0;
        if (vVal >= seizeP && px > 0) {
            vOut = (seizeP * DEC) / px;
            if (vOut > vHave) vOut = vHave;
            collatV[u] = vHave - vOut;
            return (vOut, 0);
        }
        vOut = vHave;
        collatV[u] = 0;
        if (seizeP <= vVal) return (vOut, 0);
        uint256 rest = seizeP - vVal;
        uint256 pHave = _assetsOf(u);
        pOut = rest < pHave ? rest : pHave;
        if (pOut == 0) return (vOut, 0);
        uint256 sSh = _supplySharesForAssets(pOut);
        uint256 sHave = supplyShares[u];
        if (sSh > sHave) {
            sSh = sHave;
            pOut = _assetsFromSupplyShares(sSh);
        }
        if (sSh > 0) {
            supplyShares[u] = sHave - sSh;
            totalSupplyShares -= sSh;
        }
    }

    struct Snap {
        uint256 cash;
        uint256 totalSupplyAssets;
        uint256 totalBorrowAssets_;
        uint256 util;
        uint256 borrowApyBps;
        uint256 supplyApyBps;
        uint256 ltvBps;
        uint256 lltvBps;
        uint256 px;
        uint256 supplied;
        uint256 collatV_;
        uint256 debt;
        uint256 collatValue;
        uint256 health;
        uint256 vapurrBal;
        uint256 pusdBal;
        address vapurrToken;
        address pusdToken;
        address market_;
        uint256 room;
        uint256 bootBps;
        uint256 flowWad;
        uint256 cashTarget;
    }

    function snapshot(address a) external view returns (Snap memory s) {
        uint256 cash = pusd.balanceOf(address(this));
        uint256 borrows = _previewBorrows();
        uint256 assets = cash + borrows;
        uint256 rate = _irm(borrows, assets, cash);
        s.cash = cash;
        s.totalSupplyAssets = assets;
        s.totalBorrowAssets_ = borrows;
        s.util = assets == 0 ? 0 : (borrows * DEC) / assets;
        s.borrowApyBps = (rate * 10_000) / DEC;
        s.supplyApyBps = assets == 0
            ? 0
            : (rate * s.util / DEC) * (10_000 - RESERVE_BPS) / 10_000 * 10_000 / DEC;
        s.ltvBps = LTV_BPS;
        s.lltvBps = LLTV_BPS;
        s.px = market.lunaRate();
        s.supplied = _assetsOfPreview(a, cash, borrows);
        s.collatV_ = collatV[a];
        s.debt = _debtOfPreview(a, borrows);
        s.collatValue = _collatValuePreview(a, s.supplied, s.px);
        s.health = s.debt == 0 ? 100 * DEC : (s.collatValue * LLTV_BPS / 10_000) * DEC / s.debt;
        s.vapurrBal = vapurr.balanceOf(a);
        s.pusdBal = pusd.balanceOf(a);
        s.vapurrToken = address(vapurr);
        s.pusdToken = address(pusd);
        s.market_ = address(market);
        uint256 maxDebt = (s.collatValue * LTV_BPS) / 10_000;
        s.room = s.debt >= maxDebt ? 0 : maxDebt - s.debt;
        s.bootBps = (_slope1(cash) * 10_000) / DEC;
        s.flowWad = _flow(cash);
        s.cashTarget = BOOT_CASH;
    }

    function _accrue() internal {
        uint256 dt = block.timestamp - lastAccrue;
        lastAccrue = block.timestamp;
        if (dt == 0) return;
        if (dt > 2 * YEAR) dt = 2 * YEAR;
        uint256 b = totalBorrowAssets;
        if (b == 0) return;
        uint256 cash = pusd.balanceOf(address(this));
        uint256 rate = _irm(b, cash + b, cash);
        uint256 interest = ((b * rate) / DEC * dt) / YEAR;
        if (interest == 0) return;
        totalBorrowAssets = b + interest;
        uint256 fee = (interest * RESERVE_BPS) / 10_000;
        if (fee > 0 && totalSupplyShares > 0) {
            uint256 ts = totalSupplyShares + VIRTUAL;
            uint256 ta = cash + totalBorrowAssets + VIRTUAL;
            if (ta > fee) {
                uint256 sh = (fee * ts) / (ta - fee);
                if (sh > 0) {
                    supplyShares[owner] += sh;
                    totalSupplyShares += sh;
                }
            }
        }
        emit Accrue(interest, totalBorrowAssets);
        // Remittance hook: best-effort push reserve cash to sink after owner share mint.
        if (remitOnAccrue && address(remittance) != address(0) && fee > 0) {
            _remitReserve(fee);
        }
    }

    function _flow(uint256 cash) internal pure returns (uint256) {
        if (cash >= BOOT_CASH) return DEC;
        return (cash * DEC) / BOOT_CASH;
    }

    function _slope1(uint256 cash) internal pure returns (uint256) {
        uint256 f = _flow(cash);
        return BASE_SLOPE1 + ((BOOT_SLOPE1 - BASE_SLOPE1) * (DEC - f)) / DEC;
    }

    /// Borrow rate. Wild while cash is thin. Fades to 6% kink as real $PUSD shows up.
    /// Recursive loop does not raise cash â€” only exogenous supply / Lithe does.
    function _irm(uint256 borrows, uint256 assets, uint256 cash) internal pure returns (uint256) {
        if (assets == 0 || borrows == 0) return 0;
        uint256 s1 = _slope1(cash);
        if (borrows >= assets) return s1 + SLOPE2;
        uint256 util = (borrows * DEC) / assets;
        if (util <= KINK) return (s1 * util) / KINK;
        return s1 + (SLOPE2 * (util - KINK)) / (DEC - KINK);
    }

    function _pull(IERC20 t, uint256 amt) internal returns (uint256 got) {
        require(amt > 0, "TINY");
        uint256 before = t.balanceOf(address(this));
        require(t.transferFrom(msg.sender, address(this), amt), "PULL");
        got = t.balanceOf(address(this)) - before;
        require(got > 0, "TINY");
    }

    function _px() internal view returns (uint256) {
        return market.lunaRate();
    }

    function _mintSupply(address u, uint256 assets, bool alreadyIn) internal returns (uint256 sh) {
        uint256 ta = pusd.balanceOf(address(this)) + totalBorrowAssets + VIRTUAL;
        if (alreadyIn) {
            require(ta > assets, "TINY");
            ta -= assets;
        }
        sh = (assets * (totalSupplyShares + VIRTUAL)) / ta;
        require(sh > 0, "TINY");
        supplyShares[u] += sh;
        totalSupplyShares += sh;
        emit Supply(u, assets, sh);
    }

    function _supplySharesFromAssets(uint256 amt) internal view returns (uint256) {
        return (amt * (totalSupplyShares + VIRTUAL)) / (pusd.balanceOf(address(this)) + totalBorrowAssets + VIRTUAL);
    }

    function _supplySharesForAssets(uint256 amt) internal view returns (uint256) {
        uint256 ts = totalSupplyShares + VIRTUAL;
        uint256 ta = pusd.balanceOf(address(this)) + totalBorrowAssets + VIRTUAL;
        return (amt * ts + ta - 1) / ta;
    }

    function _assetsFromSupplyShares(uint256 sh) internal view returns (uint256) {
        if (sh == 0) return 0;
        return (sh * (pusd.balanceOf(address(this)) + totalBorrowAssets + VIRTUAL)) / (totalSupplyShares + VIRTUAL);
    }

    function _debtSharesFromAssets(uint256 amt) internal view returns (uint256) {
        uint256 ts = totalBorrowShares + VIRTUAL;
        uint256 ta = totalBorrowAssets + VIRTUAL;
        return (amt * ts + ta - 1) / ta;
    }

    function _debtSharesForAssets(uint256 amt) internal view returns (uint256) {
        uint256 ts = totalBorrowShares + VIRTUAL;
        uint256 ta = totalBorrowAssets + VIRTUAL;
        return (amt * ts) / ta;
    }

    function _assetsOf(address u) internal view returns (uint256) {
        return _assetsFromSupplyShares(supplyShares[u]);
    }

    function _debtOf(address u) internal view returns (uint256) {
        uint256 sh = borrowShares[u];
        if (sh == 0) return 0;
        return (sh * (totalBorrowAssets + VIRTUAL)) / (totalBorrowShares + VIRTUAL);
    }

    function _previewBorrows() internal view returns (uint256) {
        uint256 b = totalBorrowAssets;
        uint256 dt = block.timestamp - lastAccrue;
        if (dt == 0 || b == 0) return b;
        if (dt > 2 * YEAR) dt = 2 * YEAR;
        uint256 cash = pusd.balanceOf(address(this));
        uint256 rate = _irm(b, cash + b, cash);
        uint256 interest = ((b * rate) / DEC * dt) / YEAR;
        return b + interest;
    }

    function _assetsOfPreview(address u, uint256 cash, uint256 borrows) internal view returns (uint256) {
        uint256 sh = supplyShares[u];
        if (sh == 0) return 0;
        return (sh * (cash + borrows + VIRTUAL)) / (totalSupplyShares + VIRTUAL);
    }

    function _debtOfPreview(address u, uint256 borrows) internal view returns (uint256) {
        uint256 sh = borrowShares[u];
        if (sh == 0) return 0;
        return (sh * (borrows + VIRTUAL)) / (totalBorrowShares + VIRTUAL);
    }

    function _collatValue(address u) internal view returns (uint256) {
        return _collatValuePreview(u, _assetsOf(u), _px());
    }

    function _collatValuePreview(address u, uint256 supplied, uint256 px) internal view returns (uint256) {
        uint256 v = collatV[u];
        uint256 vVal = (px > 0 && v > 0) ? (v * px) / DEC : 0;
        return vVal + supplied;
    }

    function _room(address u) internal view returns (uint256) {
        uint256 maxDebt = (_collatValue(u) * LTV_BPS) / 10_000;
        uint256 debt = _debtOf(u);
        return debt >= maxDebt ? 0 : maxDebt - debt;
    }

    function _requireLtv(address u) internal view {
        uint256 debt = _debtOf(u);
        if (debt == 0) return;
        require(debt * 10_000 <= _collatValue(u) * LTV_BPS, "LTV");
    }

    function _liquidatable(address u) internal view returns (bool) {
        uint256 debt = _debtOf(u);
        if (debt == 0) return false;
        return debt * 10_000 > _collatValue(u) * LLTV_BPS;
    }
}
