// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @dev ERC-1967 implementation slot
bytes32 constant IMPLEMENTATION_SLOT = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;
/// @dev ERC-1967 admin slot
bytes32 constant ADMIN_SLOT = 0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103;

error ZeroAddr();

library ERC1967Store {
    function getImplementation() internal view returns (address impl) {
        bytes32 slot = IMPLEMENTATION_SLOT;
        assembly {
            impl := sload(slot)
        }
    }

    function setImplementation(address impl) internal {
        if (impl == address(0)) revert ZeroAddr();
        bytes32 slot = IMPLEMENTATION_SLOT;
        assembly {
            sstore(slot, impl)
        }
    }

    function getAdmin() internal view returns (address adm) {
        bytes32 slot = ADMIN_SLOT;
        assembly {
            adm := sload(slot)
        }
    }

    function setAdmin(address adm) internal {
        if (adm == address(0)) revert ZeroAddr();
        bytes32 slot = ADMIN_SLOT;
        assembly {
            sstore(slot, adm)
        }
    }
}

/// Minimal ERC1967 proxy. Vanity Lithe address is this proxy (CREATE / CREATE2), not the impl.
contract ERC1967Proxy {
    event Upgraded(address indexed implementation);
    event AdminChanged(address previousAdmin, address newAdmin);

    constructor(address implementation_, bytes memory initData) payable {
        ERC1967Store.setImplementation(implementation_);
        ERC1967Store.setAdmin(msg.sender);
        emit Upgraded(implementation_);
        emit AdminChanged(address(0), msg.sender);
        if (initData.length > 0) {
            (bool ok, bytes memory ret) = implementation_.delegatecall(initData);
            if (!ok) {
                assembly {
                    revert(add(ret, 0x20), mload(ret))
                }
            }
        }
    }

    fallback() external payable {
        _delegate(ERC1967Store.getImplementation());
    }

    receive() external payable {
        _delegate(ERC1967Store.getImplementation());
    }

    function _delegate(address impl) internal {
        assembly {
            calldatacopy(0, 0, calldatasize())
            let result := delegatecall(gas(), impl, 0, calldatasize(), 0, 0)
            returndatacopy(0, 0, returndatasize())
            switch result
            case 0 { revert(0, returndatasize()) }
            default { return(0, returndatasize()) }
        }
    }
}
