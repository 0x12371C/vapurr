// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Minimal initializer guard (OZ-shaped, no dependency).
abstract contract Initializable {
    uint8 private _initialized;
    bool private _initializing;

    error InvalidInitialization();
    error NotInitializing();

    event Initialized(uint64 version);

    modifier initializer() {
        bool isTopLevel = !_initializing;
        uint8 initialized = _initialized;
        if (isTopLevel) {
            if (initialized >= 1) revert InvalidInitialization();
            _initialized = 1;
            _initializing = true;
        } else if (initialized != 0) {
            revert InvalidInitialization();
        }
        _;
        if (isTopLevel) {
            _initializing = false;
            emit Initialized(1);
        }
    }

    modifier onlyInitializing() {
        if (!_initializing) revert NotInitializing();
        _;
    }

    function _getInitializedVersion() internal view returns (uint8) {
        return _initialized;
    }

    function _disableInitializers() internal {
        if (_initializing) revert InvalidInitialization();
        if (_initialized != type(uint8).max) {
            _initialized = type(uint8).max;
            emit Initialized(type(uint8).max);
        }
    }
}
