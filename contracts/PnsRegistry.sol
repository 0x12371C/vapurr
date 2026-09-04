// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// PNS — Purr Name Service. TLD .hood, the same way ENS holds .eth.
///
/// Root node (0x00) is owned by the deployer. Constructor assigns
/// namehash("hood") to this contract as registrar. Public register()
/// mints 2LDs (alice.hood). Users own the label. Root can move the TLD
/// with setSubnodeOwner. A clone of this bytecode is not PNS — vapurr
/// pins one registry address.
contract PnsRegistry {
    bytes32 public constant ROOT_NODE = bytes32(0);
    bytes32 public constant HOOD_NODE =
        keccak256(abi.encodePacked(bytes32(0), keccak256("hood")));

    struct Rec {
        address owner;
        address addr;
        bytes32 x25519;
        uint64 ts;
    }

    mapping(bytes32 => Rec) public recs;
    mapping(bytes32 => string) public names;
    mapping(address => bytes32) public primary;

    event NewOwner(bytes32 indexed node, bytes32 indexed label, address owner);
    event Registered(bytes32 indexed node, string name, address indexed owner, bytes32 x25519);
    event Transfer(bytes32 indexed node, address owner);
    event AddrChanged(bytes32 indexed node, address a);
    event NameChanged(address indexed who, string name);

    constructor() {
        recs[ROOT_NODE].owner = msg.sender;
        recs[HOOD_NODE].owner = address(this);
        names[HOOD_NODE] = "hood";
        emit Transfer(ROOT_NODE, msg.sender);
        emit NewOwner(ROOT_NODE, keccak256("hood"), address(this));
        emit Transfer(HOOD_NODE, address(this));
    }

    function nodeOf(string calldata label) public pure returns (bytes32) {
        return _node(bytes(label));
    }

    function _node(bytes memory label) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(HOOD_NODE, keccak256(label)));
    }

    function owner(bytes32 node) external view returns (address) {
        return recs[node].owner;
    }

    function resolver(bytes32 node) external view returns (address) {
        return recs[node].owner == address(0) ? address(0) : address(this);
    }

    function ttl(bytes32) external pure returns (uint64) {
        return 0;
    }

    function addr(bytes32 node) external view returns (address) {
        return recs[node].addr;
    }

    function name(bytes32 node) external view returns (string memory) {
        return names[node];
    }

    function recordExists(bytes32 node) external view returns (bool) {
        return recs[node].owner != address(0);
    }

    function available(string calldata label) external view returns (bool) {
        if (!_okLabel(label)) return false;
        return recs[nodeOf(label)].owner == address(0);
    }

    /// ENS Registry. Only the owner of `node` can assign `label` under it.
    /// Root moves .hood with setSubnodeOwner(ROOT_NODE, keccak256("hood"), to).
    function setSubnodeOwner(bytes32 node, bytes32 label, address owner_)
        external
        returns (bytes32)
    {
        require(recs[node].owner == msg.sender, "OWNER");
        require(owner_ != address(0), "ADDR");
        bytes32 sub = keccak256(abi.encodePacked(node, label));
        recs[sub].owner = owner_;
        emit NewOwner(node, label, owner_);
        emit Transfer(sub, owner_);
        return sub;
    }

    /// Public controller. Only live while this contract still owns .hood.
    function register(string calldata label, bytes32 x25519) external {
        require(_okLabel(label), "NAME");
        require(recs[HOOD_NODE].owner == address(this), "TLD");
        bytes32 node = nodeOf(label);
        require(recs[node].owner == address(0), "TAKEN");
        recs[node] = Rec(msg.sender, msg.sender, x25519, uint64(block.timestamp));
        string memory full = string(abi.encodePacked(label, ".hood"));
        names[node] = full;
        if (primary[msg.sender] == bytes32(0)) {
            primary[msg.sender] = node;
            emit NameChanged(msg.sender, full);
        }
        emit NewOwner(HOOD_NODE, keccak256(bytes(label)), msg.sender);
        emit Registered(node, full, msg.sender, x25519);
        emit Transfer(node, msg.sender);
        emit AddrChanged(node, msg.sender);
    }

    function setAddr(bytes32 node, address a) external {
        require(recs[node].owner == msg.sender, "OWNER");
        require(a != address(0), "ADDR");
        recs[node].addr = a;
        emit AddrChanged(node, a);
    }

    function setOwner(bytes32 node, address o) external {
        require(recs[node].owner == msg.sender, "OWNER");
        require(o != address(0), "ADDR");
        require(node != HOOD_NODE, "TLD");
        if (primary[msg.sender] == node) {
            primary[msg.sender] = bytes32(0);
        }
        recs[node].owner = o;
        emit Transfer(node, o);
    }

    function setName(string calldata nam) external {
        bytes32 node = _node(bytes(_label(nam)));
        require(recs[node].owner == msg.sender, "OWNER");
        primary[msg.sender] = node;
        emit NameChanged(msg.sender, names[node]);
    }

    function setX25519(bytes32 node, bytes32 pk) external {
        require(recs[node].owner == msg.sender, "OWNER");
        recs[node].x25519 = pk;
    }

    function resolveName(string calldata nam)
        external
        view
        returns (address owner_, address addr_, bytes32 x25519, uint64 ts)
    {
        Rec memory r = recs[_node(bytes(_label(nam)))];
        return (r.owner, r.addr, r.x25519, r.ts);
    }

    function reverse(address a) external view returns (string memory) {
        return names[primary[a]];
    }

    function _label(string calldata nam) internal pure returns (string memory) {
        bytes memory b = bytes(nam);
        uint256 n = b.length;
        if (n > 5 && b[n - 5] == "." && b[n - 4] == "h" && b[n - 3] == "o" && b[n - 2] == "o" && b[n - 1] == "d") {
            bytes memory lab = new bytes(n - 5);
            for (uint256 i; i < lab.length; i++) lab[i] = b[i];
            return string(lab);
        }
        return nam;
    }

    function _okLabel(string calldata s) internal pure returns (bool) {
        bytes memory b = bytes(s);
        uint256 n = b.length;
        if (n < 3 || n > 32) return false;
        if (b[0] == "-" || b[n - 1] == "-") return false;
        if (n >= 2 && b[0] == "0" && b[1] == "x") return false;
        for (uint256 i; i < n; i++) {
            bytes1 c = b[i];
            bool ok = (c >= "a" && c <= "z") || (c >= "0" && c <= "9") || c == "-";
            if (!ok) return false;
        }
        return !_reserved(s);
    }

    function _reserved(string calldata s) internal pure returns (bool) {
        return _eq(s, "www") || _eq(s, "mail") || _eq(s, "email") || _eq(s, "zzzmail")
            || _eq(s, "zmail") || _eq(s, "vapurr") || _eq(s, "registry") || _eq(s, "resolver")
            || _eq(s, "admin") || _eq(s, "root") || _eq(s, "ens") || _eq(s, "eth")
            || _eq(s, "hood") || _eq(s, "localhost") || _eq(s, "official") || _eq(s, "support")
            || _eq(s, "nic") || _eq(s, "pns");
    }

    function _eq(string calldata a, string memory b) internal pure returns (bool) {
        return keccak256(bytes(a)) == keccak256(bytes(b));
    }
}
