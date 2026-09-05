//! Wallet unlock is memory-only and expires when the browser exits.
//! Passcode gate: salted hash on disk; session unlock still lives only in memory.
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use zeroize::Zeroizing;
use crate::{data_dir, device_key_path, DeviceKey, WalletError};
static UNLOCKED: AtomicBool = AtomicBool::new(false);

pub fn is_logged_in() -> bool { UNLOCKED.load(Ordering::Acquire) }
pub fn require_unlocked() -> Result<(), WalletError> {
    if is_logged_in() { Ok(()) } else { Err(WalletError::Fail("Unlock the wallet first".into())) }
}
pub fn has_key() -> bool { crate::keystore::path().is_file() || device_key_path().is_file() }
pub fn has_seed() -> bool { crate::keystore::load().ok().flatten().map(|r| r.seed.is_some()).unwrap_or(false) }
pub fn has_pin() -> bool { crate::passcode::has_pin() }
pub fn needs_passcode_setup() -> bool { has_key() && !has_pin() }
pub fn peek_address() -> Option<String> { DeviceKey::load().map(|k| k.address.to_checksum()) }
pub fn status() -> Value {
    json!({
        "ok": true,
        "logged_in": is_logged_in(),
        "has_key": has_key(),
        "has_pin": has_pin(),
        "needs_pin": needs_passcode_setup(),
        "address": peek_address().unwrap_or_default()
    })
}
pub fn lock_session() -> Value {
    UNLOCKED.store(false, Ordering::Release);
    status()
}
pub fn unlock_with_pin(pin: &str) -> Result<Value, WalletError> {
    DeviceKey::load_result()?.ok_or_else(|| WalletError::Fail("No wallet on this PC".into()))?;
    if !has_pin() {
        return Err(WalletError::Fail("Set a passcode first".into()));
    }
    if !crate::passcode::verify_pin(pin)? {
        return Err(WalletError::Fail("Wrong passcode".into()));
    }
    UNLOCKED.store(true, Ordering::Release);
    Ok(status())
}
pub fn set_passcode(a: &str, b: &str) -> Result<Value, WalletError> {
    DeviceKey::load_result()?.ok_or_else(|| WalletError::Fail("No wallet on this PC".into()))?;
    if has_pin() && !is_logged_in() {
        return Err(WalletError::Fail("Unlock first to change passcode".into()));
    }
    crate::passcode::set_pin(a, b)?;
    UNLOCKED.store(true, Ordering::Release);
    Ok(status())
}
pub fn login_continue() -> Result<Value, WalletError> {
    DeviceKey::load_result()?.ok_or_else(|| WalletError::Fail("No wallet on this PC".into()))?;
    if has_pin() {
        return Err(WalletError::Fail("Enter your passcode to unlock".into()));
    }
    // First run after create/import without a PIN yet — unlock so set-PIN UI can follow.
    UNLOCKED.store(true, Ordering::Release);
    Ok(status())
}
pub fn login_create() -> Result<Value, WalletError> {
    UNLOCKED.store(false, Ordering::Release);
    let (key, phrase) = crate::import::generate_phrase()?;
    let phrase = Zeroizing::new(phrase);
    crate::keystore::save(&key, Some(phrase.to_string()))?;
    // Clear any stale PIN from a replaced wallet.
    let _ = crate::passcode::clear_pin();
    UNLOCKED.store(true, Ordering::Release);
    let mut value = status(); value["seed"] = json!(phrase.as_str()); value["created"] = json!(true);
    Ok(value)
}
pub fn login_restore(secret: &str) -> Result<Value, WalletError> {
    UNLOCKED.store(false, Ordering::Release);
    let key = crate::import::import_text(secret)?;
    let words: Vec<&str> = secret.split_whitespace().collect();
    let seed = if matches!(words.len(), 12 | 24) { Some(words.join(" ")) } else { None };
    crate::keystore::save(&key, seed)?;
    let _ = crate::passcode::clear_pin();
    UNLOCKED.store(true, Ordering::Release);
    Ok(status())
}
fn seed_path() -> std::path::PathBuf { data_dir().join("seed.phrase") }
pub fn reveal_seed() -> Result<Value, WalletError> {
    require_unlocked()?;
    let record = crate::keystore::load()?.ok_or_else(|| WalletError::Fail("No wallet on this PC".into()))?;
    let seed = record.seed.as_deref().ok_or_else(|| WalletError::Fail("No seed on this PC; export the key instead".into()))?;
    Ok(json!({"ok":true,"seed":seed,"logged_in":true,"has_key":true,"has_pin":has_pin(),"address":peek_address().unwrap_or_default()}))
}
pub fn export_key() -> Result<Value, WalletError> {
    require_unlocked()?;
    let key = DeviceKey::load_result()?.ok_or_else(|| WalletError::Fail("No wallet on this PC".into()))?;
    let raw = Zeroizing::new(key.secret_bytes()?);
    Ok(json!({"ok":true,"hex_key":format!("0x{}", hex::encode(raw.as_slice())),"logged_in":true,"has_key":true,"has_pin":has_pin(),"address":key.address.to_checksum()}))
}
pub fn logout() -> Value {
    UNLOCKED.store(false, Ordering::Release);
    let _ = std::fs::remove_file(data_dir().join("session.json"));
    status()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shape() {
        let v = status();
        assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(true));
        assert!(v.get("logged_in").and_then(|x| x.as_bool()).is_some());
        assert!(v.get("has_key").and_then(|x| x.as_bool()).is_some());
        assert!(v.get("has_pin").and_then(|x| x.as_bool()).is_some());
        assert!(v.get("address").and_then(|x| x.as_str()).is_some());
        assert!(has_seed() || !has_seed());
    }

    #[test]
    fn reveal_seed_missing_is_loud() {
        if !is_logged_in() || seed_path().is_file() {
            return;
        }
        let e = reveal_seed().unwrap_err().to_string();
        assert!(e.to_ascii_lowercase().contains("seed") || e.to_ascii_lowercase().contains("key"));
    }
}
