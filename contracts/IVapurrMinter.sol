// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Dual V mint authority for canonical $VAPURR (Fed-side / post-unify).
///
/// Two intentional printers (documented, not silent):
/// 1. Policy minter (`minter`) - gV / RebasePolicy inflate to stakers (dynamic 1-9%/yr).
/// 2. Market minter (`marketMinter`) - Lithe seigniorage: mint V on PUSD redeem, burn V on PUSD expand.
///
/// - `minter == address(0)` -> policy mint disabled (revoked).
/// - `marketMinter == address(0)` -> Lithe cannot mint/burn V (seigniorage offline).
/// - Only the current policy minter may `setMinter` / `setMarketMinter`.
/// BrowserStream never holds either role - transfers already-minted float only.
/// See docs/econ/MINT_AUTHORITY.md.
interface IVapurrMinter {
    function minter() external view returns (address);
    function marketMinter() external view returns (address);

    /// Transfer or revoke policy mint rights. Callable only by the current policy minter.
    /// Pass `address(0)` to revoke policy minting.
    function setMinter(address m) external;

    /// Assign or revoke Lithe seigniorage mint/burn rights. Callable only by the current policy minter.
    function setMarketMinter(address m) external;

    function mint(address to, uint256 amt) external;
}
