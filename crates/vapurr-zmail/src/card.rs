//! Public mailcard: handle / 0x → X25519. Required to seal a letter.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use x25519_dalek::PublicKey;

use crate::{Handle, Recipient, ZmailError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailcard {
    pub handle: String,
    #[serde(default)]
    pub address: String,
    pub x25519: String,
    #[serde(default)]
    pub hood: String,
}

impl Mailcard {
    pub fn pubkey(&self) -> Result<PublicKey, ZmailError> {
        let raw = hex::decode(self.x25519.trim().trim_start_matches("0x")).map_err(|_| ZmailError::BadCard)?;
        if raw.len() != 32 {
            return Err(ZmailError::BadCard);
        }
        let mut b = [0u8; 32];
        b.copy_from_slice(&raw);
        Ok(PublicKey::from(b))
    }

    pub fn at(&self) -> String {
        if !self.hood.is_empty() {
            format!("@{}", self.hood)
        } else {
            format!("@{}", self.handle)
        }
    }
}

pub struct CardDir {
    root: PathBuf,
}

impl CardDir {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ZmailError> {
        let root = root.as_ref().join("cards");
        fs::create_dir_all(&root).map_err(|_| ZmailError::Io)?;
        Ok(Self { root })
    }

    pub fn put(&self, card: &Mailcard) -> Result<(), ZmailError> {
        let bytes = serde_json::to_vec_pretty(card).map_err(|_| ZmailError::Io)?;
        if !card.handle.is_empty() {
            fs::write(self.root.join(format!("{}.json", card.handle)), &bytes).map_err(|_| ZmailError::Io)?;
        }
        if card.address.len() == 42 {
            fs::write(
                self.root.join(format!("{}.json", card.address.to_ascii_lowercase())),
                &bytes,
            )
            .map_err(|_| ZmailError::Io)?;
        }
        if !card.hood.is_empty() {
            fs::write(self.root.join(format!("{}.json", card.hood)), &bytes).map_err(|_| ZmailError::Io)?;
        }
        Ok(())
    }

    pub fn get(&self, to: &Recipient) -> Option<Mailcard> {
        match to {
            Recipient::Hood(n) => self.read(&format!("{}.json", n.as_str())),
            Recipient::Handle(Handle(h)) => self.read(&format!("{h}.json")),
            Recipient::Address(a) => self.read(&format!("{}.json", a.to_ascii_lowercase())),
        }
    }

    fn read(&self, name: &str) -> Option<Mailcard> {
        let b = fs::read(self.root.join(name)).ok()?;
        serde_json::from_slice(&b).ok()
    }
}
