//! zzzmail is a protocol in the browser, not a website.
//!
//! Delivery: seal → IPFS-shaped CID of ciphertext → $PUSD/$VAPURR voucher.
//! Sender pays 0 ETH. Body never goes on-chain.

mod card;
pub mod chain;
mod cid;
mod hood;
mod office;
mod pin;
mod postage;

pub use card::Mailcard;
pub use cid::{cid_raw_sha256, looks_like_cid, EMPTY_RAW};
pub use hood::{
    looks_like_hood, namehash, HoodName, HoodReceipt, HoodRecord, ENS_ETH_NODE, TLD, TLD_DOT,
};
pub use office::{InboxItem, PostOffice, Receipt};
pub use pin::{PinKind, Pinset};
pub use postage::{quote, Asset, Quote, Voucher, MAX_USD_MICROS, POSTAGE_USD_MICROS, TOKEN_DECIMALS};

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Handle(pub String);

impl Handle {
    pub fn parse(s: &str) -> Result<Self, ZmailError> {
        let s = s.trim().trim_start_matches('@').to_ascii_lowercase();
        if s.starts_with("0x") {
            return Err(ZmailError::BadHandle);
        }
        if hood::looks_like_hood(&s) {
            return Err(ZmailError::BadHandle);
        }
        if s.len() < 3 || s.len() > 32 {
            return Err(ZmailError::BadHandle);
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            return Err(ZmailError::BadHandle);
        }
        Ok(Handle(s))
    }

    pub fn as_at(&self) -> String {
        format!("@{}", self.0)
    }
}

/// Who a letter is for: a `.hood` name, a handle, or a 0x address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Recipient {
    Hood(HoodName),
    Handle(Handle),
    Address(String),
}

impl Recipient {
    pub fn parse(s: &str) -> Result<Self, ZmailError> {
        let raw = s.trim().trim_start_matches('@');
        if raw.to_ascii_lowercase().starts_with("0x") {
            return parse_addr(raw).map(Recipient::Address);
        }
        if hood::looks_like_hood(raw) {
            return HoodName::parse(raw).map(Recipient::Hood);
        }
        Handle::parse(raw).map(Recipient::Handle)
    }

    pub fn as_at(&self) -> String {
        match self {
            Recipient::Hood(n) => n.as_at(),
            Recipient::Handle(h) => h.as_at(),
            Recipient::Address(a) => format!("@{a}"),
        }
    }

    pub fn key(&self) -> &str {
        match self {
            Recipient::Hood(n) => n.as_str(),
            Recipient::Handle(h) => h.0.as_str(),
            Recipient::Address(a) => a.as_str(),
        }
    }
}

fn parse_addr(s: &str) -> Result<String, ZmailError> {
    let s = s.trim();
    if s.len() != 42 || !s.starts_with("0x") && !s.starts_with("0X") {
        return Err(ZmailError::BadAddress);
    }
    let hex = &s[2..];
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ZmailError::BadAddress);
    }
    Ok(format!("0x{}", hex.to_ascii_lowercase()))
}

#[derive(Clone)]
pub struct Identity {
    pub handle: Handle,
    pub hood: Option<HoodName>,
    secret: StaticSecret,
    pub public: PublicKey,
}

impl Identity {
    pub fn generate(handle: Handle) -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self {
            handle,
            hood: None,
            secret,
            public,
        }
    }

    pub fn mail_name(&self) -> String {
        self.hood
            .as_ref()
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|| self.handle.0.clone())
    }

    pub fn as_at(&self) -> String {
        format!("@{}", self.mail_name())
    }

    pub fn load_or_create(path: &Path) -> Result<Self, ZmailError> {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(s) = serde_json::from_slice::<StoredId>(&bytes) {
                if let Ok(id) = Identity::from_stored(&s) {
                    return Ok(id);
                }
            }
        }
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        let handle = handle_from_pk(&public);
        let id = Identity {
            handle,
            hood: None,
            secret,
            public,
        };
        id.save(path)?;
        Ok(id)
    }

    pub fn save(&self, path: &Path) -> Result<(), ZmailError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|_| ZmailError::Io)?;
        }
        let stored = StoredId {
            handle: self.handle.0.clone(),
            secret: hex::encode(self.secret.to_bytes()),
            hood: self
                .hood
                .as_ref()
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
        };
        let bytes = serde_json::to_vec_pretty(&stored).map_err(|_| ZmailError::Io)?;
        std::fs::write(path, bytes).map_err(|_| ZmailError::Io)
    }

    fn from_stored(s: &StoredId) -> Result<Self, ZmailError> {
        let handle = Handle::parse(&s.handle)?;
        let raw = hex::decode(s.secret.trim()).map_err(|_| ZmailError::Crypto)?;
        if raw.len() != 32 {
            return Err(ZmailError::Crypto);
        }
        let mut sk = [0u8; 32];
        sk.copy_from_slice(&raw);
        let secret = StaticSecret::from(sk);
        let public = PublicKey::from(&secret);
        let hood = if s.hood.trim().is_empty() {
            None
        } else {
            Some(HoodName::parse(&s.hood)?)
        };
        Ok(Self {
            handle,
            hood,
            secret,
            public,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct StoredId {
    handle: String,
    secret: String,
    #[serde(default)]
    hood: String,
}

fn handle_from_pk(pk: &PublicKey) -> Handle {
    let h = Sha256::digest(pk.as_bytes());
    Handle(format!("z{}", hex::encode(&h[..4])))
}

impl Drop for Identity {
    fn drop(&mut self) {
        let mut b = self.secret.to_bytes();
        b.zeroize();
    }
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("handle", &self.as_at())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub sent_at: DateTime<Utc>,
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub ephemeral_pubkey: [u8; 32],
    #[serde(default)]
    pub from_pk: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opened {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub sent_at: DateTime<Utc>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainPointer {
    pub chain_id: u64,
    pub commitment: String,
}

impl OnchainPointer {
    pub fn rhc(commitment: impl Into<String>) -> Self {
        Self {
            chain_id: 4663,
            commitment: commitment.into(),
        }
    }
}

pub fn seal(
    sender: &Identity,
    to: &Recipient,
    to_pk: &PublicKey,
    subject: &str,
    body: &str,
) -> Envelope {
    let eph = StaticSecret::random_from_rng(OsRng);
    let eph_pub = PublicKey::from(&eph);
    let shared = eph.diffie_hellman(to_pk);
    let from_key = sender.mail_name();
    let key_bytes = derive_key(
        shared.as_bytes(),
        from_key.as_bytes(),
        to.key().as_bytes(),
    );
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext =
        serde_json::to_vec(&serde_json::json!({ "subject": subject, "body": body })).unwrap();
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).expect("encrypt");
    Envelope {
        from: sender.as_at(),
        to: to.as_at(),
        subject: String::new(),
        sent_at: Utc::now(),
        ciphertext,
        nonce: nonce_bytes,
        ephemeral_pubkey: *eph_pub.as_bytes(),
        from_pk: *sender.public.as_bytes(),
    }
}

#[derive(Serialize, Deserialize)]
struct PackedLetter {
    v: u8,
    from: String,
    to: String,
    sent_at: DateTime<Utc>,
    nonce: String,
    eph: String,
    #[serde(default)]
    from_pk: String,
    ct: String,
}

/// Ciphertext envelope for IPFS. No subject, no body.
pub fn pack(env: &Envelope) -> Vec<u8> {
    serde_json::to_vec(&PackedLetter {
        v: 1,
        from: env.from.clone(),
        to: env.to.clone(),
        sent_at: env.sent_at,
        nonce: hex::encode(env.nonce),
        eph: hex::encode(env.ephemeral_pubkey),
        from_pk: hex::encode(env.from_pk),
        ct: hex::encode(&env.ciphertext),
    })
    .unwrap_or_default()
}

pub fn unpack(bytes: &[u8]) -> Result<Envelope, ZmailError> {
    let p: PackedLetter = serde_json::from_slice(bytes).map_err(|_| ZmailError::Crypto)?;
    Ok(Envelope {
        from: p.from,
        to: p.to,
        subject: String::new(),
        sent_at: p.sent_at,
        ciphertext: hex::decode(p.ct).map_err(|_| ZmailError::Crypto)?,
        nonce: decode_n12(&p.nonce)?,
        ephemeral_pubkey: decode_n32(&p.eph)?,
        from_pk: decode_n32(&p.from_pk).unwrap_or([0u8; 32]),
    })
}

fn decode_n12(s: &str) -> Result<[u8; 12], ZmailError> {
    let raw = hex::decode(s).map_err(|_| ZmailError::Crypto)?;
    if raw.len() != 12 {
        return Err(ZmailError::Crypto);
    }
    let mut n = [0u8; 12];
    n.copy_from_slice(&raw);
    Ok(n)
}

fn decode_n32(s: &str) -> Result<[u8; 32], ZmailError> {
    let raw = hex::decode(s).map_err(|_| ZmailError::Crypto)?;
    if raw.len() != 32 {
        return Err(ZmailError::Crypto);
    }
    let mut n = [0u8; 32];
    n.copy_from_slice(&raw);
    Ok(n)
}

pub fn open(recipient: &Identity, env: &Envelope) -> Result<Opened, ZmailError> {
    let eph = PublicKey::from(env.ephemeral_pubkey);
    let shared = recipient.secret.diffie_hellman(&eph);
    let from = Recipient::parse(&env.from)?;
    let to = Recipient::parse(&env.to)?;
    let key_bytes = derive_key(
        shared.as_bytes(),
        from.key().as_bytes(),
        to.key().as_bytes(),
    );
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(&env.nonce);
    let pt = cipher
        .decrypt(nonce, env.ciphertext.as_ref())
        .map_err(|_| ZmailError::Crypto)?;
    let v: serde_json::Value = serde_json::from_slice(&pt).map_err(|_| ZmailError::Crypto)?;
    Ok(Opened {
        from: env.from.clone(),
        to: env.to.clone(),
        subject: v
            .get("subject")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        sent_at: env.sent_at,
        body: v
            .get("body")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn derive_key(shared: &[u8], from: &[u8], to: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"vapurr-zmail-v1"), shared);
    let mut okm = [0u8; 32];
    let mut info = Vec::from(from);
    info.extend_from_slice(b"|");
    info.extend_from_slice(to);
    hk.expand(&info, &mut okm).expect("hkdf");
    okm
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub envelopes: Vec<Envelope>,
}

impl Mailbox {
    pub fn push(&mut self, env: Envelope) {
        self.envelopes.insert(0, env);
    }
}

pub struct Zmail {
    pub identity: Option<Identity>,
    pub mailbox: Mailbox,
}

impl Zmail {
    pub fn unverified() -> Self {
        Self {
            identity: None,
            mailbox: Mailbox::default(),
        }
    }

    pub fn for_handle(handle: Handle) -> Self {
        Self {
            identity: Some(Identity::generate(handle)),
            mailbox: Mailbox::default(),
        }
    }

    pub fn send_local(
        &mut self,
        to: Recipient,
        to_pk: &PublicKey,
        subject: &str,
        body: &str,
    ) -> Result<Envelope, ZmailError> {
        let id = self.identity.as_ref().ok_or(ZmailError::NeedKyc)?;
        let env = seal(id, &to, to_pk, subject, body);
        self.mailbox.push(env.clone());
        Ok(env)
    }

    pub fn inbox(&self) -> Vec<&Envelope> {
        self.mailbox.envelopes.iter().collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ZmailError {
    #[error("verify with zer0ID first")]
    NeedKyc,
    #[error("need their mailcard — they open zzzmail once")]
    NeedCard,
    #[error("unknown .hood name")]
    NeedName,
    #[error("bad .hood name")]
    BadName,
    #[error("that .hood is taken")]
    NameTaken,
    #[error("that .hood is reserved")]
    ReservedName,
    #[error("already have {0}")]
    AlreadyNamed(String),
    #[error("bad handle")]
    BadHandle,
    #[error("bad 0x address")]
    BadAddress,
    #[error("bad mailcard")]
    BadCard,
    #[error("bad cid")]
    BadCid,
    #[error("crypto")]
    Crypto,
    #[error("io")]
    Io,
    #[error("missing")]
    Missing,
    #[error("postage plus gas is over a cent")]
    OverCap,
    #[error("need testnet ETH for PNS — faucet.testnet.chain.robinhood.com")]
    NeedGas,
    #[error("rpc quiet")]
    Rpc,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let alice = Identity::generate(Handle::parse("alice").unwrap());
        let bob = Identity::generate(Handle::parse("bob").unwrap());
        let env = seal(
            &alice,
            &Recipient::Handle(bob.handle.clone()),
            &bob.public,
            "hi",
            "purr",
        );
        let opened = open(&bob, &env).unwrap();
        assert_eq!(opened.subject, "hi");
        assert_eq!(opened.body, "purr");
    }

    #[test]
    fn unverified_cannot_send() {
        let mut z = Zmail::unverified();
        let bob = Identity::generate(Handle::parse("bob").unwrap());
        assert!(z
            .send_local(Recipient::Handle(bob.handle.clone()), &bob.public, "x", "y")
            .is_err());
    }

    #[test]
    fn pointer_is_rhc() {
        assert_eq!(OnchainPointer::rhc("0xabc").chain_id, 4663);
    }

    #[test]
    fn recipient_parses_handle_and_address() {
        let h = Recipient::parse("@Alice").unwrap();
        assert_eq!(h.as_at(), "@alice");
        let a = Recipient::parse("0xC8ae558F58BaF209cF371e64b7baa84181A90060").unwrap();
        assert_eq!(
            a.as_at(),
            "@0xc8ae558f58baf209cf371e64b7baa84181a90060"
        );
        let a2 = Recipient::parse("@0xc8ae558f58baf209cf371e64b7baa84181a90060").unwrap();
        assert_eq!(a, a2);
        let n = Recipient::parse("@Alice.hood").unwrap();
        assert_eq!(n.as_at(), "@alice.hood");
        assert!(matches!(n, Recipient::Hood(_)));
        assert!(Handle::parse("alice.hood").is_err());
        assert!(Recipient::parse("0xdead").is_err());
        assert!(Recipient::parse("@ab").is_err());
        assert!(Handle::parse("0xc8ae558f58baf209cf371e64b7baa84181a90060").is_err());
    }

    #[test]
    fn seal_to_address() {
        let alice = Identity::generate(Handle::parse("alice").unwrap());
        let bob = Identity::generate(Handle::parse("bob").unwrap());
        let to = Recipient::parse("0xc8ae558f58baf209cf371e64b7baa84181a90060").unwrap();
        let env = seal(&alice, &to, &bob.public, "hi", "purr");
        assert_eq!(env.to, "@0xc8ae558f58baf209cf371e64b7baa84181a90060");
        let opened = open(&bob, &env).unwrap();
        assert_eq!(opened.body, "purr");
    }

    #[test]
    fn pack_is_cidable_and_has_no_body() {
        let alice = Identity::generate(Handle::parse("alice").unwrap());
        let bob = Identity::generate(Handle::parse("bob").unwrap());
        let env = seal(
            &alice,
            &Recipient::Handle(bob.handle.clone()),
            &bob.public,
            "hi",
            "secret-body",
        );
        let packed = pack(&env);
        let s = String::from_utf8_lossy(&packed);
        assert!(!s.contains("secret-body"));
        assert!(!s.contains("\"hi\""));
        let back = unpack(&packed).unwrap();
        assert_eq!(open(&bob, &back).unwrap().body, "secret-body");
        assert!(cid_raw_sha256(&packed).starts_with("bafkrei"));
    }
}
