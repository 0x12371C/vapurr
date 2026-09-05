//! One atomic, Windows-user protected record for both the key and recovery phrase.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};
use crate::{DeviceKey, WalletError};

const MAGIC: &[u8] = b"VAPURR-DPAPI-1\0";
static STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub(crate) struct Record {
    pub key: Vec<u8>,
    pub seed: Option<String>,
}

fn failure() -> WalletError { WalletError::Fail("Wallet storage could not be opened or saved. Keep your recovery backup; no replacement key was created.".into()) }
pub(crate) fn path() -> PathBuf { crate::data_dir().join("wallet.vault") }

#[cfg(windows)]
fn crypt(bytes: &[u8], encrypt: bool) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN};
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    let input = CRYPT_INTEGER_BLOB { cbData: bytes.len().try_into().map_err(|_| failure())?, pbData: bytes.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        if encrypt {
            CryptProtectData(&input, windows::core::w!("VAPURR wallet"), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        } else {
            CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        }.map_err(|_| failure())?;
        let result = Zeroizing::new(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec());
        std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize).zeroize();
        let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(result)
    }
}
#[cfg(not(windows))]
fn crypt(_bytes: &[u8], _encrypt: bool) -> Result<Zeroizing<Vec<u8>>, WalletError> {
    Err(WalletError::Fail("Encrypted wallet storage requires Windows".into()))
}

fn read_record(path: &Path) -> Result<Record, WalletError> {
    let bytes = std::fs::read(path).map_err(|_| failure())?;
    let encrypted = bytes.strip_prefix(MAGIC).ok_or_else(failure)?;
    let plain = crypt(encrypted, false)?;
    let record: Record = serde_json::from_slice(&plain).map_err(|_| failure())?;
    if DeviceKey::from_secret(&record.key).is_none() { return Err(failure()); }
    Ok(record)
}

fn write_record(path: &Path, record: &Record) -> Result<(), WalletError> {
    use std::io::Write;
    use rand::RngCore;
    let plain = Zeroizing::new(serde_json::to_vec(record).map_err(|_| failure())?);
    let encrypted = crypt(&plain, true)?;
    let mut bytes = MAGIC.to_vec(); bytes.extend_from_slice(&encrypted);
    let dir = path.parent().ok_or_else(failure)?;
    std::fs::create_dir_all(dir).map_err(|_| failure())?;
    let temporary = dir.join(format!("wallet-{:016x}.tmp", rand::rngs::OsRng.next_u64()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|_| failure())?;
        file.write_all(&bytes).map_err(|_| failure())?;
        file.sync_all().map_err(|_| failure())?;
        drop(file);
        let verified = read_record(&temporary)?;
        if verified.key != record.key || verified.seed != record.seed { return Err(failure()); }
        #[cfg(windows)]
        unsafe {
            use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
            MoveFileExW(&windows::core::HSTRING::from(temporary.as_os_str()), &windows::core::HSTRING::from(path.as_os_str()), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH).map_err(|_| failure())?;
        }
        #[cfg(not(windows))]
        return Err(failure());
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}

fn load_at(dir: &Path) -> Result<Option<Record>, WalletError> {
    let vault = dir.join("wallet.vault");
    let key_path = dir.join("device.sk");
    let seed_path = dir.join("seed.phrase");
    let record = if vault.exists() { read_record(&vault)? } else {
        if !key_path.exists() { return Ok(None); }
        let key = Zeroizing::new(std::fs::read(&key_path).map_err(|_| failure())?);
        if DeviceKey::from_secret(&key).is_none() { return Err(failure()); }
        let seed = if seed_path.exists() {
            Some(std::fs::read_to_string(&seed_path).map_err(|_| failure())?)
        } else { None };
        let record = Record { key: key.to_vec(), seed };
        write_record(&vault, &record)?;
        record
    };
    // Never remove a legacy key that differs from the verified encrypted record.
    if key_path.exists() {
        let legacy = Zeroizing::new(std::fs::read(&key_path).map_err(|_| failure())?);
        if legacy.as_slice() != record.key { return Err(failure()); }
        if seed_path.exists() {
            let seed = Zeroizing::new(std::fs::read_to_string(&seed_path).map_err(|_| failure())?);
            if record.seed.as_deref() != Some(seed.as_str()) { return Err(failure()); }
            std::fs::remove_file(&seed_path).map_err(|_| failure())?;
        }
        std::fs::remove_file(&key_path).map_err(|_| failure())?;
    } else if seed_path.exists() {
        let seed = Zeroizing::new(std::fs::read_to_string(&seed_path).map_err(|_| failure())?);
        if record.seed.as_deref() != Some(seed.as_str()) { return Err(failure()); }
        std::fs::remove_file(&seed_path).map_err(|_| failure())?;
    }
    Ok(Some(record))
}

pub(crate) fn load() -> Result<Option<Record>, WalletError> {
    let _guard = STORE_LOCK.lock().map_err(|_| failure())?;
    load_at(&crate::data_dir())
}
pub(crate) fn save(key: &DeviceKey, seed: Option<String>) -> Result<(), WalletError> {
    let _guard = STORE_LOCK.lock().map_err(|_| failure())?;
    // Migrate before replacement so the legacy cleanup can never delete the wrong key.
    let _ = load_at(&crate::data_dir())?;
    let raw = Zeroizing::new(key.secret_bytes()?);
    write_record(&path(), &Record { key: raw.to_vec(), seed })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn migration_preserves_key_and_phrase_and_removes_plaintext() {
        let dir = std::env::temp_dir().join(format!("vapurr-vault-test-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let key = vec![1u8;32];
        std::fs::write(dir.join("device.sk"), &key).unwrap();
        std::fs::write(dir.join("seed.phrase"), "synthetic recovery fixture").unwrap();
        let record = load_at(&dir).unwrap().unwrap();
        assert!(record.key == key);
        assert!(record.seed.as_deref() == Some("synthetic recovery fixture"));
        assert!(!dir.join("device.sk").exists()); assert!(!dir.join("seed.phrase").exists());
        let bytes = std::fs::read(dir.join("wallet.vault")).unwrap();
        assert!(!bytes.windows(key.len()).any(|w| w == key));
        assert!(!bytes.windows(9).any(|w| w == b"synthetic"));
        assert!(load_at(&dir).unwrap().unwrap().key == key);
        for name in ["wallet.vault"] { std::fs::remove_file(dir.join(name)).unwrap(); }
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn corrupt_vault_never_falls_back_or_destroys_legacy_key() {
        let dir = std::env::temp_dir().join(format!("vapurr-vault-test-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("device.sk"), [1u8;32]).unwrap();
        std::fs::write(dir.join("wallet.vault"), b"corrupt").unwrap();
        assert!(load_at(&dir).is_err());
        assert!(dir.join("device.sk").is_file());
        for name in ["wallet.vault", "device.sk"] { std::fs::remove_file(dir.join(name)).unwrap(); }
        std::fs::remove_dir(dir).unwrap();
    }
}
