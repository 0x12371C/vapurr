//! PNS — Purr Name Service. TLD `.hood`.
//! Namehash is ENS-shaped. Root owns node 0; the registry owns namehash("hood").
//! Live names are on Robinhood Chain testnet (46630). Pinset is a cache.

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::pin::Pinset;
use crate::postage::{quote, Asset, Quote, Voucher};
use crate::ZmailError;

pub const TLD: &str = "hood";
pub const TLD_DOT: &str = ".hood";

/// ENS namehash("eth") — proves we hash the same way they do.
pub const ENS_ETH_NODE: &str =
    "0x93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae";

const RESERVED: &[&str] = &[
    "www", "mail", "email", "zzzmail", "zmail", "vapurr", "registry", "resolver", "admin",
    "root", "ens", "eth", "hood", "localhost", "official", "support", "nic", "pns",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HoodName(pub String);

impl HoodName {
    pub fn parse(s: &str) -> Result<Self, ZmailError> {
        let s = s.trim().trim_start_matches('@').to_ascii_lowercase();
        let s = s.strip_suffix(TLD_DOT).unwrap_or(&s);
        if s.len() < 3 || s.len() > 32 {
            return Err(ZmailError::BadName);
        }
        if s.starts_with("0x") || s.contains('.') {
            return Err(ZmailError::BadName);
        }
        if s.starts_with('-') || s.ends_with('-') {
            return Err(ZmailError::BadName);
        }
        if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(ZmailError::BadName);
        }
        if RESERVED.contains(&s) {
            return Err(ZmailError::ReservedName);
        }
        Ok(HoodName(format!("{s}.{TLD}")))
    }

    pub fn label(&self) -> &str {
        self.0.strip_suffix(TLD_DOT).unwrap_or(&self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_at(&self) -> String {
        format!("@{}", self.0)
    }

    pub fn node(&self) -> [u8; 32] {
        namehash(&self.0)
    }

    pub fn node_hex(&self) -> String {
        format!("0x{}", hex::encode(self.node()))
    }
}

pub fn looks_like_hood(s: &str) -> bool {
    let s = s.trim().trim_start_matches('@').to_ascii_lowercase();
    s.ends_with(TLD_DOT) && s.len() > TLD_DOT.len()
}

pub fn keccak(bytes: &[u8]) -> [u8; 32] {
    let h = Keccak256::digest(bytes);
    let mut o = [0u8; 32];
    o.copy_from_slice(&h);
    o
}

/// ENS namehash. `namehash("")` is 32 zero bytes. Labels hashed from the TLD up.
pub fn namehash(name: &str) -> [u8; 32] {
    let mut node = [0u8; 32];
    let name = name.trim().trim_start_matches('.').to_ascii_lowercase();
    if name.is_empty() {
        return node;
    }
    for label in name.split('.').rev() {
        if label.is_empty() {
            continue;
        }
        let lh = keccak(label.as_bytes());
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&node);
        buf[32..].copy_from_slice(&lh);
        node = keccak(&buf);
    }
    node
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoodRecord {
    pub name: String,
    pub node: String,
    pub owner: String,
    pub addr: String,
    pub x25519: String,
    #[serde(default)]
    pub cid: String,
    pub ts: u64,
    #[serde(default)]
    pub reverse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoodReceipt {
    pub ok: bool,
    pub kind: String,
    pub pns: bool,
    pub name: String,
    pub node: String,
    pub owner: String,
    pub cid: String,
    pub postage: Quote,
    pub voucher: Voucher,
    #[serde(default)]
    pub onchain: bool,
    #[serde(default)]
    pub chain_id: u64,
    #[serde(default)]
    pub tx: String,
    #[serde(default)]
    pub tx_url: String,
    #[serde(default)]
    pub registry: String,
    #[serde(default)]
    pub need_gas: bool,
    #[serde(default)]
    pub already: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    names: BTreeMap<String, HoodRecord>,
    #[serde(default)]
    reverse: BTreeMap<String, String>,
}

pub struct HoodRegistry {
    path: PathBuf,
    store: Store,
}

impl HoodRegistry {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ZmailError> {
        let dir = root.as_ref().join("hood");
        fs::create_dir_all(&dir).map_err(|_| ZmailError::Io)?;
        let path = dir.join("registry.json");
        let store = fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Ok(Self { path, store })
    }

    pub fn resolve(&self, name: &HoodName) -> Option<&HoodRecord> {
        self.store.names.get(name.as_str())
    }

    pub fn reverse(&self, addr: &str) -> Option<&str> {
        let a = addr.trim().to_ascii_lowercase();
        self.store.reverse.get(&a).map(|s| s.as_str())
    }

    pub fn primary_of(&self, addr: &str) -> Option<HoodName> {
        self.reverse(addr).and_then(|s| HoodName::parse(s).ok())
    }

    pub fn owned_by(&self, addr: &str) -> Vec<&HoodRecord> {
        let a = addr.trim().to_ascii_lowercase();
        self.store
            .names
            .values()
            .filter(|r| r.owner.eq_ignore_ascii_case(&a))
            .collect()
    }

    pub fn register(
        &mut self,
        name: HoodName,
        owner: &str,
        x25519: &str,
        pinset: &Pinset,
    ) -> Result<HoodReceipt, ZmailError> {
        let owner = owner.trim().to_ascii_lowercase();
        if owner.len() != 42 || !owner.starts_with("0x") {
            return Err(ZmailError::BadAddress);
        }
        if let Some(have) = self.reverse(&owner) {
            return Err(ZmailError::AlreadyNamed(have.to_string()));
        }
        if self.store.names.contains_key(name.as_str()) {
            return Err(ZmailError::NameTaken);
        }
        let postage = quote(Asset::Pusd, 0, 0).map_err(|_| ZmailError::OverCap)?;
        let now = now_secs();
        let mut rec = HoodRecord {
            name: name.as_str().to_string(),
            node: name.node_hex(),
            owner: owner.clone(),
            addr: owner.clone(),
            x25519: x25519.trim().trim_start_matches("0x").to_ascii_lowercase(),
            cid: String::new(),
            ts: now,
            reverse: true,
        };
        let packed = serde_json::to_vec(&rec).map_err(|_| ZmailError::Io)?;
        let (cid, _pin) = pinset.put(&packed)?;
        rec.cid = cid.clone();
        self.store.names.insert(name.as_str().to_string(), rec.clone());
        self.store.reverse.insert(owner.clone(), name.as_str().to_string());
        self.save()?;
        let voucher = Voucher::new(&owner, name.as_at().as_str(), &cid, Asset::Pusd, now, now);
        Ok(HoodReceipt {
            ok: true,
            kind: "hood".into(),
            pns: true,
            name: name.as_str().to_string(),
            node: rec.node,
            owner,
            cid,
            postage,
            voucher,
            onchain: false,
            chain_id: 0,
            tx: String::new(),
            tx_url: String::new(),
            registry: String::new(),
            need_gas: false,
            already: false,
        })
    }

    /// Cache a name after the chain accepted it. First reverse wins.
    pub fn remember(
        &mut self,
        name: HoodName,
        owner: &str,
        x25519: &str,
        pinset: &Pinset,
    ) -> Result<HoodReceipt, ZmailError> {
        let owner = owner.trim().to_ascii_lowercase();
        if owner.len() != 42 || !owner.starts_with("0x") {
            return Err(ZmailError::BadAddress);
        }
        if let Some(have) = self.store.names.get(name.as_str()) {
            if !have.owner.eq_ignore_ascii_case(&owner) {
                return Err(ZmailError::NameTaken);
            }
        }
        let postage = quote(Asset::Pusd, 0, 0).map_err(|_| ZmailError::OverCap)?;
        let now = now_secs();
        let first = !self.store.reverse.contains_key(&owner);
        let mut rec = HoodRecord {
            name: name.as_str().to_string(),
            node: name.node_hex(),
            owner: owner.clone(),
            addr: owner.clone(),
            x25519: x25519.trim().trim_start_matches("0x").to_ascii_lowercase(),
            cid: String::new(),
            ts: now,
            reverse: first,
        };
        let packed = serde_json::to_vec(&rec).map_err(|_| ZmailError::Io)?;
        let (cid, _pin) = pinset.put(&packed)?;
        rec.cid = cid.clone();
        self.store
            .names
            .insert(name.as_str().to_string(), rec.clone());
        self.store
            .reverse
            .entry(owner.clone())
            .or_insert_with(|| name.as_str().to_string());
        rec.reverse = self
            .store
            .reverse
            .get(&owner)
            .map(|s| s == name.as_str())
            .unwrap_or(false);
        self.save()?;
        let voucher = Voucher::new(&owner, name.as_at().as_str(), &cid, Asset::Pusd, now, now);
        Ok(HoodReceipt {
            ok: true,
            kind: "hood".into(),
            pns: true,
            name: name.as_str().to_string(),
            node: rec.node,
            owner,
            cid,
            postage,
            voucher,
            onchain: false,
            chain_id: 0,
            tx: String::new(),
            tx_url: String::new(),
            registry: String::new(),
            need_gas: false,
            already: false,
        })
    }

    pub fn snapshot(&self, addr: &str) -> serde_json::Value {
        let primary = self.reverse(addr).unwrap_or("");
        serde_json::json!({
            "pns": true,
            "service": "PNS",
            "tld": TLD_DOT,
            "primary": primary,
            "names": self.owned_by(addr),
            "reverse": self.store.reverse,
        })
    }

    fn save(&self) -> Result<(), ZmailError> {
        let bytes = serde_json::to_vec_pretty(&self.store).map_err(|_| ZmailError::Io)?;
        fs::write(&self.path, bytes).map_err(|_| ZmailError::Io)
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::OsRng, RngCore};

    fn tmp() -> PathBuf {
        let mut n = [0u8; 8];
        OsRng.fill_bytes(&mut n);
        std::env::temp_dir().join(format!(
            "zzzmail-hood-{}-{}",
            std::process::id(),
            hex::encode(n)
        ))
    }

    #[test]
    fn namehash_matches_ens_eth() {
        assert_eq!(format!("0x{}", hex::encode(namehash(""))), format!("0x{}", "00".repeat(32)));
        assert_eq!(format!("0x{}", hex::encode(namehash("eth"))), ENS_ETH_NODE);
        assert_ne!(namehash("alice.hood"), namehash("bob.hood"));
        assert_eq!(namehash("alice.hood"), namehash("ALICE.HOOD"));
    }

    #[test]
    fn parse_rejects_junk() {
        assert_eq!(HoodName::parse("alice.hood").unwrap().as_str(), "alice.hood");
        assert_eq!(HoodName::parse("@Alice").unwrap().as_str(), "alice.hood");
        assert!(HoodName::parse("ab.hood").is_err());
        assert!(HoodName::parse("alice.eth").is_err());
        assert!(HoodName::parse("pay.alice.hood").is_err());
        assert!(HoodName::parse("vapurr.hood").is_err());
        assert!(HoodName::parse("pns.hood").is_err());
        assert!(HoodName::parse("0xabc.hood").is_err());
        assert!(looks_like_hood("alice.hood"));
        assert!(!looks_like_hood("alice"));
    }

    #[test]
    fn register_resolve_reverse() {
        let dir = tmp();
        let pin = Pinset::open(&dir).unwrap();
        let mut r = HoodRegistry::open(&dir).unwrap();
        let got = r
            .register(
                HoodName::parse("alice.hood").unwrap(),
                "0xc8ae558f58baf209cf371e64b7baa84181a90060",
                "aa".repeat(32).as_str(),
                &pin,
            )
            .unwrap();
        assert!(got.ok);
        assert_eq!(got.kind, "hood");
        assert!(got.postage.gasless);
        assert_eq!(got.postage.usd_micros, 2_500);
        assert!(got.cid.starts_with("bafkrei"));
        let rec = r.resolve(&HoodName::parse("alice").unwrap()).unwrap();
        assert_eq!(rec.addr, "0xc8ae558f58baf209cf371e64b7baa84181a90060");
        assert_eq!(
            r.reverse("0xC8ae558F58BaF209cF371e64b7baa84181A90060")
                .unwrap(),
            "alice.hood"
        );
        let again = r.register(
            HoodName::parse("bob.hood").unwrap(),
            "0xc8ae558f58baf209cf371e64b7baa84181a90060",
            "bb".repeat(32).as_str(),
            &pin,
        );
        assert!(matches!(again, Err(ZmailError::AlreadyNamed(_))));
        let two = r
            .remember(
                HoodName::parse("relic").unwrap(),
                "0xc8ae558f58baf209cf371e64b7baa84181a90060",
                "cc".repeat(32).as_str(),
                &pin,
            )
            .unwrap();
        assert_eq!(two.name, "relic.hood");
        assert_eq!(
            r.reverse("0xc8ae558f58baf209cf371e64b7baa84181a90060")
                .unwrap(),
            "alice.hood"
        );
        assert!(r.resolve(&HoodName::parse("relic").unwrap()).is_some());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn taken_name_fails() {
        let dir = tmp();
        let pin = Pinset::open(&dir).unwrap();
        let mut r = HoodRegistry::open(&dir).unwrap();
        r.register(
            HoodName::parse("taken").unwrap(),
            "0xc8ae558f58baf209cf371e64b7baa84181a90060",
            "aa".repeat(32).as_str(),
            &pin,
        )
        .unwrap();
        let err = r
            .register(
                HoodName::parse("taken.hood").unwrap(),
                "0x0000000000000000000000000000000000000001",
                "bb".repeat(32).as_str(),
                &pin,
            )
            .unwrap_err();
        assert!(matches!(err, ZmailError::NameTaken));
        let _ = fs::remove_dir_all(dir);
    }
}
