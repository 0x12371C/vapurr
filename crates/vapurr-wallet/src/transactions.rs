//! Persist submitted hashes; absence of a receipt never means settlement.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;
use crate::WalletError;

static JOURNAL: Mutex<()> = Mutex::new(());
static SIGNING: Mutex<()> = Mutex::new(());
pub fn signing_guard() -> Result<std::sync::MutexGuard<'static, ()>, WalletError> {
    crate::require_unlocked()?;
    SIGNING.try_lock().map_err(|_| WalletError::Fail("Another wallet transaction is in progress. Wait for its result.".into()))
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction { pub hash: String, pub chain_id: u64, pub from: String, pub status: String }

pub fn receipt_status(receipt: Option<&Value>) -> &'static str {
    match receipt.and_then(|r| r.get("status")).and_then(Value::as_str) {
        Some("0x1") => "confirmed", Some("0x0") => "reverted", _ => "pending",
    }
}
fn path() -> std::path::PathBuf { crate::data_dir().join("transactions.json") }
fn read() -> Result<Vec<Transaction>, WalletError> {
    match std::fs::read(path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| WalletError::Fail("Transaction journal needs recovery; do not resend pending transactions".into())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(_) => Err(WalletError::Io),
    }
}
fn write(rows: &[Transaction]) -> Result<(), WalletError> {
    use std::io::Write;
    std::fs::create_dir_all(crate::data_dir()).map_err(|_| WalletError::Io)?;
    let temp = path().with_extension("tmp");
    let mut file = std::fs::File::create(&temp).map_err(|_| WalletError::Io)?;
    file.write_all(&serde_json::to_vec(rows).map_err(|_| WalletError::Io)?).map_err(|_| WalletError::Io)?;
    file.sync_all().map_err(|_| WalletError::Io)?; drop(file);
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
        MoveFileExW(&windows::core::HSTRING::from(temp.as_os_str()), &windows::core::HSTRING::from(path().as_os_str()), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH).map_err(|_| WalletError::Io)?;
    }
    #[cfg(not(windows))]
    std::fs::rename(temp, path()).map_err(|_| WalletError::Io)?;
    Ok(())
}
pub fn record(hash: &str, chain_id: u64, from: &str, status: &str) -> Result<(), WalletError> {
    let _guard = JOURNAL.lock().map_err(|_| WalletError::Io)?;
    let mut rows = read()?;
    if let Some(row) = rows.iter_mut().find(|r| r.hash == hash && r.chain_id == chain_id) {
        row.status = status.into();
    } else { rows.push(Transaction { hash: hash.into(), chain_id, from: from.into(), status: status.into() }); }
    if rows.len() > 256 {
        if let Some(i) = rows.iter().position(|r| r.status != "pending") { rows.remove(i); }
    }
    write(&rows)
}
pub fn latest(from: &str, chain_id: u64) -> Result<Option<Transaction>, WalletError> {
    let _guard = JOURNAL.lock().map_err(|_| WalletError::Io)?;
    Ok(read()?.into_iter().rev().find(|r| r.chain_id == chain_id && r.from.eq_ignore_ascii_case(from)))
}
pub fn refresh(mut tx: Transaction) -> Transaction {
    if tx.status == "pending" {
        if let Some(url) = vapurr_rhc::rpc_http(tx.chain_id) {
            if let Ok(receipt) = vapurr_rhc::Rpc::at_timeout(url, 4).eth_receipt(&tx.hash) {
                let status = receipt_status(receipt.as_ref());
                if status != "pending" && record(&tx.hash, tx.chain_id, &tx.from, status).is_ok() { tx.status = status.into(); }
            }
        }
    }
    tx
}
pub fn ensure_no_pending(from: &str, chain: u64) -> Result<(), WalletError> {
    if let Some(tx) = latest(from, chain)? {
        let tx = refresh(tx);
        if tx.status == "pending" { return Err(WalletError::Fail(format!("Transaction {} is still pending. Check its receipt before sending again.", tx.hash))); }
    }
    Ok(())
}
pub fn status_json(chain: u64, hash: &str) -> Value {
    let row = { let Ok(_guard) = JOURNAL.lock() else { return json!({"ok":false,"error":"Journal unavailable"}) };
        read().ok().and_then(|rows| rows.into_iter().find(|r| r.hash == hash && r.chain_id == chain)) };
    match row {
        Some(tx) => { let tx = refresh(tx); json!({"ok":true,"tx":tx.hash,"tx_chain_id":tx.chain_id,"tx_status":tx.status}) },
        None => json!({"ok":false,"error":"Unknown transaction"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_successful_receipts_confirm_payment() {
        assert_eq!(receipt_status(None), "pending");
        assert_eq!(receipt_status(Some(&json!({}))), "pending");
        assert_eq!(receipt_status(Some(&json!({"status":"0x1"}))), "confirmed");
        assert_eq!(receipt_status(Some(&json!({"status":"0x0"}))), "reverted");
    }
}
