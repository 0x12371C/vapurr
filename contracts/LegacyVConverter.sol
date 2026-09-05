// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Fixed 1:1 legacy-V cutover claim. The contract is deliberately inventory-only:
/// it cannot mint canonical V, release legacy V, or change either token address.
interface IERC20Cutover {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
}

contract LegacyVConverter {
    IERC20Cutover public immutable legacyV;
    IERC20Cutover public immutable canonicalV;
    uint256 public converted;

    event Funded(address indexed from, uint256 amount);
    event Converted(address indexed account, uint256 legacyIn, uint256 canonicalOut);

    constructor(address legacyV_, address canonicalV_) {
        require(legacyV_ != address(0) && canonicalV_ != address(0) && legacyV_ != canonicalV_, "TO");
        legacyV = IERC20Cutover(legacyV_);
        canonicalV = IERC20Cutover(canonicalV_);
    }

    /// Permissionless top-up of pre-existing canonical V. No mint authority is required or accepted.
    function fund(uint256 amount) external {
        require(amount > 0, "TINY");
        uint256 beforeCash = canonicalV.balanceOf(address(this));
        require(canonicalV.transferFrom(msg.sender, address(this), amount), "PULL");
        require(canonicalV.balanceOf(address(this)) == beforeCash + amount, "VAPURR");
        emit Funded(msg.sender, amount);
    }

    function available() public view returns (uint256) {
        return canonicalV.balanceOf(address(this));
    }

    function legacyLocked() external view returns (uint256) {
        return legacyV.balanceOf(address(this));
    }

    /// 1:1 nominal conversion. Received legacy V remains permanently locked;
    /// there is intentionally no pause, owner, recover, or sweep function.
    function convert(uint256 amount) external returns (uint256 out) {
        require(amount > 0 && available() >= amount, "INV");
        uint256 beforeLegacy = legacyV.balanceOf(address(this));
        require(legacyV.transferFrom(msg.sender, address(this), amount), "PULL");
        require(legacyV.balanceOf(address(this)) == beforeLegacy + amount, "LEGACY");
        require(canonicalV.transfer(msg.sender, amount), "VAPURR");
        converted += amount;
        emit Converted(msg.sender, amount, amount);
        return amount;
    }
}
