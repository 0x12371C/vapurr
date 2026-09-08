use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

const TTL: Duration = Duration::from_secs(45);
static NEXT: AtomicU64 = AtomicU64::new(1);
static EXECUTIONS: Mutex<Vec<Execution>> = Mutex::new(Vec::new());

#[derive(Clone)]
pub struct Execution {
    id: String,
    created: Instant,
    from: String,
    to: String,
    data: String,
    value: String,
    chain: u64,
}

impl Execution {
    pub fn ensure_fresh(&self) -> Result<(), String> {
        if self.created.elapsed() >= TTL { Err("Quote expired. Refresh and review again.".into()) }
        else { Ok(()) }
    }

    fn matches(&self, from: &str, to: &str, data: &str, value: &str, chain: u64) -> bool {
        addr_eq(&self.from, from) && addr_eq(&self.to, to) && self.data.eq_ignore_ascii_case(data)
            && self.value == value && self.chain == chain
    }
}

/// Called by the wallet worker using its actual key, never a browser-supplied sender.
pub fn take_execution(id: &str, from: &str, to: &str, data: &str, value: &str, chain: u64) -> Result<Execution, String> {
    let mut entries = EXECUTIONS.lock().map_err(|_| "route authorization unavailable")?;
    let index = entries.iter().position(|e| e.id == id).ok_or("Quote missing or already used. Refresh it.")?;
    let entry = entries.remove(index);
    entry.ensure_fresh()?;
    if !entry.matches(from, to, data, value, chain) { return Err("Transaction differs from the reviewed route.".into()); }
    Ok(entry)
}

fn issue(req: &QuoteReq, tx: &Value) -> Option<String> {
    let to = tx.get("to")?.as_str()?;
    let data = tx.get("data")?.as_str()?;
    let value = tx.get("value").and_then(Value::as_str).unwrap_or("0x0");
    let chain = tx.get("chainId").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(parse_chain)))?;
    if !real_wallet(&req.from_address) || !valid_address(to) || chain != req.from_chain { return None; }
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let id = hex::encode(keccak(format!("{serial}:{}:{}:{tx}", req.from_address, req.from_amount).as_bytes()));
    let mut entries = EXECUTIONS.lock().ok()?;
    entries.retain(|e| e.created.elapsed() < TTL);
    if entries.len() >= 64 { entries.remove(0); }
    entries.push(Execution { id:id.clone(), created:Instant::now(), from:req.from_address.clone(),
        to:to.into(), data:data.into(), value:value.into(), chain });
    Some(id)
}

pub(super) fn authorize(req: &QuoteReq, quote: &mut Value) {
    quote["expires_in_ms"] = json!(TTL.as_millis() as u64);
    if quote["payable"] == true {
        if let Some(id) = issue(req, &quote["tx"]) { quote["execution_id"] = json!(id); }
        else { quote["payable"] = json!(false); }
    }
    if quote["needs_approve"] == true {
        if let Some(id) = issue(req, &quote["approve"]) { quote["approve"]["execution_id"] = json!(id); }
        else { quote["needs_approve"] = json!(false); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authorization_binds_wallet_calldata_value_chain_and_expiry() {
        let e = Execution { id:"test".into(), created:Instant::now(), from:"alice".into(),
            to:"router".into(), data:"0x1234".into(), value:"0x0".into(), chain:46630 };
        assert!(e.matches("alice","router","0x1234","0x0",46630));
        assert!(!e.matches("bob","router","0x1234","0x0",46630));
        assert!(!e.matches("alice","other","0x1234","0x0",46630));
        assert!(!e.matches("alice","router","0xffff","0x0",46630));
        assert!(!e.matches("alice","router","0x1234","0x1",46630));
        assert!(!e.matches("alice","router","0x1234","0x0",1));
        let expired = Execution { created:Instant::now()-TTL, ..e };
        assert!(expired.ensure_fresh().is_err());
    }
    #[test]
    fn authorization_is_single_use() {
        let e = Execution { id:"single-use-test".into(), created:Instant::now(), from:"alice".into(),
            to:"router".into(), data:"0x1234".into(), value:"0x0".into(), chain:46630 };
        EXECUTIONS.lock().unwrap().push(e);
        assert!(take_execution("single-use-test","alice","router","0x1234","0x0",46630).is_ok());
        assert!(take_execution("single-use-test","alice","router","0x1234","0x0",46630).is_err());
    }
}
