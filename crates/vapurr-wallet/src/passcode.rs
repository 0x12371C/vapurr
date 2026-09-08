//! 4-digit passcode: salted iterated HMAC stored under LocalAppData\vapurr.
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::WalletError;

type HmacSha256 = Hmac<Sha256>;

const MAGIC: &str = "vapurr-pin-v1";
const ITERS: u32 = 120_000;
const FAIL_SOFT: u32 = 5;
const BACKOFF_SECS: u64 = 30;

static FAIL_COUNT: AtomicU32 = AtomicU32::new(0);
static LOCKOUT_UNTIL: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
struct Record {
    v: String,
    salt: String,
    hash: String,
    iters: u32,
}

fn path() -> PathBuf {
    crate::data_dir().join("passcode.json")
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn valid_pin(pin: &str) -> bool {
    pin.len() == 4 && pin.bytes().all(|b| b.is_ascii_digit())
}

fn derive(salt: &[u8], pin: &str, iters: u32) -> Result<[u8; 32], WalletError> {
    let mut block = [0u8; 32];
    {
        let mut mac = HmacSha256::new_from_slice(salt).map_err(|_| WalletError::Fail("passcode".into()))?;
        mac.update(pin.as_bytes());
        mac.update(MAGIC.as_bytes());
        let out = mac.finalize().into_bytes();
        block.copy_from_slice(&out);
    }
    for i in 1..iters {
        let mut mac = HmacSha256::new_from_slice(salt).map_err(|_| WalletError::Fail("passcode".into()))?;
        mac.update(&block);
        mac.update(&i.to_le_bytes());
        let out = mac.finalize().into_bytes();
        block.copy_from_slice(&out);
    }
    Ok(block)
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn has_pin() -> bool {
    path().is_file()
}

pub fn clear_pin() -> Result<(), WalletError> {
    let p = path();
    if p.exists() {
        std::fs::remove_file(&p).map_err(|_| WalletError::Fail("Could not clear passcode".into()))?;
    }
    FAIL_COUNT.store(0, Ordering::Release);
    LOCKOUT_UNTIL.store(0, Ordering::Release);
    Ok(())
}

fn lockout_remaining() -> u64 {
    let until = LOCKOUT_UNTIL.load(Ordering::Acquire);
    let now = now_unix();
    until.saturating_sub(now)
}

pub fn set_pin(a: &str, b: &str) -> Result<(), WalletError> {
    if !valid_pin(a) || !valid_pin(b) {
        return Err(WalletError::Fail("Passcode must be 4 digits".into()));
    }
    if a != b {
        return Err(WalletError::Fail("Passcodes did not match".into()));
    }
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let hash = derive(&salt, a, ITERS)?;
    let record = Record {
        v: MAGIC.into(),
        salt: hex::encode(salt),
        hash: hex::encode(hash),
        iters: ITERS,
    };
    let dir = crate::data_dir();
    std::fs::create_dir_all(&dir).map_err(|_| WalletError::Fail("Could not save passcode".into()))?;
    let bytes = serde_json::to_vec_pretty(&record).map_err(|_| WalletError::Fail("Could not save passcode".into()))?;
    let tmp = dir.join(format!("passcode-{:016x}.tmp", rand::random::<u64>()));
    std::fs::write(&tmp, &bytes).map_err(|_| WalletError::Fail("Could not save passcode".into()))?;
    #[cfg(windows)]
    {
        use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
        unsafe {
            MoveFileExW(
                &windows::core::HSTRING::from(tmp.as_os_str()),
                &windows::core::HSTRING::from(path().as_os_str()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
            .map_err(|_| WalletError::Fail("Could not save passcode".into()))?;
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(&tmp, path()).map_err(|_| WalletError::Fail("Could not save passcode".into()))?;
    }
    let _ = std::fs::remove_file(&tmp);
    FAIL_COUNT.store(0, Ordering::Release);
    LOCKOUT_UNTIL.store(0, Ordering::Release);
    Ok(())
}

pub fn verify_pin(pin: &str) -> Result<bool, WalletError> {
    if !valid_pin(pin) {
        return Ok(false);
    }
    let wait = lockout_remaining();
    if wait > 0 {
        return Err(WalletError::Fail(format!("Too many attempts. Try again in {wait}s")));
    }
    let bytes = std::fs::read(path()).map_err(|_| WalletError::Fail("Passcode not set".into()))?;
    let record: Record = serde_json::from_slice(&bytes).map_err(|_| WalletError::Fail("Passcode storage corrupt".into()))?;
    if record.v != MAGIC {
        return Err(WalletError::Fail("Passcode storage corrupt".into()));
    }
    let salt = hex::decode(&record.salt).map_err(|_| WalletError::Fail("Passcode storage corrupt".into()))?;
    let expected = hex::decode(&record.hash).map_err(|_| WalletError::Fail("Passcode storage corrupt".into()))?;
    let iters = if record.iters == 0 { ITERS } else { record.iters };
    let got = derive(&salt, pin, iters)?;
    if constant_eq(&got, &expected) {
        FAIL_COUNT.store(0, Ordering::Release);
        LOCKOUT_UNTIL.store(0, Ordering::Release);
        Ok(true)
    } else {
        let n = FAIL_COUNT.fetch_add(1, Ordering::AcqRel) + 1;
        if n >= FAIL_SOFT {
            LOCKOUT_UNTIL.store(now_unix() + BACKOFF_SECS, Ordering::Release);
            FAIL_COUNT.store(0, Ordering::Release);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_shape() {
        assert!(valid_pin("1234"));
        assert!(!valid_pin("12a4"));
        assert!(!valid_pin("123"));
        assert!(!valid_pin("12345"));
    }
}
