//! Local session. Not a network login. You are in, or you are not.

use serde_json::{json, Value};

use crate::{data_dir, device_key_path, DeviceKey, WalletError};

pub fn is_logged_in() -> bool {
    std::fs::read(session_path())
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v.get("on").and_then(|x| x.as_bool()))
        .unwrap_or(false)
}

pub fn has_key() -> bool {
    device_key_path().is_file()
}

pub fn has_seed() -> bool {
    seed_path().is_file()
}

pub fn peek_address() -> Option<String> {
    DeviceKey::load().map(|k| k.address.to_checksum())
}

pub fn status() -> Value {
    let addr = peek_address().unwrap_or_default();
    json!({
        "ok": true,
        "logged_in": is_logged_in(),
        "has_key": has_key(),
        "address": addr,
    })
}

pub fn login_continue() -> Result<Value, WalletError> {
    let addr = peek_address().ok_or(WalletError::Fail("no wallet on this PC".into()))?;
    write_session(&addr)?;
    Ok(status())
}

pub fn login_create() -> Result<Value, WalletError> {
    let (k, phrase) = crate::import::generate_phrase()?;
    k.save()?;
    save_phrase(&phrase);
    write_session(&k.address.to_checksum())?;
    let mut v = status();
    v["seed"] = json!(phrase);
    v["created"] = json!(true);
    Ok(v)
}

pub fn login_restore(secret: &str) -> Result<Value, WalletError> {
    let k = crate::import::import_text(secret)?;
    let words: Vec<&str> = secret.split_whitespace().collect();
    if words.len() == 12 || words.len() == 24 {
        save_phrase(&words.join(" "));
    } else {
        let _ = std::fs::remove_file(seed_path());
    }
    write_session(&k.address.to_checksum())?;
    Ok(status())
}

fn seed_path() -> std::path::PathBuf {
    data_dir().join("seed.phrase")
}

fn save_phrase(phrase: &str) {
    let _ = std::fs::create_dir_all(data_dir());
    let _ = std::fs::write(seed_path(), phrase.as_bytes());
}

pub fn reveal_seed() -> Result<Value, WalletError> {
    let raw = std::fs::read_to_string(seed_path()).map_err(|_| {
        WalletError::Fail(
            "no seed on this PC. This bag was restored from a key — export the hex key.".into(),
        )
    })?;
    let phrase = raw.trim().to_string();
    let n = phrase.split_whitespace().count();
    if n != 12 && n != 24 {
        return Err(WalletError::Fail("seed file is not 12 or 24 words".into()));
    }
    Ok(json!({
        "ok": true,
        "seed": phrase,
        "logged_in": is_logged_in(),
        "has_key": has_key(),
        "address": peek_address().unwrap_or_default(),
    }))
}

pub fn export_key() -> Result<Value, WalletError> {
    let k = DeviceKey::load().ok_or(WalletError::Fail("no wallet on this PC".into()))?;
    let raw = k.secret_bytes()?;
    let hex = format!("0x{}", hex::encode(raw));
    Ok(json!({
        "ok": true,
        "hex_key": hex,
        "logged_in": is_logged_in(),
        "has_key": has_key(),
        "address": k.address.to_checksum(),
    }))
}

pub fn logout() -> Value {
    let _ = std::fs::write(
        session_path(),
        serde_json::to_vec(&json!({ "on": false })).unwrap_or_default(),
    );
    status()
}

fn session_path() -> std::path::PathBuf {
    data_dir().join("session.json")
}

fn write_session(address: &str) -> Result<(), WalletError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let v = json!({ "on": true, "address": address, "at": now });
    let bytes = serde_json::to_vec(&v).map_err(|_| WalletError::Io)?;
    std::fs::write(session_path(), bytes).map_err(|_| WalletError::Io)
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
        assert!(v.get("address").and_then(|x| x.as_str()).is_some());
        assert!(has_seed() || !has_seed());
    }

    #[test]
    fn reveal_seed_missing_is_loud() {
        if seed_path().is_file() {
            return;
        }
        let e = reveal_seed().unwrap_err().to_string();
        assert!(e.to_ascii_lowercase().contains("seed") || e.to_ascii_lowercase().contains("key"));
    }
}
