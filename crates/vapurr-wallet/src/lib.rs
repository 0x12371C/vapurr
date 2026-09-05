//! Device keys and session keys. The shell shows a handle and a dollar figure.

use k256::ecdsa::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

use vapurr_id::VerifiedAccount;
use vapurr_rhc::{self as rhc, USDG};

pub mod desk;
mod import;
mod session;
mod keystore;
pub mod transactions;
pub mod tx;
pub use desk::{parse_units, Desk as WalletDesk, WalletCmd};
pub use session::{require_unlocked, has_key, is_logged_in, logout, peek_address, status as login_status};
pub use tx::{
    abi_addr, abi_u256, decode_abi_string, decode_dyn_string, decode_hex_bytes, decode_word_addr,
    decode_word_u128, encode_fn, encode_fn_addr, encode_fn_addr_addr, encode_fn_addr_u256,
    encode_fn_bytes32, encode_fn_bytes32_addr, encode_fn_four_u256, encode_fn_str,
    encode_fn_str_bytes32, encode_fn_two_addr_three_str_u256, encode_fn_two_addr_two_str_u256,
    encode_fn_two_str_u256,
    encode_fn_two_u256, encode_fn_u256, hex0x,
    revert_reason, Tx,
};

#[derive(Clone)]
pub struct DeviceKey {
    pub(crate) signing: SigningKey,
    pub address: Address,
}

impl Zeroize for DeviceKey {
    fn zeroize(&mut self) {
        self.address.0 = [0u8; 20];
    }
}

impl Drop for DeviceKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl std::fmt::Debug for DeviceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceKey")
            .field("address", &self.address.to_checksum())
            .finish_non_exhaustive()
    }
}

impl DeviceKey {
    pub fn generate() -> Self {
        let signing = SigningKey::random(&mut OsRng);
        let address = address_from_key(&signing);
        Self { signing, address }
    }

    pub fn load_result() -> Result<Option<Self>, WalletError> {
        let record = keystore::load()?;
        Ok(record.and_then(|r| Self::from_secret(&r.key)))
    }
    pub fn load() -> Option<Self> { Self::load_result().ok().flatten() }
    pub fn load_or_create() -> Result<Self, WalletError> {
        if let Some(key) = Self::load_result()? { return Ok(key); }
        let key = Self::generate(); key.save()?; Ok(key)
    }

    pub(crate) fn from_secret(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 32 {
            return None;
        }
        let mut sk = [0u8; 32];
        sk.copy_from_slice(bytes);
        let signing = SigningKey::from_bytes((&sk).into()).ok()?;
        let address = address_from_key(&signing);
        Some(Self { signing, address })
    }

    pub(crate) fn save(&self) -> Result<(), WalletError> { keystore::save(self, None) }

    pub(crate) fn secret_bytes(&self) -> Result<[u8; 32], WalletError> {
        let b = self.signing.to_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&b);
        Ok(out)
    }

    /// Sign a 32-byte digest (zzzmail postage voucher). 65-byte r||s||yParity.
    pub fn sign_digest(&self, hash: &[u8; 32]) -> Result<[u8; 65], WalletError> {
        let (sig, rec) = self
            .signing
            .sign_prehash_recoverable(hash)
            .map_err(|_| WalletError::Sign)?;
        let b = sig.to_bytes();
        let mut out = [0u8; 65];
        out[..32].copy_from_slice(&b[..32]);
        out[32..64].copy_from_slice(&b[32..]);
        out[64] = rec.to_byte();
        Ok(out)
    }
}

pub(crate) fn device_key_path() -> PathBuf {
    data_dir().join("device.sk")
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData/Local")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("vapurr")
}

pub fn addr_from_hex(s: &str) -> Option<Address> {
    let s = s.trim().trim_start_matches("0x");
    if s.len() != 40 {
        return None;
    }
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 20 {
        return None;
    }
    let mut a = [0u8; 20];
    a.copy_from_slice(&bytes);
    Some(Address(a))
}

fn address_from_key(sk: &SigningKey) -> Address {
    let pk = sk.verifying_key().to_encoded_point(false);
    let bytes = pk.as_bytes();
    let hash = Keccak256::digest(&bytes[1..]);
    let mut out = [0u8; 20];
    out.copy_from_slice(&hash[12..]);
    Address(out)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    pub fn to_checksum(&self) -> String {
        let hexed = hex::encode(self.0);
        let hash = Keccak256::digest(hexed.as_bytes());
        let mut out = String::from("0x");
        for (i, c) in hexed.chars().enumerate() {
            let nibble = hash[i / 2] >> if i % 2 == 0 { 4 } else { 0 } & 0x0f;
            if c.is_ascii_alphabetic() && nibble > 7 {
                out.push(c.to_ascii_uppercase());
            } else {
                out.push(c);
            }
        }
        out
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_checksum())
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Account {
    Guest,
    LocalUnverified {
        address: Address,
    },
    Verified {
        address: Address,
        account: VerifiedAccount,
    },
}

impl Account {
    pub fn display_name(&self) -> String {
        match self {
            Account::Guest => "guest".into(),
            Account::LocalUnverified { .. } => "unverified".into(),
            Account::Verified { account, .. } => format!("@{}", account.handle),
        }
    }

    pub fn is_verified(&self) -> bool {
        matches!(self, Account::Verified { .. })
    }

    pub fn address(&self) -> Option<Address> {
        match self {
            Account::Guest => None,
            Account::LocalUnverified { address } | Account::Verified { address, .. } => {
                Some(*address)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionMethod {
    PayX402,
    ZmailSend,
    DappCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    pub id: String,
    pub address: Address,
    pub spend_limit_usdg_minor: u128,
    pub spent_usdg_minor: u128,
    pub expiry_unix: u64,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<SessionMethod>,
}

impl SessionKey {
    pub fn active(&self, now: u64) -> bool {
        now < self.expiry_unix && self.spent_usdg_minor < self.spend_limit_usdg_minor
    }

    pub fn allows(&self, origin: &str, method: SessionMethod, amount: u128, now: u64) -> bool {
        self.active(now)
            && self.allowed_methods.contains(&method)
            && (self.allowed_origins.is_empty() || self.allowed_origins.iter().any(|o| o == origin))
            && self.spent_usdg_minor.saturating_add(amount) <= self.spend_limit_usdg_minor
    }

    pub fn debit(&mut self, amount: u128, now: u64) -> Result<(), WalletError> {
        if !self.active(now) {
            return Err(WalletError::SessionExpired);
        }
        let next = self.spent_usdg_minor.saturating_add(amount);
        if next > self.spend_limit_usdg_minor {
            return Err(WalletError::SpendCap);
        }
        self.spent_usdg_minor = next;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIntent {
    pub origin: String,
    pub method: SessionMethod,
    pub amount_usdg_minor: u128,
    pub note: String,
}

pub struct WalletFacade {
    pub account: Account,
    pub session: Option<SessionKey>,
    pub usdg_minor: u128,
    pub eth_wei: u128,
    pub card_usdc_minor: u128,
}

impl WalletFacade {
    pub fn guest() -> Self {
        Self {
            account: Account::Guest,
            session: None,
            usdg_minor: 0,
            eth_wei: 0,
            card_usdc_minor: 0,
        }
    }

    pub fn display_name(&self) -> String {
        self.account.display_name()
    }

    pub fn spendable_usd(&self) -> u128 {
        if self.account.is_verified() {
            self.usdg_minor.saturating_add(self.card_usdc_minor)
        } else {
            self.usdg_minor
        }
    }

    pub fn can_pay(&self, amount_minor: u128) -> bool {
        self.account.is_verified() && self.spendable_usd() >= amount_minor
    }

    pub fn sign_session_intent(
        &mut self,
        intent: SessionIntent,
    ) -> Result<SignedIntent, WalletError> {
        if !self.account.is_verified() {
            return Err(WalletError::NeedKyc);
        }
        let now = now_unix();
        let sess = self.session.as_mut().ok_or(WalletError::NoSession)?;
        if !sess.allows(&intent.origin, intent.method, intent.amount_usdg_minor, now) {
            return Err(WalletError::SessionDenied);
        }
        sess.debit(intent.amount_usdg_minor, now)?;
        Ok(SignedIntent {
            origin: intent.origin,
            method: intent.method,
            amount_usdg_minor: intent.amount_usdg_minor,
            session_id: sess.id.clone(),
        })
    }

    pub fn attach_unverified(key: &DeviceKey) -> Self {
        Self {
            account: Account::LocalUnverified {
                address: key.address,
            },
            session: None,
            usdg_minor: 0,
            eth_wei: 0,
            card_usdc_minor: 0,
        }
    }

    pub fn grant_verified_session(&mut self, limit_minor: u128) {
        if let Some(addr) = self.account.address() {
            self.session = Some(SessionKey {
                id: "sess_vapurr".into(),
                address: addr,
                spend_limit_usdg_minor: limit_minor,
                spent_usdg_minor: 0,
                expiry_unix: now_unix() + 7 * 86400,
                allowed_origins: vec![],
                allowed_methods: vec![
                    SessionMethod::PayX402,
                    SessionMethod::ZmailSend,
                    SessionMethod::DappCall,
                ],
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedIntent {
    pub origin: String,
    pub method: SessionMethod,
    pub amount_usdg_minor: u128,
    pub session_id: String,
}

pub fn keccak4(sig: &str) -> [u8; 4] {
    let hash = Keccak256::digest(sig.as_bytes());
    let mut out = [0u8; 4];
    out.copy_from_slice(&hash[0..4]);
    out
}

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    Keccak256::digest(bytes).into()
}

pub fn balance_of_calldata(holder: Address) -> [u8; 36] {
    let mut data = [0u8; 36];
    data[0..4].copy_from_slice(&keccak4("balanceOf(address)"));
    data[16..36].copy_from_slice(&holder.0);
    data
}

pub fn parse_hex_u64(s: &str) -> Result<u64, WalletError> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16).map_err(|_| WalletError::Rpc)
}

pub fn parse_hex_u128(s: &str) -> Result<u128, WalletError> {
    let s = s.trim_start_matches("0x");
    if s.is_empty() {
        return Ok(0);
    }
    u128::from_str_radix(s, 16).map_err(|_| WalletError::Rpc)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[allow(dead_code)]
fn _home() {
    let _ = (rhc::CHAIN_ID, USDG);
}

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("rpc")]
    Rpc,
    #[error("{0}")]
    Fail(String),
    #[error("sign")]
    Sign,
    #[error("io")]
    Io,
    #[error("verify with zer0ID first")]
    NeedKyc,
    #[error("no session")]
    NoSession,
    #[error("session denied")]
    SessionDenied,
    #[error("session expired")]
    SessionExpired,
    #[error("spend cap")]
    SpendCap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_of_selector() {
        assert_eq!(hex::encode(keccak4("balanceOf(address)")), "70a08231");
    }

    #[test]
    fn parse_quantities() {
        assert_eq!(parse_hex_u64("0x1237").unwrap(), 4663);
        assert_eq!(parse_hex_u128("0x124f80").unwrap(), 1_200_000);
    }

    #[test]
    fn session_spend_cap() {
        let mut s = SessionKey {
            id: "s".into(),
            address: Address([1; 20]),
            spend_limit_usdg_minor: 1_000_000,
            spent_usdg_minor: 0,
            expiry_unix: now_unix() + 60,
            allowed_origins: vec!["https://api.example".into()],
            allowed_methods: vec![SessionMethod::PayX402],
        };
        s.debit(500_000, now_unix()).unwrap();
        assert!(s.debit(600_000, now_unix()).is_err());
    }

    #[test]
    fn debug_hides_secret() {
        let k = DeviceKey::generate();
        let d = format!("{k:?}");
        assert!(!d.contains("signing"));
        assert_eq!(k.address.to_checksum().len(), 42);
        assert_eq!(k.address.to_hex().len(), 42);
    }

    #[test]
    fn guest_cannot_pay() {
        assert!(!WalletFacade::guest().can_pay(1));
    }
}
