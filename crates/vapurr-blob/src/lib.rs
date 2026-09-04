//! Encrypted, content-addressed blobs on this device.
//! CID is sha256 of ciphertext. Chain lease / IPFS pin are later; this is the disk.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Soft cap for the unpaid local envelope (~$0.50/mo product, no chain yet).
pub const QUOTA_BYTES: u64 = 64 * 1024 * 1024;

const KEY_INFO: &[u8] = b"vapurr-blob-v1";
const AAD: &[u8] = b"vapurr-blob-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cid(pub String);

impl Cid {
    fn of_ciphertext(ct: &[u8]) -> Self {
        let h = Sha256::digest(ct);
        Self(format!("b{}", hex::encode(h)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub kind: String,
    pub cid: Cid,
    pub bytes: u64,
    pub ts: u64,
    #[serde(default)]
    plain_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    #[serde(default = "default_quota")]
    quota: u64,
    #[serde(default)]
    entries: Vec<Entry>,
}

fn default_quota() -> u64 {
    QUOTA_BYTES
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            quota: QUOTA_BYTES,
            entries: Vec::new(),
        }
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct DeviceKey([u8; 32]);

pub struct Vault {
    root: PathBuf,
    key: DeviceKey,
    manifest: Manifest,
    /// Ciphertext preload. Not plaintext.
    hot: HashMap<String, Vec<u8>>,
}

impl Vault {
    pub fn default_root() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData/Local")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vapurr")
            .join("blobs")
    }

    pub fn open_default() -> Result<Self, BlobError> {
        Self::open(Self::default_root())
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, BlobError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|_| BlobError::Io)?;
        let key = load_or_create_key(&root.join("device.key"))?;
        let manifest = load_manifest(&root.join("manifest.json"));
        let mut v = Self {
            root,
            key,
            manifest,
            hot: HashMap::new(),
        };
        v.preload();
        Ok(v)
    }

    /// Fetch every stored ciphertext into RAM. Local stand-in for IPFS preload.
    pub fn preload(&mut self) {
        self.hot.clear();
        let names: Vec<String> = self.manifest.entries.iter().map(|e| e.cid.0.clone()).collect();
        for cid in names {
            if let Ok(raw) = fs::read(self.blob_path(&cid)) {
                self.hot.insert(cid, raw);
            }
        }
    }

    pub fn put_named(&mut self, name: &str, kind: &str, plaintext: &[u8]) -> Result<Cid, BlobError> {
        let plain_sha = hex::encode(Sha256::digest(plaintext));
        if let Some(e) = self.manifest.entries.iter().find(|e| e.name == name) {
            if e.plain_sha == plain_sha {
                return Ok(e.cid.clone());
            }
        }
        let (cid, packed) = seal(&self.key, plaintext)?;
        let new_bytes = packed.len() as u64;
        let old = self
            .manifest
            .entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| (e.cid.clone(), e.bytes));
        let used = self.used().saturating_sub(old.as_ref().map(|o| o.1).unwrap_or(0));
        if used.saturating_add(new_bytes) > self.manifest.quota {
            return Err(BlobError::Quota);
        }
        fs::write(self.blob_path(&cid.0), &packed).map_err(|_| BlobError::Io)?;
        self.hot.insert(cid.0.clone(), packed);
        self.manifest.entries.retain(|e| e.name != name);
        if let Some((old_cid, _)) = old {
            if old_cid != cid && !self.manifest.entries.iter().any(|e| e.cid == old_cid) {
                self.hot.remove(&old_cid.0);
                let _ = fs::remove_file(self.blob_path(&old_cid.0));
            }
        }
        self.manifest.entries.push(Entry {
            name: name.to_string(),
            kind: kind.to_string(),
            cid: cid.clone(),
            bytes: new_bytes,
            ts: now_secs(),
            plain_sha,
        });
        self.save_manifest()?;
        Ok(cid)
    }

    pub fn get(&self, cid: &Cid) -> Result<Vec<u8>, BlobError> {
        let packed = if let Some(p) = self.hot.get(&cid.0) {
            p.clone()
        } else {
            fs::read(self.blob_path(&cid.0)).map_err(|_| BlobError::Missing)?
        };
        open(&self.key, &packed)
    }

    pub fn get_named(&self, name: &str) -> Result<Vec<u8>, BlobError> {
        let cid = self
            .manifest
            .entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.cid.clone())
            .ok_or(BlobError::Missing)?;
        self.get(&cid)
    }

    pub fn used(&self) -> u64 {
        self.manifest.entries.iter().map(|e| e.bytes).sum()
    }

    pub fn quota(&self) -> u64 {
        self.manifest.quota
    }

    pub fn entries(&self) -> &[Entry] {
        &self.manifest.entries
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "quota": self.quota(),
            "used": self.used(),
            "preloaded": self.hot.len(),
            "chain": "off",
            "note": "local encrypted CID store. 4663 lease later.",
            "blobs": self.manifest.entries.iter().map(|e| serde_json::json!({
                "name": e.name,
                "kind": e.kind,
                "cid": e.cid.0,
                "bytes": e.bytes,
                "ts": e.ts,
            })).collect::<Vec<_>>(),
        })
    }

    fn blob_path(&self, cid: &str) -> PathBuf {
        self.root.join(format!("{cid}.bin"))
    }

    fn save_manifest(&self) -> Result<(), BlobError> {
        let bytes = serde_json::to_vec_pretty(&self.manifest).map_err(|_| BlobError::Io)?;
        fs::write(self.root.join("manifest.json"), bytes).map_err(|_| BlobError::Io)
    }
}

fn load_or_create_key(path: &Path) -> Result<DeviceKey, BlobError> {
    if let Ok(bytes) = fs::read(path) {
        if bytes.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes);
            return Ok(DeviceKey(k));
        }
    }
    let mut k = [0u8; 32];
    OsRng.fill_bytes(&mut k);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|_| BlobError::Io)?;
    }
    fs::write(path, k).map_err(|_| BlobError::Io)?;
    Ok(DeviceKey(k))
}

fn load_manifest(path: &Path) -> Manifest {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn blob_key(device: &DeviceKey) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(KEY_INFO), &device.0);
    let mut okm = [0u8; 32];
    hk.expand(KEY_INFO, &mut okm).expect("hkdf");
    okm
}

fn seal(device: &DeviceKey, plaintext: &[u8]) -> Result<(Cid, Vec<u8>), BlobError> {
    let kb = blob_key(device);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&kb));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, Payload { msg: plaintext, aad: AAD })
        .map_err(|_| BlobError::Crypto)?;
    let cid = Cid::of_ciphertext(&ct);
    let mut packed = Vec::with_capacity(12 + ct.len());
    packed.extend_from_slice(&nonce_bytes);
    packed.extend_from_slice(&ct);
    Ok((cid, packed))
}

fn open(device: &DeviceKey, packed: &[u8]) -> Result<Vec<u8>, BlobError> {
    if packed.len() < 13 {
        return Err(BlobError::Crypto);
    }
    let kb = blob_key(device);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&kb));
    let nonce = Nonce::from_slice(&packed[..12]);
    cipher
        .decrypt(nonce, Payload { msg: &packed[12..], aad: AAD })
        .map_err(|_| BlobError::Crypto)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("io")]
    Io,
    #[error("crypto")]
    Crypto,
    #[error("missing")]
    Missing,
    #[error("quota")]
    Quota,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let mut n = [0u8; 8];
        OsRng.fill_bytes(&mut n);
        let p = std::env::temp_dir().join(format!(
            "vapurr-blob-{}-{}",
            std::process::id(),
            hex::encode(n)
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn roundtrip_and_preload() {
        let dir = tmp();
        let mut v = Vault::open(&dir).unwrap();
        let cid = v.put_named("desk", "memory", b"purr").unwrap();
        assert!(cid.0.starts_with('b'));
        assert_eq!(v.get(&cid).unwrap(), b"purr");
        drop(v);
        let mut v = Vault::open(&dir).unwrap();
        v.preload();
        assert_eq!(v.get_named("desk").unwrap(), b"purr");
        assert_eq!(v.hot.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn same_plain_skips_rewrite() {
        let dir = tmp();
        let mut v = Vault::open(&dir).unwrap();
        let a = v.put_named("desk", "memory", b"same").unwrap();
        let b = v.put_named("desk", "memory", b"same").unwrap();
        assert_eq!(a, b);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn quota_rejects() {
        let dir = tmp();
        let mut v = Vault::open(&dir).unwrap();
        v.manifest.quota = 40;
        assert!(v.put_named("x", "memory", &[7u8; 80]).is_err());
        let _ = fs::remove_dir_all(dir);
    }
}
