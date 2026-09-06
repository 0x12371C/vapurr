// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// Gasless meta-transactions for vapurr. ERC-2771-shaped minimal forwarder:
/// a user signs a ForwardRequest (EIP-712) for free, an authorized relayer
/// submits it on-chain and pays the gas. `executeBatch` is the actual
/// savings mechanism — batching N requests into one transaction amortizes
/// the fixed ~21,000 gas base-tx cost across all of them instead of paying
/// it N times, which is where "up to 50% less gas" comes from. It is not
/// a third-party compute-offload service; it is arithmetic.
///
/// Security model: `execute`/`executeBatch` are gated to `authorizedRelayers`
/// — the calldata itself only ever moves a user's OWN pre-signed intent,
/// but only an authorized relayer address may submit it. The relayer set
/// is owner-controlled and revocable on-chain without redeploying: a
/// compromised relayer hot key gets pulled here, not by shipping a new
/// contract. Owner should be a multisig, not an EOA.
///
/// Per-request replay protection is a strictly increasing nonce per
/// `from` address (checked and incremented atomically), plus a
/// `validUntil` deadline so a stale signed request cannot be replayed
/// indefinitely if the relayer ever holds onto one.
contract VapurrForwarder {
    struct ForwardRequest {
        address from;
        address to;
        uint256 value;
        uint256 gas;
        uint256 nonce;
        bytes data;
        uint256 validUntil;
    }

    bytes32 public constant DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");

    bytes32 public constant FORWARD_REQUEST_TYPEHASH = keccak256(
        "ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data,uint256 validUntil)"
    );

    bytes32 public immutable DOMAIN_SEPARATOR;

    address public owner;
    mapping(address => bool) public authorizedRelayers;
    mapping(address => uint256) public nonces;

    event RelayerAuthorized(address indexed relayer, bool allowed);
    event OwnerChanged(address indexed previous, address indexed next);
    /// `to` is deliberately NOT indexed and `gasUsed` is dropped — "show my
    /// history" (filter by `from`) is the query that matters; a second
    /// indexed topic and a data word both cost real gas on every single
    /// item. Trimmed for that reason, not for free: this is less queryable
    /// than the original, on purpose, to buy back gas.
    event Executed(address indexed from, address to, uint256 nonce, bool success);

    modifier onlyOwner() {
        require(msg.sender == owner, "OWNER");
        _;
    }

    modifier onlyRelayer() {
        require(authorizedRelayers[msg.sender], "RELAYER");
        _;
    }

    constructor(address owner_) {
        require(owner_ != address(0), "OWNER");
        owner = owner_;
        DOMAIN_SEPARATOR = keccak256(
            abi.encode(
                DOMAIN_TYPEHASH,
                keccak256(bytes("VapurrForwarder")),
                keccak256(bytes("1")),
                block.chainid,
                address(this)
            )
        );
    }

    // ─── Owner admin ──────────────────────────────────────────────────────

    function setRelayer(address relayer, bool allowed) external onlyOwner {
        require(relayer != address(0), "RELAYER");
        authorizedRelayers[relayer] = allowed;
        emit RelayerAuthorized(relayer, allowed);
    }

    function transferOwner(address next) external onlyOwner {
        require(next != address(0), "OWNER");
        emit OwnerChanged(owner, next);
        owner = next;
    }

    // ─── Signature verification ───────────────────────────────────────────

    /// The one canonical struct hash. `execute` and `executeBatch` both
    /// route through this — computing it two different ways in two places
    /// is exactly the kind of drift that would silently make a request
    /// valid on one path and rejected (or worse, differently interpreted)
    /// on the other.
    function _hashFields(
        address from,
        address to,
        uint256 value,
        uint256 gas,
        uint256 nonce,
        bytes calldata data,
        uint256 validUntil
    ) private pure returns (bytes32) {
        return keccak256(
            abi.encode(FORWARD_REQUEST_TYPEHASH, from, to, value, gas, nonce, keccak256(data), validUntil)
        );
    }

    function _hashRequest(ForwardRequest calldata req) private pure returns (bytes32) {
        return _hashFields(req.from, req.to, req.value, req.gas, req.nonce, req.data, req.validUntil);
    }

    /// The EIP-712 digest a wallet actually signs for `req`.
    function digest(ForwardRequest calldata req) public view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR, _hashRequest(req)));
    }

    function verify(ForwardRequest calldata req, bytes calldata sig) public view returns (bool) {
        if (block.timestamp > req.validUntil) return false;
        if (nonces[req.from] != req.nonce) return false;
        return _recover(digest(req), sig) == req.from;
    }

    /// The N/2 canonical-low-s ceiling (secp256k1 order / 2), shared by
    /// both recovery paths below.
    uint256 private constant _S_CEILING = 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0;

    /// `execute()`'s path — standard 65-byte (r, s, v). Reverts on a
    /// malformed signature, which is correct here: there is only one
    /// request in this call, so "the signature was garbage" and "the
    /// request is invalid" are the same outcome.
    function _recover(bytes32 h, bytes calldata sig) private pure returns (address) {
        require(sig.length == 65, "SIG_LEN");
        bytes32 r;
        bytes32 s;
        uint8 v;
        assembly {
            r := calldataload(sig.offset)
            s := calldataload(add(sig.offset, 32))
            v := byte(0, calldataload(add(sig.offset, 64)))
        }
        if (v < 27) v += 27;
        require(v == 27 || v == 28, "SIG_V");
        require(uint256(s) <= _S_CEILING, "SIG_S");
        address signer = ecrecover(h, v, r, s);
        require(signer != address(0), "SIG_BAD");
        return signer;
    }

    /// `executeBatch()`'s path — EIP-2098 compact (64-byte: r, then s with
    /// its recovery bit packed into s's otherwise-always-zero top bit,
    /// which canonical low-s guarantees is free). 32 bytes of calldata
    /// saved per item over the standard encoding — no wallet-visible
    /// change, since the relayer converts the user's ordinary 65-byte
    /// wallet signature to this form itself before batching (see
    /// `crates/vapurr-relay/src/eip712.rs::to_compact`).
    ///
    /// NEVER reverts on malformed input — returns `address(0)` instead,
    /// same as `ecrecover` itself does on failure. `execute()`'s `_recover`
    /// can afford to revert because there is nothing else in that call to
    /// protect; here, one item's garbage signature must fail ONLY that
    /// item (see `_runFields`), not the whole batch everyone else is
    /// riding in.
    function _recoverCompact(bytes32 h, bytes calldata sig) private pure returns (address) {
        if (sig.length != 64) return address(0);
        bytes32 r;
        bytes32 vs;
        assembly {
            r := calldataload(sig.offset)
            vs := calldataload(add(sig.offset, 32))
        }
        uint8 v = uint8((uint256(vs) >> 255) + 27);
        bytes32 s = bytes32(uint256(vs) & _S_CEILING_MASK);
        if (uint256(s) > _S_CEILING) return address(0);
        return ecrecover(h, v, r, s);
    }

    uint256 private constant _S_CEILING_MASK = 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;

    // ─── Execution ────────────────────────────────────────────────────────

    function _run(ForwardRequest calldata req, bytes calldata sig) private returns (bool success, bytes memory ret) {
        require(block.timestamp <= req.validUntil, "EXPIRED");
        require(nonces[req.from] == req.nonce, "NONCE");
        require(_recover(digest(req), sig) == req.from, "SIG");

        nonces[req.from] = req.nonce + 1;

        // ERC-2771 convention: append the true sender so a `data`-recipient
        // contract that trusts this forwarder can recover it via its own
        // `_msgSender()` override. A recipient that does not know about
        // that convention just sees this forwarder as msg.sender, which is
        // safe as long as it does not gate on msg.sender for anything the
        // forwarder itself should not be trusted to do on a user's behalf.
        (success, ret) = req.to.call{value: req.value, gas: req.gas}(abi.encodePacked(req.data, req.from));

        emit Executed(req.from, req.to, req.nonce, success);
    }

    function execute(ForwardRequest calldata req, bytes calldata sig)
        external
        onlyRelayer
        returns (bool success, bytes memory ret)
    {
        return _run(req, sig);
    }

    /// The actual gas-savings path: N requests, one transaction, one paid
    /// base-tx overhead instead of N. Flat parallel arrays rather than an
    /// array of structs — cheaper calldata and a simpler encoder on the
    /// relayer side; semantically identical to N calls to `execute`.
    /// Reverts only on an array-length mismatch; an individual request's
    /// own failure (bad sig, stale nonce, expired, or the call itself
    /// reverting) is reported per-item in `successes` and does not abort
    /// the rest of the batch — one bad or lagging signature must not be
    /// able to grief everyone else riding in the same batch.
    function executeBatch(
        address[] calldata froms,
        address[] calldata tos,
        uint256[] calldata values,
        uint256[] calldata gases,
        uint256[] calldata nonceList,
        bytes[] calldata datas,
        uint256[] calldata validUntils,
        bytes[] calldata sigs
    ) external onlyRelayer returns (bool[] memory successes) {
        uint256 n = froms.length;
        require(
            tos.length == n && values.length == n && gases.length == n && nonceList.length == n
                && datas.length == n && validUntils.length == n && sigs.length == n,
            "LEN"
        );

        successes = new bool[](n);
        for (uint256 i = 0; i < n; i++) {
            (bool ok, ) = _runFields(
                froms[i], tos[i], values[i], gases[i], nonceList[i], datas[i], validUntils[i], sigs[i]
            );
            successes[i] = ok;
        }
    }

    function _runFields(
        address from,
        address to,
        uint256 value,
        uint256 gas,
        uint256 nonce,
        bytes calldata data,
        uint256 validUntil,
        bytes calldata sig
    ) private returns (bool success, bytes memory ret) {
        bytes32 h = keccak256(
            abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR, _hashFields(from, to, value, gas, nonce, data, validUntil))
        );

        if (block.timestamp > validUntil) {
            emit Executed(from, to, nonce, false);
            return (false, bytes("EXPIRED"));
        }
        if (nonces[from] != nonce) {
            emit Executed(from, to, nonce, false);
            return (false, bytes("NONCE"));
        }
        // Non-reverting recovery: a malformed signature here is a per-item
        // failure (signer resolves to address(0), which never equals a
        // real `from`), never a revert that would take the rest of the
        // batch down with it.
        address signer = _recoverCompact(h, sig);
        if (signer != from) {
            emit Executed(from, to, nonce, false);
            return (false, bytes("SIG"));
        }

        nonces[from] = nonce + 1;

        (success, ret) = to.call{value: value, gas: gas}(abi.encodePacked(data, from));
        emit Executed(from, to, nonce, success);
    }

    receive() external payable {}
}
