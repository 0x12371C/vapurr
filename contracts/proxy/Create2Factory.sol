// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Thin CREATE2 deployer for vanity proxy mining (testnet 46630 prep).
/// computeAddress matches forge/cast create2. No privileged roles.
contract Create2Factory {
    event Deployed(address indexed addr, bytes32 indexed salt);

    error Create2Failed();

    function deploy(bytes32 salt, bytes memory initCode) external payable returns (address addr) {
        assembly {
            addr := create2(callvalue(), add(initCode, 0x20), mload(initCode), salt)
        }
        if (addr == address(0)) revert Create2Failed();
        emit Deployed(addr, salt);
    }

    function computeAddress(bytes32 salt, bytes32 initCodeHash) external view returns (address) {
        return address(uint160(uint256(keccak256(abi.encodePacked(bytes1(0xff), address(this), salt, initCodeHash)))));
    }
}
