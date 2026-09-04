//! Live JSON-RPC against Robinhood Chain. No keys. Read only.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{CHAIN_ID, RPC_HTTP, USDG};

pub const TRANSFER_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

const SYS: &[&str] = &[
    "0x0000000000000000000000000000000000000000",
    "0x00000000000000000000000000000000000a4b05",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainFeed {
    pub ok: bool,
    pub chain_id: u64,
    pub rpc: String,
    pub block: u64,
    pub hash: String,
    pub gwei: String,
    pub base_fee: String,
    pub gas_used: u64,
    pub l1: u64,
    pub txs: usize,
    pub user_txs: usize,
    pub usdg: usize,
    pub load: f64,
    pub skipped: u64,
    pub tps: f64,
    pub block_obj: Value,
    pub usdg_hashes: Vec<String>,
}

impl Default for ChainFeed {
    fn default() -> Self {
        Self {
            ok: false,
            chain_id: CHAIN_ID,
            rpc: RPC_HTTP.into(),
            block: 0,
            hash: "0x".into(),
            gwei: "—".into(),
            base_fee: "—".into(),
            gas_used: 0,
            l1: 0,
            txs: 0,
            user_txs: 0,
            usdg: 0,
            load: 0.0,
            skipped: 0,
            tps: 0.0,
            block_obj: Value::Null,
            usdg_hashes: vec![],
        }
    }
}

pub struct Rpc {
    http: reqwest::blocking::Client,
    url: String,
    last_block: u64,
    last_at: std::time::Instant,
    tps: f64,
    skipped: u64,
}

impl Rpc {
    pub fn new() -> Self {
        Self::at(RPC_HTTP)
    }

    pub fn at(url: impl Into<String>) -> Self {
        Self::at_timeout(url, 12)
    }

    /// Liquidity crawls getLogs + fat `eth_call` batches. Give them room.
    pub fn liq() -> Self {
        Self::at_timeout(RPC_HTTP, 30)
    }

    pub fn at_timeout(url: impl Into<String>, secs: u64) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(secs))
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .pool_max_idle_per_host(4)
            .tcp_nodelay(true)
            .user_agent("vapurr/0.1")
            .build()
            .expect("rhc rpc client");
        Self {
            http,
            url: url.into(),
            last_block: 0,
            last_at: std::time::Instant::now(),
            tps: 0.0,
            skipped: 0,
        }
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let resp: Value = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .map_err(|_| RpcError::Transport)?
            .json()
            .map_err(|_| RpcError::Decode)?;
        if let Some(err) = resp.get("error") {
            let msg = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("rpc");
            let data = err.get("data").and_then(|v| v.as_str()).unwrap_or("");
            return Err(RpcError::Remote(format!("{msg} {data}").trim().into()));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn eth_call(&self, from: &str, to: Option<&str>, data: &str) -> Result<String, RpcError> {
        let mut obj = serde_json::Map::new();
        obj.insert("from".into(), json!(from));
        if let Some(t) = to {
            obj.insert("to".into(), json!(t));
        }
        obj.insert("data".into(), json!(data));
        let v = self.call("eth_call", json!([Value::Object(obj), "latest"]))?;
        Ok(v.as_str().unwrap_or("0x").into())
    }

    pub fn eth_estimate_gas(
        &self,
        from: &str,
        to: Option<&str>,
        data: &str,
    ) -> Result<u64, RpcError> {
        self.eth_estimate_gas_value(from, to, data, 0)
    }

    pub fn eth_estimate_gas_value(
        &self,
        from: &str,
        to: Option<&str>,
        data: &str,
        value: u128,
    ) -> Result<u64, RpcError> {
        let mut obj = serde_json::Map::new();
        obj.insert("from".into(), json!(from));
        if let Some(t) = to {
            obj.insert("to".into(), json!(t));
        }
        obj.insert("data".into(), json!(data));
        if value > 0 {
            obj.insert("value".into(), json!(format!("0x{value:x}")));
        }
        let v = self.call("eth_estimateGas", json!([Value::Object(obj)]))?;
        Ok(hex_u64(&v))
    }

    pub fn eth_balance(&self, addr: &str) -> Result<u128, RpcError> {
        let v = self.call("eth_getBalance", json!([addr, "latest"]))?;
        Ok(hex_u128(&v))
    }

    pub fn eth_nonce(&self, addr: &str) -> Result<u64, RpcError> {
        let v = self.call("eth_getTransactionCount", json!([addr, "pending"]))?;
        Ok(hex_u64(&v))
    }

    pub fn eth_gas_price(&self) -> Result<u128, RpcError> {
        Ok(hex_u128(&self.call("eth_gasPrice", json!([]))?))
    }

    pub fn eth_code(&self, addr: &str) -> Result<String, RpcError> {
        let v = self.call("eth_getCode", json!([addr, "latest"]))?;
        Ok(v.as_str().unwrap_or("0x").into())
    }

    pub fn eth_send_raw(&self, raw: &str) -> Result<String, RpcError> {
        let v = self.call("eth_sendRawTransaction", json!([raw]))?;
        v.as_str()
            .map(|s| s.to_string())
            .ok_or(RpcError::Decode)
    }

    pub fn eth_receipt(&self, hash: &str) -> Result<Option<Value>, RpcError> {
        let v = self.call("eth_getTransactionReceipt", json!([hash]))?;
        if v.is_null() {
            Ok(None)
        } else {
            Ok(Some(v))
        }
    }

    pub fn batch(&self, reqs: &[Value]) -> Result<Vec<Value>, RpcError> {
        if reqs.is_empty() {
            return Ok(Vec::new());
        }
        let resp: Value = self
            .http
            .post(&self.url)
            .json(reqs)
            .send()
            .map_err(|_| RpcError::Transport)?
            .json()
            .map_err(|_| RpcError::Decode)?;
        let arr = match resp.as_array() {
            Some(a) => a,
            None => return Err(RpcError::Decode),
        };
        let mut by_id = std::collections::HashMap::new();
        for item in arr {
            let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            if item.get("error").is_some() {
                by_id.insert(id, Value::Null);
            } else {
                by_id.insert(id, item.get("result").cloned().unwrap_or(Value::Null));
            }
        }
        Ok(reqs
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let id = r.get("id").and_then(|v| v.as_u64()).unwrap_or(i as u64);
                by_id.get(&id).cloned().unwrap_or(Value::Null)
            })
            .collect())
    }

    /// `want_body` pulls the full block + USDG logs. Home no longer needs that
    /// (the mill is gone). Head-only is block number + gas for the chrome chip.
    pub fn poll(&mut self, want_body: bool) -> Result<Option<ChainFeed>, RpcError> {
        let bn = hex_u64(&self.call("eth_blockNumber", json!([]))?);
        let gp = self.call("eth_gasPrice", json!([]))?;
        let mut feed = ChainFeed {
            ok: true,
            gwei: fmt_gwei(&gp),
            ..ChainFeed::default()
        };
        if bn == 0 {
            return Ok(Some(feed));
        }
        if bn == self.last_block {
            return Ok(None);
        }
        if self.last_block != 0 && bn > self.last_block + 1 {
            self.skipped += bn - self.last_block - 1;
        }
        if !want_body {
            self.last_block = bn;
            feed.block = bn;
            return Ok(Some(feed));
        }
        let hex_n = format!("0x{bn:x}");
        let from = if self.last_block == 0 {
            bn
        } else {
            self.last_block + 1
        };
        let hex_from = format!("0x{from:x}");
        let block = self.call("eth_getBlockByNumber", json!([hex_n, true]))?;
        let logs = self
            .call(
                "eth_getLogs",
                json!([{
                    "address": USDG,
                    "fromBlock": hex_from,
                    "toBlock": hex_n,
                    "topics": [TRANSFER_TOPIC]
                }]),
            )
            .unwrap_or(Value::Array(vec![]));

        let mut usdg_hashes = Vec::new();
        if let Some(arr) = logs.as_array() {
            for log in arr {
                if let Some(h) = log.get("transactionHash").and_then(|v| v.as_str()) {
                    usdg_hashes.push(h.to_ascii_lowercase());
                }
            }
        }
        let usdg_set: std::collections::HashSet<String> =
            usdg_hashes.iter().cloned().collect();

        let mut txs: Vec<Value> = block
            .get("transactions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let tx_count = txs.len();
        if txs.len() > 48 {
            txs.truncate(48);
        }

        let mut user_txs = 0usize;
        let mut usdg_hits = 0usize;
        for tx in &txs {
            let from = tx
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let to = tx
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let hash = tx
                .get("hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if is_sys(&from) && is_sys(&to) {
                continue;
            }
            user_txs += 1;
            if usdg_set.contains(&hash) || to == USDG.to_ascii_lowercase() {
                usdg_hits += 1;
            }
        }

        let now = std::time::Instant::now();
        if self.last_block != 0 && bn != self.last_block {
            let dt = now.duration_since(self.last_at).as_secs_f64();
            if dt > 0.12 && dt < 8.0 {
                let inst = user_txs as f64 / dt;
                self.tps = if self.tps == 0.0 {
                    inst
                } else {
                    self.tps * 0.7 + inst * 0.3
                };
            }
            self.last_at = now;
        }
        self.last_block = bn;

        let gas_used = hex_u64(block.get("gasUsed").unwrap_or(&Value::Null));
        let mut slim = block.clone();
        if let Some(obj) = slim.as_object_mut() {
            obj.insert("transactions".into(), Value::Array(txs));
        }

        feed.block = bn;
        feed.hash = block
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("0x")
            .into();
        feed.base_fee = fmt_gwei(block.get("baseFeePerGas").unwrap_or(&Value::Null));
        feed.gas_used = gas_used;
        feed.l1 = hex_u64(block.get("l1BlockNumber").unwrap_or(&Value::Null));
        feed.txs = tx_count;
        feed.user_txs = user_txs;
        feed.usdg = usdg_hits;
        feed.load = (gas_used as f64 / 8_000_000.0).clamp(0.0, 1.0);
        feed.skipped = self.skipped;
        feed.tps = self.tps;
        feed.block_obj = slim;
        feed.usdg_hashes = usdg_hashes;
        Ok(Some(feed))
    }
}

impl Default for Rpc {
    fn default() -> Self {
        Self::new()
    }
}

fn is_sys(addr: &str) -> bool {
    addr.is_empty() || SYS.iter().any(|s| *s == addr)
}

pub(crate) fn hex_u64(v: &Value) -> u64 {
    let s = match v {
        Value::String(s) => s.as_str(),
        _ => return 0,
    };
    u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
}

pub(crate) fn hex_u128(v: &Value) -> u128 {
    let s = match v {
        Value::String(s) => s.as_str(),
        _ => return 0,
    };
    let s = s.trim_start_matches("0x");
    if s.is_empty() {
        return 0;
    }
    u128::from_str_radix(s, 16).unwrap_or(0)
}

/// Low 128 bits of an ABI word. `eth_call` returns 32 bytes; `u128::from_str_radix`
/// on 64 hex chars overflows to 0.
pub(crate) fn abi_u128(v: &Value) -> u128 {
    let s = match v {
        Value::String(s) => s.trim_start_matches("0x"),
        _ => return 0,
    };
    if s.is_empty() {
        return 0;
    }
    let take = if s.len() > 32 { &s[s.len() - 32..] } else { s };
    u128::from_str_radix(take, 16).unwrap_or(0)
}

pub(crate) fn fmt_gwei(v: &Value) -> String {
    let s = match v {
        Value::String(s) => s.as_str(),
        _ => return "—".into(),
    };
    let wei = u128::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let gwei = wei as f64 / 1e9;
    if !gwei.is_finite() {
        return "—".into();
    }
    if gwei < 10.0 {
        format!("{gwei:.2} GWEI")
    } else {
        format!("{gwei:.1} GWEI")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("transport")]
    Transport,
    #[error("decode")]
    Decode,
    #[error("{0}")]
    Remote(String),
}
