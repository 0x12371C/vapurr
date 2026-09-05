// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Single-minter mint authority for canonical $VAPURR (Fed-side / post-unify).
/// Invariant: **zero or one** minter — never two silent printers.
/// - `minter == address(0)` → mint disabled (revoked).
/// - `minter != 0` → only that address may `mint` / `setMinter`.
/// Intended sole inflation role after genesis: Fed gV / RebasePolicy path (`gVAPURR.accrue`).
/// Market inventory redeem must NOT hold this role — unwrap via take/give only.
interface IVapurrMinter {
    function minter() external view returns (address);

    /// Transfer or revoke mint rights. Callable only by the current minter.
    /// Pass `address(0)` to revoke (zero minters). Pass a non-zero address to set the sole minter.
    function setMinter(address m) external;

    function mint(address to, uint256 amt) external;
}