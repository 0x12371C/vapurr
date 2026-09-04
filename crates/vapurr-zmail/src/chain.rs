//! PNS on Robinhood Chain testnet (46630). Local pinset is cache, not the source.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use vapurr_rhc::{self as rhc, Rpc};
use vapurr_wallet::tx::{
    decode_abi_string, decode_hex_bytes, decode_word_addr, encode_fn_addr, encode_fn_bytes32,
    encode_fn_bytes32_addr, encode_fn_str, encode_fn_str_bytes32, hex0x, revert_reason, Tx,
};
use vapurr_wallet::{addr_from_hex, Address, DeviceKey};

use crate::hood::HoodName;
use crate::ZmailError;

const PNS_HEX: &str = include_str!("pns.hex");

fn bytecode() -> Result<Vec<u8>, ZmailError> {
    let s = PNS_HEX.trim().trim_start_matches("0x");
    hex::decode(s).map_err(|_| ZmailError::Io)
}

fn cfg_path() -> PathBuf {
    vapurr_wallet::data_dir().join("pns.json")
}

fn saved_registry() -> Option<Address> {
    if !rhc::TESTNET_PNS.is_empty() {
        if let Some(a) = addr_from_hex(rhc::TESTNET_PNS) {
            return Some(a);
        }
    }
    let raw = fs::read_to_string(cfg_path()).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    // Ignore pre-ENS pins (old 0x7eAc… did not own .hood).
    if v.get("ens").and_then(|x| x.as_bool()) != Some(true) {
        return None;
    }
    addr_from_hex(v.get("testnet")?.as_str()?)
}

fn save_registry(addr: &Address, tx: &str) {
    let v = json!({
        "pns": true,
        "ens": true,
        "service": "PNS",
        "tld": ".hood",
        "chain_id": rhc::TESTNET_CHAIN_ID,
        "testnet": addr.to_checksum(),
        "tx": tx,
    });
    let _ = fs::create_dir_all(vapurr_wallet::data_dir());
    let _ = fs::write(cfg_path(), v.to_string());
}

fn rpc() -> Rpc {
    Rpc::at(rhc::TESTNET_RPC_HTTP)
}

fn live_code(rpc: &Rpc, addr: &Address) -> bool {
    let code = rpc.eth_code(&addr.to_hex()).unwrap_or_default();
    let hex = code.trim().trim_start_matches("0x").trim();
    hex.len() > 2
}

fn send(key: &DeviceKey, rpc: &Rpc, to: Option<Address>, data: &[u8]) -> Result<String, ZmailError> {
    let from = key.address.to_hex();
    let eth = rpc.eth_balance(&from).map_err(|_| ZmailError::NeedGas)?;
    if eth < 100_000_000_000_000 {
        return Err(ZmailError::NeedGas);
    }
    let nonce = rpc.eth_nonce(&from).map_err(|_| ZmailError::Rpc)?;
    let gas_price = rpc.eth_gas_price().unwrap_or(100_000_000);
    let to_hex = to.map(|a| a.to_hex());
    let data_hex = hex0x(data);
    let est = match rpc.eth_estimate_gas(&from, to_hex.as_deref(), &data_hex) {
        Ok(g) => g,
        Err(e) => {
            let err = map_rpc(e);
            if matches!(err, ZmailError::Rpc) {
                if to.is_none() { 2_000_000 } else { 250_000 }
            } else {
                return Err(err);
            }
        }
    };
    let gas = est.saturating_mul(13) / 10;
    let tx = Tx {
        chain_id: rhc::TESTNET_CHAIN_ID,
        nonce,
        max_priority_fee: 1_000_000,
        max_fee: gas_price.saturating_mul(3).max(1_000_000),
        gas,
        to,
        value: 0,
        data: data.to_vec(),
    };
    let raw = key.sign_tx(&tx).map_err(|_| ZmailError::Crypto)?;
    let hash = rpc.eth_send_raw(&hex0x(&raw)).map_err(|e| map_rpc(e))?;
    let receipt = wait(rpc, &hash)?;
    let status = receipt
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0");
    if status != "0x1" {
        let why = receipt
            .get("revertReason")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let why = revert_reason(why).unwrap_or_else(|| why.to_string());
        if why.contains("TAKEN") {
            return Err(ZmailError::NameTaken);
        }
        if why.contains("PRIMARY") {
            return Err(ZmailError::AlreadyNamed("primary".into()));
        }
        if why.contains("NAME") {
            return Err(ZmailError::BadName);
        }
        if why.contains("TLD") {
            return Err(ZmailError::Rpc);
        }
        return Err(ZmailError::Rpc);
    }
    Ok(hash)
}

fn wait(rpc: &Rpc, hash: &str) -> Result<Value, ZmailError> {
    for _ in 0..80 {
        match rpc.eth_receipt(hash) {
            Ok(Some(r)) => return Ok(r),
            _ => std::thread::sleep(Duration::from_millis(400)),
        }
    }
    Err(ZmailError::Rpc)
}

fn owner_of(rpc: &Rpc, registry: &Address, node: &[u8; 32]) -> Option<Address> {
    let from = "0x0000000000000000000000000000000000000000";
    let data = encode_fn_bytes32("owner(bytes32)", node);
    let raw = rpc
        .eth_call(from, Some(&registry.to_hex()), &hex0x(&data))
        .ok()?;
    let bytes = decode_hex_bytes(&raw).ok()?;
    decode_word_addr(&bytes, 0)
}

fn ensure_registry(key: &DeviceKey, rpc: &Rpc) -> Result<(Address, Option<String>), ZmailError> {
    if let Some(a) = saved_registry() {
        if live_code(rpc, &a) {
            return Ok((a, None));
        }
        // A baked pin with no code is dead. Do not mint a second registry.
        if !rhc::TESTNET_PNS.is_empty() {
            return Err(ZmailError::Rpc);
        }
    }
    let hash = send(key, rpc, None, &bytecode()?)?;
    let receipt = wait(rpc, &hash)?;
    let ca = receipt
        .get("contractAddress")
        .and_then(|v| v.as_str())
        .ok_or(ZmailError::Rpc)?;
    let addr = addr_from_hex(ca).ok_or(ZmailError::BadAddress)?;
    save_registry(&addr, &hash);
    Ok((addr, Some(hash)))
}

/// Register `label.hood` on testnet. Returns the tx hash.
/// Does not deploy. One registry — Open first.
pub fn register(name: &HoodName, x25519: &[u8; 32]) -> Result<String, ZmailError> {
    let key = DeviceKey::load_or_create();
    let rpc = rpc();
    let registry = saved_registry().ok_or(ZmailError::Rpc)?;
    if !live_code(&rpc, &registry) {
        return Err(ZmailError::Rpc);
    }
    let data = encode_fn_str_bytes32("register(string,bytes32)", name.label(), *x25519);
    send(&key, &rpc, Some(registry), &data)
}

pub fn resolve(name: &str) -> Option<Value> {
    let rpc = rpc();
    let registry = saved_registry()?;
    if !live_code(&rpc, &registry) {
        return None;
    }
    let from = "0x0000000000000000000000000000000000000000";
    let data = encode_fn_str("resolveName(string)", name);
    let raw = rpc
        .eth_call(from, Some(&registry.to_hex()), &hex0x(&data))
        .ok()?;
    let bytes = decode_hex_bytes(&raw).ok()?;
    if bytes.len() < 128 {
        return None;
    }
    let owner = decode_word_addr(&bytes, 0)?;
    if owner.0.iter().all(|&b| b == 0) {
        return None;
    }
    let addr = decode_word_addr(&bytes, 1)?;
    let mut x25519 = [0u8; 32];
    x25519.copy_from_slice(&bytes[64..96]);
    Some(json!({
        "ok": true,
        "pns": true,
        "kind": "hood",
        "service": "PNS",
        "chain_id": rhc::TESTNET_CHAIN_ID,
        "registry": registry.to_checksum(),
        "onchain": true,
        "record": {
            "name": if name.ends_with(".hood") { name.to_ascii_lowercase() } else { format!("{}.hood", name.trim().trim_start_matches('@').to_ascii_lowercase()) },
            "owner": owner.to_hex(),
            "addr": addr.to_hex(),
            "x25519": hex::encode(x25519),
        }
    }))
}

pub fn reverse(addr: &str) -> Option<String> {
    let rpc = rpc();
    let registry = saved_registry()?;
    if !live_code(&rpc, &registry) {
        return None;
    }
    let a = addr_from_hex(addr)?;
    let from = "0x0000000000000000000000000000000000000000";
    let data = encode_fn_addr("reverse(address)", a);
    let raw = rpc
        .eth_call(from, Some(&registry.to_hex()), &hex0x(&data))
        .ok()?;
    let bytes = decode_hex_bytes(&raw).ok()?;
    let s = decode_abi_string(&bytes)?;
    let s = s.trim().to_ascii_lowercase();
    if s.is_empty() || !s.ends_with(".hood") {
        return None;
    }
    Some(s)
}

pub fn registry_hex() -> Option<String> {
    saved_registry().map(|a| a.to_hex())
}

fn map_rpc(e: vapurr_rhc::rpc::RpcError) -> ZmailError {
    let s = e.to_string();
    if let Some(why) = revert_reason(&s) {
        if why.contains("TAKEN") {
            return ZmailError::NameTaken;
        }
        if why.contains("PRIMARY") {
            return ZmailError::AlreadyNamed("primary".into());
        }
        if why.contains("NAME") {
            return ZmailError::BadName;
        }
        if why.contains("OWNER") {
            return ZmailError::NeedName;
        }
        if why.contains("TLD") {
            return ZmailError::Rpc;
        }
    }
    if s.contains("insufficient funds") {
        return ZmailError::NeedGas;
    }
    ZmailError::Rpc
}

pub fn set_addr(name: &str, addr: &str) -> Result<String, ZmailError> {
    let name = HoodName::parse(name)?;
    let dest = addr_from_hex(addr).ok_or(ZmailError::BadAddress)?;
    let key = DeviceKey::load_or_create();
    let rpc = rpc();
    let registry = saved_registry().ok_or(ZmailError::Rpc)?;
    let node = name.node();
    let data = encode_fn_bytes32_addr("setAddr(bytes32,address)", &node, dest);
    send(&key, &rpc, Some(registry), &data)
}

pub fn set_name(name: &str) -> Result<String, ZmailError> {
    let name = HoodName::parse(name)?;
    let key = DeviceKey::load_or_create();
    let rpc = rpc();
    let registry = saved_registry().ok_or(ZmailError::Rpc)?;
    let data = encode_fn_str("setName(string)", name.as_str());
    send(&key, &rpc, Some(registry), &data)
}

pub fn status(addr: &str) -> Value {
    let rpc = rpc();
    let registry = saved_registry();
    let live = registry.as_ref().map(|a| live_code(&rpc, a)).unwrap_or(false);
    let primary = if live {
        reverse(addr).unwrap_or_default()
    } else {
        String::new()
    };
    let label = primary
        .strip_suffix(".hood")
        .unwrap_or(primary.as_str())
        .to_string();
    let eth = if addr.len() == 42 {
        rpc.eth_balance(addr).unwrap_or(0)
    } else {
        0
    };
    let (root, tld_owner, tld_locked) = if live {
        let reg = registry.as_ref().unwrap();
        let root = owner_of(&rpc, reg, &[0u8; 32]);
        let tld = owner_of(&rpc, reg, &crate::hood::namehash("hood"));
        let locked = match (&tld, registry) {
            (Some(t), Some(r)) => t.0 == r.0,
            _ => false,
        };
        (
            root.map(|a| a.to_checksum()).unwrap_or_default(),
            tld.map(|a| a.to_checksum()).unwrap_or_default(),
            locked,
        )
    } else {
        (String::new(), String::new(), false)
    };
    json!({
        "live": live,
        "need_deploy": !live,
        "need_eth": eth < 100_000_000_000_000,
        "registry": registry.map(|a| a.to_checksum()).unwrap_or_default(),
        "root": root,
        "tld_owner": tld_owner,
        "tld_locked": tld_locked,
        "primary": primary,
        "label": label,
        "tld": ".hood",
        "chain_id": rhc::TESTNET_CHAIN_ID,
        "explorer": rhc::TESTNET_EXPLORER,
        "faucet": rhc::TESTNET_FAUCET,
        "pns": true,
        "ens": true,
        "service": "PNS",
    })
}

const STATUS_TTL: Duration = Duration::from_secs(12);
static STATUS: Mutex<Option<(Instant, String, Value)>> = Mutex::new(None);
static STATUS_LOOP: AtomicBool = AtomicBool::new(false);
static STATUS_ADDR: Mutex<String> = Mutex::new(String::new());

fn idle_status(addr: &str) -> Value {
    json!({
        "ok": true,
        "live": false,
        "loading": true,
        "need_deploy": false,
        "need_eth": false,
        "registry": saved_registry().map(|a| a.to_checksum()).unwrap_or_default(),
        "root": "",
        "tld_owner": "",
        "tld_locked": false,
        "primary": "",
        "label": "",
        "tld": ".hood",
        "chain_id": rhc::TESTNET_CHAIN_ID,
        "explorer": rhc::TESTNET_EXPLORER,
        "faucet": rhc::TESTNET_FAUCET,
        "pns": true,
        "ens": true,
        "service": "PNS",
        "address": addr,
    })
}

pub fn kick_status(addr: &str) {
    if let Ok(mut g) = STATUS_ADDR.lock() {
        *g = addr.to_string();
    }
    if STATUS_LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("pns-status".into())
        .spawn(|| loop {
            let addr = STATUS_ADDR
                .lock()
                .ok()
                .map(|g| g.clone())
                .unwrap_or_default();
            let v = status(&addr);
            if let Ok(mut g) = STATUS.lock() {
                *g = Some((Instant::now(), addr, v));
            }
            std::thread::sleep(STATUS_TTL);
        });
    if spawned.is_err() {
        STATUS_LOOP.store(false, Ordering::SeqCst);
    }
}

/// Cached chain status. Never RPC. Safe on the WebView protocol thread.
pub fn status_snapshot(addr: &str) -> Value {
    kick_status(addr);
    if let Ok(g) = STATUS.lock() {
        if let Some((_, a, v)) = g.as_ref() {
            if addr.is_empty() || a.eq_ignore_ascii_case(addr) {
                return v.clone();
            }
        }
    }
    idle_status(addr)
}

pub fn deploy() -> Result<Value, ZmailError> {
    let key = DeviceKey::load_or_create();
    let rpc = rpc();
    let (_addr, minted) = ensure_registry(&key, &rpc)?;
    let mut v = status(&key.address.to_hex());
    v["ok"] = json!(true);
    if let Some(hash) = minted {
        v["tx"] = json!(hash);
        v["tx_url"] = json!(format!("{}/tx/{}", rhc::TESTNET_EXPLORER, hash));
    } else {
        v["already"] = json!(true);
    }
    Ok(v)
}

fn call_reverts(rpc: &Rpc, from: &str, to: &Address, data: &[u8]) -> bool {
    rpc.eth_call(from, Some(&to.to_hex()), &hex0x(data))
        .is_err()
}

/// Live checks: old CA does not own .hood; new registry does; steal reverts.
pub fn prove_tld() -> Result<Value, ZmailError> {
    let key = DeviceKey::load_or_create();
    let rpc = rpc();
    let from = key.address.to_hex();
    let eth = rpc.eth_balance(&from).unwrap_or(0);
    let hood = crate::hood::namehash("hood");
    let zero = [0u8; 32];
    let old = addr_from_hex("0x7eAc2c587Dbb60B2a7f357cfCB28c37c74A6E7d6").ok_or(ZmailError::BadAddress)?;
    let old_root = owner_of(&rpc, &old, &zero).map(|a| a.to_checksum()).unwrap_or_default();
    let old_tld = owner_of(&rpc, &old, &hood).map(|a| a.to_checksum()).unwrap_or_default();
    let old_unlocked = old_tld.is_empty()
        || old_tld == "0x0000000000000000000000000000000000000000";

    if eth < 100_000_000_000_000 {
        return Ok(json!({
            "ok": false,
            "need_eth": true,
            "deployer": key.address.to_checksum(),
            "eth": eth,
            "old_registry": old.to_checksum(),
            "old_root": old_root,
            "old_tld_owner": old_tld,
            "old_tld_unlocked": old_unlocked,
        }));
    }

    let deployed = deploy()?;
    let registry = saved_registry().ok_or(ZmailError::Rpc)?;
    let root = owner_of(&rpc, &registry, &zero).ok_or(ZmailError::Rpc)?;
    let tld = owner_of(&rpc, &registry, &hood).ok_or(ZmailError::Rpc)?;
    let steal_owner = encode_fn_bytes32_addr("setOwner(bytes32,address)", &hood, key.address);
    let mut steal_sub = vapurr_wallet::keccak4("setSubnodeOwner(bytes32,bytes32,address)").to_vec();
    steal_sub.extend_from_slice(&hood);
    steal_sub.extend_from_slice(&crate::hood::keccak(b"stolen"));
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(&key.address.0);
    steal_sub.extend_from_slice(&word);
    let attacker = "0x0000000000000000000000000000000000000001";
    let root_ok = root.0 == key.address.0;
    let tld_ok = tld.0 == registry.0;
    let steal_owner_reverts = call_reverts(&rpc, &from, &registry, &steal_owner)
        && call_reverts(&rpc, attacker, &registry, &steal_owner);
    let steal_sub_reverts = call_reverts(&rpc, &from, &registry, &steal_sub)
        && call_reverts(&rpc, attacker, &registry, &steal_sub);

    Ok(json!({
        "ok": root_ok && tld_ok && steal_owner_reverts && steal_sub_reverts && old_unlocked,
        "deployer": key.address.to_checksum(),
        "registry": registry.to_checksum(),
        "tx": deployed.get("tx").cloned().unwrap_or(Value::Null),
        "tx_url": deployed.get("tx_url").cloned().unwrap_or(Value::Null),
        "root": root.to_checksum(),
        "tld_owner": tld.to_checksum(),
        "root_is_deployer": root_ok,
        "tld_is_registry": tld_ok,
        "steal_setOwner_reverts": steal_owner_reverts,
        "steal_setSubnodeOwner_reverts": steal_sub_reverts,
        "old_registry": old.to_checksum(),
        "old_root": old_root,
        "old_tld_owner": old_tld,
        "old_tld_unlocked": old_unlocked,
        "explorer": rhc::TESTNET_EXPLORER,
        "chain_id": rhc::TESTNET_CHAIN_ID,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hood::namehash;

    #[test]
    fn hood_node_matches_ens_style() {
        let hood = namehash("hood");
        let mut packed = [0u8; 64];
        packed[..32].copy_from_slice(&hood);
        packed[32..].copy_from_slice(&crate::hood::keccak(b"alice"));
        assert_eq!(crate::hood::keccak(&packed), namehash("alice.hood"));
    }

    #[test]
    fn bytecode_is_present() {
        let b = bytecode().unwrap();
        assert!(b.len() > 1000, "{}", b.len());
        assert!(b[0] == 0x60 || b[0] == 0x61, "got {:#x}", b[0]);
    }

    #[test]
    fn status_does_not_block_protocol_thread() {
        use std::time::Instant;
        let t0 = Instant::now();
        let v = status_snapshot("0x0000000000000000000000000000000000000001");
        let ms = t0.elapsed().as_millis();
        eprintln!("chain::status_snapshot {ms}ms loading={:?}", v.get("loading"));
        assert!(
            ms < 250,
            "status_snapshot took {ms}ms; explorer loadPns → /zzzmail/api/pns freezes every WebView. body={}",
            v.to_string().chars().take(160).collect::<String>()
        );
    }

    #[test]
    fn ens_selectors() {
        assert_eq!(vapurr_wallet::keccak4("addr(bytes32)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("setAddr(bytes32,address)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("owner(bytes32)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("resolver(bytes32)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("setName(string)").len(), 4);
        assert_eq!(
            vapurr_wallet::keccak4("setSubnodeOwner(bytes32,bytes32,address)").len(),
            4
        );
    }

    #[test]
    fn hood_node_is_ens_namehash() {
        assert_eq!(namehash("hood"), crate::hood::namehash("hood"));
        assert_ne!(namehash("hood"), [0u8; 32]);
        assert_ne!(namehash("alice.hood"), namehash("hood"));
    }

    #[test]
    #[ignore]
    fn live_prove_hood_tld() {
        let v = super::prove_tld().expect("prove");
        eprintln!("{}", serde_json::to_string_pretty(&v).unwrap());
        assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(true), "{v}");
        assert_eq!(v.get("root_is_deployer").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("tld_is_registry").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("steal_setOwner_reverts").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("steal_setSubnodeOwner_reverts").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("old_tld_unlocked").and_then(|x| x.as_bool()), Some(true));
    }

    #[test]
    #[ignore]
    fn live_deploy_registry() {
        match super::deploy() {
            Ok(v) => {
                eprintln!("{}", v);
                assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(true));
                assert_eq!(v.get("live").and_then(|x| x.as_bool()), Some(true));
            }
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    #[ignore]
    fn live_register_resolve_reverse() {
        let key = DeviceKey::load_or_create();
        let from = key.address.to_hex();
        let rpc = rpc();
        let eth = rpc.eth_balance(&from).expect("balance");
        eprintln!("addr {from} eth {eth}");
        assert!(eth > 0, "need testnet ETH");
        let st = status(&from);
        eprintln!("status {st}");
        assert_eq!(st.get("live").and_then(|x| x.as_bool()), Some(true));
        let name = if let Some(p) = st.get("primary").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
            p.to_string()
        } else {
            let n = HoodName::parse("onchainprobe").unwrap();
            let pk = [0x11u8; 32];
            let tx = super::register(&n, &pk).expect("register");
            eprintln!("register tx {tx}");
            assert!(tx.starts_with("0x") && tx.len() == 66, "{tx}");
            n.as_str().to_string()
        };
        let resolved = super::resolve(&name).expect("resolve");
        eprintln!("resolve {resolved}");
        let addr = resolved
            .pointer("/record/addr")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        assert!(addr.eq_ignore_ascii_case(&from), "addr {addr} from {from}");
        let back = super::reverse(&from).expect("reverse");
        eprintln!("reverse {back}");
        assert_eq!(back, name);
    }
}
