// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Minimal token surface shared by the two Lithe markets and the V converter.
interface IERC20LitheCutover {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
    function approve(address, uint256) external returns (bool);
}

/// Legacy Lithe market: old PUSD redeem -> old V.
interface ILegacyLitheMarket {
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
    function swapPusdToV(uint256 offer) external returns (uint256 ask, uint256 fee);
}

/// Canonical Lithe market: inventory V in -> market-minted PUSD out (no V mint).
interface ICanonicalLitheMarket {
    function vapurr() external view returns (address);
    function pusd() external view returns (address);
    function swapVToPusd(uint256 offer) external returns (uint256 ask, uint256 fee);
}

/// Inventory-only V conversion shared by direct-V and Lithe-PUSD cutover paths.
interface ILegacyVConversion {
    function legacyV() external view returns (address);
    function canonicalV() external view returns (address);
    function convert(uint256 amount) external returns (uint256 out);
}

/// Atomic Lithe-to-Lithe migration.
///
/// A holder does not receive a synthetic bridge credit. Their legacy PUSD is
/// redeemed through the legacy Lithe market, the resulting legacy V is exchanged
/// from pre-funded converter inventory, and that inventory V is swapped through
/// canonical Lithe to mint new PUSD. Both Lithe spreads apply; this contract
/// never receives V or PUSD mint authority, and canonical V supply does not grow.
contract LitheCutoverMigrator {
    ILegacyLitheMarket public immutable legacyMarket;
    ICanonicalLitheMarket public immutable canonicalMarket;
    ILegacyVConversion public immutable converter;
    IERC20LitheCutover public immutable legacyPusd;
    IERC20LitheCutover public immutable legacyV;
    IERC20LitheCutover public immutable canonicalPusd;
    IERC20LitheCutover public immutable canonicalV;

    event Migrated(
        address indexed account,
        uint256 legacyPusdIn,
        uint256 legacyVOut,
        uint256 canonicalVIn,
        uint256 canonicalPusdOut
    );

    constructor(address legacyMarket_, address canonicalMarket_, address converter_) {
        require(legacyMarket_ != address(0) && canonicalMarket_ != address(0) && converter_ != address(0), "TO");
        require(legacyMarket_ != canonicalMarket_, "MARKET");

        legacyMarket = ILegacyLitheMarket(legacyMarket_);
        canonicalMarket = ICanonicalLitheMarket(canonicalMarket_);
        converter = ILegacyVConversion(converter_);

        address legacyV_ = legacyMarket.vapurr();
        address legacyPusd_ = legacyMarket.pusd();
        address canonicalV_ = canonicalMarket.vapurr();
        address canonicalPusd_ = canonicalMarket.pusd();
        require(
            legacyV_ != address(0) && legacyPusd_ != address(0) && canonicalV_ != address(0)
                && canonicalPusd_ != address(0),
            "TO"
        );
        require(legacyV_ != canonicalV_ && legacyPusd_ != canonicalPusd_, "ASSET");
        require(converter.legacyV() == legacyV_ && converter.canonicalV() == canonicalV_, "CONVERTER");

        legacyPusd = IERC20LitheCutover(legacyPusd_);
        legacyV = IERC20LitheCutover(legacyV_);
        canonicalPusd = IERC20LitheCutover(canonicalPusd_);
        canonicalV = IERC20LitheCutover(canonicalV_);
    }

    /// Redeem legacy PUSD through old Lithe, convert the released V from inventory,
    /// then swap that V through new Lithe to mint canonical PUSD. Rolls back on failure.
    function migrate(uint256 legacyPusdIn) external returns (uint256 canonicalPusdOut) {
        require(legacyPusdIn > 0, "TINY");

        uint256 oldPusdBefore = legacyPusd.balanceOf(address(this));
        uint256 oldVBefore = legacyV.balanceOf(address(this));
        uint256 newVBefore = canonicalV.balanceOf(address(this));
        uint256 newPusdBefore = canonicalPusd.balanceOf(address(this));

        require(legacyPusd.transferFrom(msg.sender, address(this), legacyPusdIn), "PULL");
        require(legacyPusd.approve(address(legacyMarket), legacyPusdIn), "ALLOW");
        (uint256 legacyVOut,) = legacyMarket.swapPusdToV(legacyPusdIn);
        require(legacyPusd.balanceOf(address(this)) == oldPusdBefore, "LEGACY_PUSD");
        require(legacyV.balanceOf(address(this)) == oldVBefore + legacyVOut, "LEGACY_V");

        require(legacyV.approve(address(converter), legacyVOut), "ALLOW");
        uint256 canonicalVIn = converter.convert(legacyVOut);
        require(canonicalVIn == legacyVOut, "CONVERT");
        require(canonicalV.balanceOf(address(this)) == newVBefore + canonicalVIn, "VAPURR");

        require(canonicalV.approve(address(canonicalMarket), canonicalVIn), "ALLOW");
        (canonicalPusdOut,) = canonicalMarket.swapVToPusd(canonicalVIn);
        require(canonicalPusd.balanceOf(address(this)) == newPusdBefore + canonicalPusdOut, "PUSD");
        require(canonicalPusd.transfer(msg.sender, canonicalPusdOut), "PUSD");
        require(canonicalPusd.balanceOf(address(this)) == newPusdBefore, "PUSD");

        emit Migrated(msg.sender, legacyPusdIn, legacyVOut, canonicalVIn, canonicalPusdOut);
    }
}
