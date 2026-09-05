// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// ERC-1967 implementation slot helpers + UUPS upgrade entrypoint.
abstract contract UUPSUpgradeable {
    /// keccak256("eip1967.proxy.implementation") - 1
    bytes32 internal constant IMPLEMENTATION_SLOT =
        0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;

    event Upgraded(address indexed implementation);

    error UnauthorizedUpgrade();
    error InvalidImplementation();

    function _getImplementation() internal view returns (address impl) {
        bytes32 slot = IMPLEMENTATION_SLOT;
        assembly {
            impl := sload(slot)
        }
    }

    function _setImplementation(address newImplementation) private {
        if (newImplementation.code.length == 0) revert InvalidImplementation();
        bytes32 slot = IMPLEMENTATION_SLOT;
        assembly {
            sstore(slot, newImplementation)
        }
        emit Upgraded(newImplementation);
    }

    /// UUPS: only callable through the proxy; impl authorizes via `_authorizeUpgrade`.
    function upgradeToAndCall(address newImplementation, bytes memory data) public payable virtual {
        _authorizeUpgrade(newImplementation);
        _setImplementation(newImplementation);
        if (data.length > 0) {
            (bool ok, bytes memory ret) = newImplementation.delegatecall(data);
            if (!ok) {
                assembly {
                    revert(add(ret, 0x20), mload(ret))
                }
            }
        }
    }

    function proxiableUUID() external view virtual returns (bytes32) {
        // Only meaningful when called via delegatecall from the proxy.
        return IMPLEMENTATION_SLOT;
    }

    function _authorizeUpgrade(address newImplementation) internal virtual;
}
