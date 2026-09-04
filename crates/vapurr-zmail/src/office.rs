//! Seal → pin CID → $PUSD/$VAPURR voucher. No body on-chain. 0 ETH from sender.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use x25519_dalek::PublicKey;

use crate::card::{CardDir, Mailcard};
use crate::hood::{HoodName, HoodReceipt, HoodRegistry};
use crate::pin::{self, PinKind, Pinset};
use crate::postage::{quote, Asset, Quote, Voucher};
use crate::{
    open, pack, seal, unpack, Envelope, Identity, Recipient, ZmailError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub ok: bool,
    pub cid: String,
    pub pin: PinKind,
    pub postage: Quote,
    pub voucher: Voucher,
    pub from: String,
    pub to: String,
    pub body: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItem {
    pub cid: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub ts: i64,
    pub me: bool,
    pub pin: PinKind,
    pub postage: String,
    pub gasless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Log {
    #[serde(default)]
    nonce: u64,
    #[serde(default)]
    letters: Vec<InboxItem>,
}

pub struct PostOffice {
    root: PathBuf,
    identity: Identity,
    address: String,
    pinset: Pinset,
    cards: CardDir,
    hood: HoodRegistry,
    log: Log,
}

impl PostOffice {
    pub fn default_root() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData/Local"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vapurr")
            .join("zzzmail")
    }

    pub fn open_default() -> Result<Self, ZmailError> {
        Self::open(Self::default_root())
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, ZmailError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|_| ZmailError::Io)?;
        let identity = Identity::load_or_create(&root.join("identity.json"))?;
        let pinset = Pinset::open(&root)?;
        let cards = CardDir::open(&root)?;
        let hood = HoodRegistry::open(&root)?;
        let address = fs::read_to_string(root.join("address.txt"))
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| s.len() == 42 && s.starts_with("0x"))
            .unwrap_or_default();
        let log = load_log(&root.join("mailbox.json"));
        let mut identity = identity;
        if identity.hood.is_none() {
            if let Some(n) = hood.primary_of(&address) {
                identity.hood = Some(n);
                let _ = identity.save(&root.join("identity.json"));
            }
        }
        let office = Self {
            root,
            identity,
            address,
            pinset,
            cards,
            hood,
            log,
        };
        office.publish_card()?;
        Ok(office)
    }

    pub fn set_address(&mut self, address: &str) -> Result<(), ZmailError> {
        let a = address.trim().to_ascii_lowercase();
        if a.len() == 42 && a.starts_with("0x") {
            self.address = a.clone();
            fs::write(self.root.join("address.txt"), &a).map_err(|_| ZmailError::Io)?;
            self.publish_card()?;
        }
        Ok(())
    }

    pub fn me(&self) -> Mailcard {
        self.card()
    }

    pub fn quote(&self, asset: &str) -> Quote {
        quote(Asset::parse(asset), 0, 0).unwrap_or_else(|_| Quote::gasless(Asset::parse(asset)))
    }

    pub fn send(&mut self, to: &str, subject: &str, body: &str, asset: &str) -> Result<Receipt, ZmailError> {
        let id = &self.identity;
        let to_r = Recipient::parse(to)?;
        let pk = self.resolve_pk(&to_r)?;
        let asset = Asset::parse(asset);
        let postage = quote(asset, 0, 0).map_err(|_| ZmailError::OverCap)?;
        let env = seal(id, &to_r, &pk, subject, body);
        let packed = pack(&env);
        let (cid, pin) = self.pinset.put(&packed)?;
        if let Ok(card) = unpack_card(&packed) {
            let _ = self.cards.put(&card);
        }
        self.log.nonce = self.log.nonce.saturating_add(1);
        let now = env.sent_at.timestamp();
        let from_s = if self.address.is_empty() {
            id.handle.as_at()
        } else {
            self.address.clone()
        };
        let voucher = Voucher::new(
            from_s.as_str(),
            to_r.as_at().as_str(),
            &cid,
            asset,
            self.log.nonce,
            now as u64,
        );
        let item = InboxItem {
            cid: cid.clone(),
            from: env.from.clone(),
            to: env.to.clone(),
            body: body.to_string(),
            ts: now,
            me: true,
            pin,
            postage: postage.label.clone(),
            gasless: postage.gasless,
        };
        self.log.letters.insert(0, item);
        self.index_letter(&to_r, &cid)?;
        pin::relay_notify_inbox(to_r.as_at().as_str(), &cid);
        self.save_log()?;
        Ok(Receipt {
            ok: true,
            cid,
            pin,
            postage,
            voucher,
            from: env.from,
            to: env.to,
            body: body.to_string(),
            ts: now,
        })
    }

    pub fn inbox(&mut self) -> Vec<InboxItem> {
        self.pull();
        self.log.letters.clone()
    }

    pub fn snapshot(&mut self) -> serde_json::Value {
        let me = self.card();
        let q = self.quote("PUSD");
        serde_json::json!({
            "ok": true,
            "me": me,
            "quote": q,
            "hood": self.hood.snapshot(&self.address),
            "pns": self.hood.snapshot(&self.address),
            "inbox": self.inbox(),
        })
    }

    fn hood_chain_receipt(&self, name: &HoodName, tx: &str, already: bool) -> HoodReceipt {
        HoodReceipt {
            ok: true,
            kind: "hood".into(),
            pns: true,
            name: name.as_str().to_string(),
            node: name.node_hex(),
            owner: self.address.clone(),
            cid: String::new(),
            postage: self.quote("PUSD"),
            voucher: Voucher::new(
                &self.address,
                name.as_at().as_str(),
                "",
                Asset::Pusd,
                0,
                0,
            ),
            onchain: true,
            chain_id: vapurr_rhc::TESTNET_CHAIN_ID,
            tx: tx.to_string(),
            tx_url: if tx.is_empty() {
                String::new()
            } else {
                format!("{}/tx/{}", vapurr_rhc::TESTNET_EXPLORER, tx)
            },
            registry: crate::chain::registry_hex().unwrap_or_default(),
            need_gas: false,
            already,
        }
    }

    fn remember_hood(&mut self, name: &HoodName, x25519: &str) {
        let _ = self
            .hood
            .remember(name.clone(), &self.address, x25519, &self.pinset);
        self.identity.hood = Some(name.clone());
        let _ = self.identity.save(&self.root.join("identity.json"));
        let _ = self.publish_card();
    }

    pub fn register_hood(&mut self, name: &str) -> Result<HoodReceipt, ZmailError> {
        let name = HoodName::parse(name)?;
        if self.address.len() != 42 {
            return Err(ZmailError::BadAddress);
        }
        let pk = self.identity.public.as_bytes();
        let x25519 = hex::encode(pk);
        if cfg!(test) {
            let rec = self
                .hood
                .register(name.clone(), &self.address, &x25519, &self.pinset)?;
            self.identity.hood = Some(name);
            self.identity.save(&self.root.join("identity.json"))?;
            self.publish_card()?;
            return Ok(rec);
        }
        let tx = match crate::chain::register(&name, pk) {
            Ok(tx) => tx,
            Err(ZmailError::NameTaken) | Err(ZmailError::AlreadyNamed(_)) => {
                let ours = crate::chain::resolve(name.as_str())
                    .and_then(|v| {
                        v.get("record")
                            .and_then(|r| r.get("owner"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string())
                    })
                    .map(|o| o.eq_ignore_ascii_case(&self.address))
                    .unwrap_or(false);
                if !ours {
                    return Err(ZmailError::NameTaken);
                }
                self.remember_hood(&name, &x25519);
                return Ok(self.hood_chain_receipt(&name, "", true));
            }
            Err(e) => return Err(e),
        };
        self.remember_hood(&name, &x25519);
        Ok(self.hood_chain_receipt(&name, &tx, false))
    }

    pub fn reverse_hood(&self, addr: &str) -> Option<String> {
        if !cfg!(test) {
            if let Some(n) = crate::chain::reverse(addr) {
                return Some(n);
            }
        }
        self.reverse_hood_local(addr)
    }

    /// In-memory only. Never RPC. Safe on the WebView protocol thread.
    pub fn reverse_hood_local(&self, addr: &str) -> Option<String> {
        self.hood.reverse(addr).map(|s| s.to_string())
    }

    /// In-memory only. Never RPC. Safe on the WebView protocol thread.
    pub fn resolve_hood_local(&self, name: &str) -> Option<serde_json::Value> {
        let raw = name.trim().trim_start_matches('@');
        if raw.len() == 42 && raw.to_ascii_lowercase().starts_with("0x") {
            return self
                .reverse_hood_local(raw)
                .and_then(|n| self.resolve_hood_local(&n));
        }
        let parsed = HoodName::parse(raw).ok()?;
        match self.hood.resolve(&parsed) {
            Some(r) => Some(serde_json::json!({
                "ok": true,
                "pns": true,
                "kind": "hood",
                "service": "PNS",
                "onchain": false,
                "record": r
            })),
            None => None,
        }
    }

    pub fn resolve_hood(&self, name: &str) -> Result<serde_json::Value, ZmailError> {
        let raw = name.trim().trim_start_matches('@');
        if raw.len() == 42 && raw.to_ascii_lowercase().starts_with("0x") {
            return match self.reverse_hood(raw) {
                Some(n) => self.resolve_hood(&n),
                None => Err(ZmailError::NeedName),
            };
        }
        let parsed = HoodName::parse(raw)?;
        if !cfg!(test) {
            if let Some(v) = crate::chain::resolve(parsed.as_str()) {
                return Ok(v);
            }
        }
        match self.hood.resolve(&parsed) {
            Some(r) => Ok(serde_json::json!({
                "ok": true,
                "pns": true,
                "kind": "hood",
                "service": "PNS",
                "onchain": false,
                "record": r
            })),
            None => Err(ZmailError::NeedName),
        }
    }

    fn pull(&mut self) {
        let mut cids: Vec<String> = Vec::new();
        for key in self.my_keys() {
            cids.extend(self.listed_inbox(&key));
            cids.extend(pin::relay_list_inbox(&key));
        }
        for cid in self.pinset.list() {
            cids.push(cid);
        }
        cids.sort();
        cids.dedup();
        for cid in cids {
            if self.log.letters.iter().any(|l| l.cid == cid) {
                continue;
            }
            let Ok(bytes) = self.pinset.get(&cid) else {
                continue;
            };
            let Ok(env) = unpack(&bytes) else {
                continue;
            };
            if !self.addressed_to_me(&env) {
                continue;
            }
            let _ = self.open_cid(&cid);
        }
    }

    fn my_keys(&self) -> Vec<String> {
        let mut k = vec![self.identity.handle.as_at(), self.identity.mail_name()];
        if let Some(h) = &self.identity.hood {
            k.push(h.as_at());
            k.push(h.as_str().to_string());
        }
        if !self.address.is_empty() {
            k.push(self.address.clone());
            k.push(format!("@{}", self.address));
        }
        k.sort();
        k.dedup();
        k
    }

    fn addressed_to_me(&self, env: &Envelope) -> bool {
        Recipient::parse(&env.to)
            .map(|r| self.is_self(&r))
            .unwrap_or(false)
    }

    fn index_letter(&self, to: &Recipient, cid: &str) -> Result<(), ZmailError> {
        let dir = self.root.join("inbox").join(safe_key(to.key()));
        fs::create_dir_all(&dir).map_err(|_| ZmailError::Io)?;
        fs::write(dir.join(cid), b"").map_err(|_| ZmailError::Io)?;
        Ok(())
    }

    fn listed_inbox(&self, key: &str) -> Vec<String> {
        let dir = self.root.join("inbox").join(safe_key(key.trim().trim_start_matches('@')));
        let Ok(rd) = fs::read_dir(dir) else {
            return Vec::new();
        };
        rd.flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| crate::looks_like_cid(n))
            .collect()
    }

    pub fn open_cid(&mut self, cid: &str) -> Result<InboxItem, ZmailError> {
        let bytes = self.pinset.get(cid)?;
        let env = unpack(&bytes)?;
        if let Ok(card) = card_from_env(&env) {
            let _ = self.cards.put(&card);
        }
        let opened = open(&self.identity, &env)?;
        let item = InboxItem {
            cid: cid.to_string(),
            from: opened.from,
            to: opened.to,
            body: opened.body,
            ts: opened.sent_at.timestamp(),
            me: false,
            pin: PinKind::Local,
            postage: String::new(),
            gasless: true,
        };
        if !self.log.letters.iter().any(|l| l.cid == item.cid) {
            self.log.letters.insert(0, item.clone());
            let _ = self.save_log();
        }
        Ok(item)
    }

    fn resolve_pk(&self, to: &Recipient) -> Result<PublicKey, ZmailError> {
        if self.is_self(to) {
            return Ok(self.identity.public);
        }
        if let Recipient::Hood(n) = to {
            if let Some(rec) = self.hood.resolve(n) {
                if let Some(pk) = pk_from_hex(&rec.x25519) {
                    return Ok(pk);
                }
            }
            if !cfg!(test) {
                if let Some(v) = crate::chain::resolve(n.as_str()) {
                    if let Some(x) = v
                        .get("record")
                        .and_then(|r| r.get("x25519"))
                        .and_then(|x| x.as_str())
                    {
                        if let Some(pk) = pk_from_hex(x) {
                            return Ok(pk);
                        }
                    }
                }
            }
            return Err(ZmailError::NeedName);
        }
        if let Recipient::Address(a) = to {
            if let Some(card) = self.cards.get(to) {
                return card.pubkey();
            }
            if let Some(name) = self.reverse_hood(a) {
                return self.resolve_pk(&Recipient::parse(&name)?);
            }
            return Err(ZmailError::NeedCard);
        }
        self.cards
            .get(to)
            .ok_or(ZmailError::NeedCard)?
            .pubkey()
    }

    fn is_self(&self, to: &Recipient) -> bool {
        match to {
            Recipient::Hood(n) => self
                .identity
                .hood
                .as_ref()
                .map(|h| h == n)
                .unwrap_or(false),
            Recipient::Handle(h) => h.0 == self.identity.handle.0,
            Recipient::Address(a) => !self.address.is_empty() && a.eq_ignore_ascii_case(&self.address),
        }
    }

    fn card(&self) -> Mailcard {
        Mailcard {
            handle: self.identity.handle.0.clone(),
            address: self.address.clone(),
            x25519: hex::encode(self.identity.public.as_bytes()),
            hood: self
                .identity
                .hood
                .as_ref()
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
        }
    }

    fn publish_card(&self) -> Result<(), ZmailError> {
        self.cards.put(&self.card())
    }

    fn save_log(&self) -> Result<(), ZmailError> {
        let bytes = serde_json::to_vec_pretty(&self.log).map_err(|_| ZmailError::Io)?;
        fs::write(self.root.join("mailbox.json"), bytes).map_err(|_| ZmailError::Io)
    }
}

fn pk_from_hex(s: &str) -> Option<PublicKey> {
    let raw = hex::decode(s.trim().trim_start_matches("0x")).ok()?;
    if raw.len() != 32 {
        return None;
    }
    let mut b = [0u8; 32];
    b.copy_from_slice(&raw);
    Some(PublicKey::from(b))
}

fn safe_key(s: &str) -> String {
    s.trim()
        .trim_start_matches('@')
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

fn load_log(path: &Path) -> Log {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn unpack_card(bytes: &[u8]) -> Result<Mailcard, ZmailError> {
    card_from_env(&unpack(bytes)?)
}

fn card_from_env(env: &Envelope) -> Result<Mailcard, ZmailError> {
    if env.from_pk == [0u8; 32] {
        return Err(ZmailError::BadCard);
    }
    let who = Recipient::parse(&env.from)?;
    let hood = match &who {
        Recipient::Hood(n) => n.as_str().to_string(),
        _ => String::new(),
    };
    Ok(Mailcard {
        handle: who.key().to_string(),
        address: String::new(),
        x25519: hex::encode(env.from_pk),
        hood,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::OsRng, RngCore};

    fn tmp() -> PathBuf {
        let mut n = [0u8; 8];
        OsRng.fill_bytes(&mut n);
        std::env::temp_dir().join(format!(
            "zzzmail-office-{}-{}",
            std::process::id(),
            hex::encode(n)
        ))
    }

    #[test]
    fn self_send_pins_and_stays_under_a_cent() {
        let dir = tmp();
        let mut o = PostOffice::open(&dir).unwrap();
        o.set_address("0xc8ae558f58baf209cf371e64b7baa84181a90060")
            .unwrap();
        let me = o.me();
        let r = o.send(&me.at(), "", "purr", "PUSD").unwrap();
        assert!(r.ok);
        assert!(r.cid.starts_with("bafkrei"));
        assert!(r.postage.gasless);
        assert_eq!(r.postage.usd_micros, 2_500);
        assert_eq!(r.body, "purr");
        let opened = o.open_cid(&r.cid).unwrap();
        assert_eq!(opened.body, "purr");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unknown_recipient_needs_a_card() {
        let dir = tmp();
        let mut o = PostOffice::open(&dir).unwrap();
        let err = o.send("@nobodyyet", "", "hi", "PUSD").unwrap_err();
        assert!(matches!(err, ZmailError::NeedCard));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn vapurr_postage_is_gasless_too() {
        let dir = tmp();
        let mut o = PostOffice::open(&dir).unwrap();
        let me = o.me().at();
        let r = o.send(&me, "", "yo", "VAPURR").unwrap();
        assert_eq!(r.postage.asset, Asset::Vapurr);
        assert!(r.postage.gasless);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn hood_register_then_self_send() {
        let dir = tmp();
        let mut o = PostOffice::open(&dir).unwrap();
        o.set_address("0xc8ae558f58baf209cf371e64b7baa84181a90060")
            .unwrap();
        let rec = o.register_hood("alice").unwrap();
        assert_eq!(rec.name, "alice.hood");
        assert!(!rec.onchain);
        assert!(rec.tx.is_empty());
        assert!(rec.postage.gasless);
        assert_eq!(o.me().hood, "alice.hood");
        assert_eq!(o.me().at(), "@alice.hood");
        let r = o.send("@alice.hood", "", "hi from hood", "PUSD").unwrap();
        assert_eq!(r.to, "@alice.hood");
        assert_eq!(r.from, "@alice.hood");
        assert_eq!(o.open_cid(&r.cid).unwrap().body, "hi from hood");
        o.log.letters.clear();
        let pulled = o.inbox();
        assert!(pulled.iter().any(|l| l.body == "hi from hood"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unknown_hood_needs_name() {
        let dir = tmp();
        let mut o = PostOffice::open(&dir).unwrap();
        let err = o.send("@ghost.hood", "", "hi", "PUSD").unwrap_err();
        assert!(matches!(err, ZmailError::NeedName));
        let _ = fs::remove_dir_all(dir);
    }
}
