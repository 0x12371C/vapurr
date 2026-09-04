//! This-device wallet. Signs with DeviceKey. Follows market.json net (testnet until mainnet).

use serde_json::{json, Value};

use vapurr_rhc::{self as rhc, Rpc, USDG, USDG_DECIMALS, WETH};

use crate::{
    addr_from_hex, decode_hex_bytes, decode_word_u128, encode_fn_addr_u256, hex0x, Address,
    DeviceKey, Tx, WalletError,
};

const MIN_GAS_MAIN: u128 = 2_000_000_000_000_000;
const MIN_GAS_TEST: u128 = 100_000_000_000_000;
const ORBIT_GAS_FLOOR: u64 = 100_000;

pub enum WalletCmd {
    Snap,
    Send {
        asset: String,
        to: String,
        amt: String,
    },
    Import {
        secret: String,
    },
    LoginStatus,
    LoginContinue,
    LoginCreate,
    LoginRestore {
        secret: String,
    },
    Logout,
}

struct Net {
    rpc: Rpc,
    chain_id: u64,
    explorer: String,
    rpc_url: String,
    usdg: String,
    pusd: String,
    vapurr: String,
    testnet: bool,
    faucet: String,
}

fn cfg_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

fn load_net() -> Net {
    let v = std::fs::read(crate::data_dir().join("market.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .unwrap_or(Value::Null);
    let testnet = v.get("net").and_then(|x| x.as_str()).unwrap_or("testnet") != "mainnet";
    let pusd = cfg_str(&v, "pusd");
    let vapurr = cfg_str(&v, "vapurr");
    if testnet {
        let usdg = {
            let s = cfg_str(&v, "usdg");
            if s.is_empty() {
                rhc::TESTNET_USDG.to_string()
            } else {
                s
            }
        };
        Net {
            rpc: Rpc::at(rhc::TESTNET_RPC_HTTP),
            chain_id: rhc::TESTNET_CHAIN_ID,
            explorer: rhc::TESTNET_EXPLORER.into(),
            rpc_url: rhc::TESTNET_RPC_HTTP.into(),
            usdg,
            pusd,
            vapurr,
            testnet: true,
            faucet: rhc::TESTNET_FAUCET.into(),
        }
    } else {
        Net {
            rpc: Rpc::new(),
            chain_id: rhc::CHAIN_ID,
            explorer: rhc::EXPLORER.into(),
            rpc_url: rhc::RPC_HTTP.into(),
            usdg: USDG.into(),
            pusd,
            vapurr,
            testnet: false,
            faucet: String::new(),
        }
    }
}

pub struct Desk {
    rpc: Rpc,
    key: DeviceKey,
    last_tx: String,
    chain_id: u64,
    explorer: String,
    rpc_url: String,
    usdg: String,
    pusd: String,
    vapurr: String,
    testnet: bool,
    faucet: String,
}

impl Desk {
    pub fn open() -> Self {
        let n = load_net();
        Self {
            rpc: n.rpc,
            key: DeviceKey::load().unwrap_or_else(DeviceKey::generate),
            last_tx: String::new(),
            chain_id: n.chain_id,
            explorer: n.explorer,
            rpc_url: n.rpc_url,
            usdg: n.usdg,
            pusd: n.pusd,
            vapurr: n.vapurr,
            testnet: n.testnet,
            faucet: n.faucet,
        }
    }

    fn reload_net(&mut self) {
        let n = load_net();
        self.rpc = n.rpc;
        self.chain_id = n.chain_id;
        self.explorer = n.explorer;
        self.rpc_url = n.rpc_url;
        self.usdg = n.usdg;
        self.pusd = n.pusd;
        self.vapurr = n.vapurr;
        self.testnet = n.testnet;
        self.faucet = n.faucet;
    }

    pub fn run(&mut self, cmd: WalletCmd) -> Result<Value, WalletError> {
        self.reload_net();
        match cmd {
            WalletCmd::Snap => Ok(self.snap()),
            WalletCmd::Send { asset, to, amt } => self.send(&asset, &to, &amt),
            WalletCmd::Import { secret } => {
                let _ = crate::session::login_restore(&secret)?;
                self.reload_key();
                Ok(self.snap())
            }
            WalletCmd::LoginStatus => Ok(crate::session::status()),
            WalletCmd::LoginContinue => {
                crate::session::login_continue()?;
                self.reload_key();
                Ok(self.snap())
            }
            WalletCmd::LoginCreate => {
                let v = crate::session::login_create()?;
                self.reload_key();
                let mut snap = self.snap();
                if let Some(seed) = v.get("seed").cloned() {
                    snap["seed"] = seed;
                    snap["created"] = serde_json::json!(true);
                }
                Ok(snap)
            }
            WalletCmd::LoginRestore { secret } => {
                crate::session::login_restore(&secret)?;
                self.reload_key();
                Ok(self.snap())
            }
            WalletCmd::Logout => Ok(crate::session::logout()),
        }
    }

    fn reload_key(&mut self) {
        if let Some(k) = DeviceKey::load() {
            self.key = k;
        }
    }

    fn min_gas(&self) -> u128 {
        if self.testnet { MIN_GAS_TEST } else { MIN_GAS_MAIN }
    }

    pub fn snap(&self) -> Value {
        let logged_in = crate::session::is_logged_in();
        let has_key = crate::session::has_key();
        if !has_key {
            return json!({
                "ok": true,
                "live": false,
                "logged_in": false,
                "has_key": false,
                "error": "",
                "address": "",
                "chain_id": self.chain_id,
                "net": if self.testnet { "testnet" } else { "mainnet" },
                "rpc": self.rpc_url,
                "explorer": self.explorer,
                "faucet": self.faucet,
                "addr_url": "",
                "eth": "0",
                "eth_wei": "0",
                "usdg": "0",
                "usdg_raw": "0",
                "pusd": "0",
                "pusd_raw": "0",
                "vapurr": "0",
                "vapurr_raw": "0",
                "weth": "0",
                "weth_raw": "0",
                "nonce": 0,
                "need_eth": true,
                "tx": "",
                "tx_url": "",
                "assets": [],
                "activity": [],
                "total_usd": "0",
            });
        }
        let address = self.key.address.to_checksum();
        let from = self.key.address.to_hex();
        let mut err = String::new();
        let eth = match self.rpc.eth_balance(&from) {
            Ok(v) => v,
            Err(e) => {
                err = e.to_string();
                0
            }
        };
        let usdg = if self.usdg.is_empty() {
            0
        } else {
            token_bal(&self.rpc, &self.usdg, &from).unwrap_or(0)
        };
        let pusd = if self.pusd.is_empty() {
            0
        } else {
            token_bal(&self.rpc, &self.pusd, &from).unwrap_or(0)
        };
        let vapurr = if self.vapurr.is_empty() {
            0
        } else {
            token_bal(&self.rpc, &self.vapurr, &from).unwrap_or(0)
        };
        let weth = if self.testnet {
            0
        } else {
            token_bal(&self.rpc, WETH, &from).unwrap_or(0)
        };
        let nonce = self.rpc.eth_nonce(&from).unwrap_or(0);
        let usdg_s = fmt_units(usdg, USDG_DECIMALS);
        let pusd_s = fmt_units(pusd, 18);
        let vapurr_s = fmt_units(vapurr, 18);
        let eth_s = fmt_units(eth, 18);
        let weth_s = fmt_units(weth, 18);
        let activity = if self.testnet {
            vec![]
        } else {
            chain_activity(&from)
        };
        let mut assets = Vec::new();
        if !self.vapurr.is_empty() {
            assets.push(asset(
                "vapurr",
                "VAPURR",
                "Equity. Book burns it to mint $PUSD",
                &vapurr_s,
                vapurr,
                false,
            ));
        }
        if !self.pusd.is_empty() {
            assets.push(asset("pusd", "PUSD", "The dollar. Index rebases", &pusd_s, pusd, true));
        }
        if !self.usdg.is_empty() {
            assets.push(asset(
                "usdg",
                "USDG",
                if self.testnet {
                    "Book collateral (test)"
                } else {
                    "Book collateral"
                },
                &usdg_s,
                usdg,
                true,
            ));
        }
        assets.push(asset("eth", "ETH", "Gas", &eth_s, eth, false));
        if !self.testnet {
            assets.push(asset("weth", "WETH", "Wrapped ether", &weth_s, weth, false));
        }
        let total = if pusd > 0 { pusd_s.clone() } else { usdg_s.clone() };
        json!({
            "ok": err.is_empty(),
            "live": err.is_empty(),
            "logged_in": logged_in,
            "has_key": has_key,
            "error": err,
            "address": address,
            "chain_id": self.chain_id,
            "net": if self.testnet { "testnet" } else { "mainnet" },
            "rpc": self.rpc_url,
            "explorer": self.explorer,
            "faucet": self.faucet,
            "addr_url": format!("{}/address/{}", self.explorer, address),
            "eth": eth_s,
            "eth_wei": eth.to_string(),
            "usdg": usdg_s,
            "usdg_raw": usdg.to_string(),
            "pusd": pusd_s,
            "pusd_raw": pusd.to_string(),
            "vapurr": vapurr_s,
            "vapurr_raw": vapurr.to_string(),
            "weth": weth_s,
            "weth_raw": weth.to_string(),
            "nonce": nonce,
            "need_eth": eth < self.min_gas(),
            "tx": self.last_tx,
            "tx_url": if self.last_tx.is_empty() {
                String::new()
            } else {
                format!("{}/tx/{}", self.explorer, self.last_tx)
            },
            "assets": assets,
            "activity": activity,
            "total_usd": total,
        })
    }

    pub fn send(&mut self, asset: &str, to: &str, amt: &str) -> Result<Value, WalletError> {
        let to = addr_from_hex(to).ok_or_else(|| WalletError::Fail("bad address".into()))?;
        if to.0 == self.key.address.0 {
            return Err(WalletError::Fail("that is this device".into()));
        }
        let asset = asset.trim().to_ascii_lowercase();
        let hash = match asset.as_str() {
            "eth" => {
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                self.send_eth(to, value)?
            }
            "usdg" => {
                let value = parse_units(amt, USDG_DECIMALS)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                let token = addr_from_hex(&self.usdg).ok_or(WalletError::Rpc)?;
                self.send_token(token, to, value)?
            }
            "pusd" => {
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                let token = addr_from_hex(&self.pusd).ok_or_else(|| {
                    WalletError::Fail("no $PUSD on this net".into())
                })?;
                self.send_token(token, to, value)?
            }
            "vapurr" | "v" => {
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                let token = addr_from_hex(&self.vapurr).ok_or_else(|| {
                    WalletError::Fail("no $VAPURR on this net — deploy the market".into())
                })?;
                self.send_token(token, to, value)?
            }
            "weth" => {
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                self.send_token(
                    addr_from_hex(WETH).ok_or(WalletError::Rpc)?,
                    to,
                    value,
                )?
            }
            _ => return Err(WalletError::Fail("pick VAPURR, PUSD, USDG, or ETH".into())),
        };
        self.last_tx = hash;
        Ok(self.snap())
    }

    fn send_eth(&self, to: Address, value: u128) -> Result<String, WalletError> {
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(rpc_err)?;
        if eth <= value.saturating_add(self.min_gas() / 4) {
            return Err(WalletError::Fail("not enough ETH for gas".into()));
        }
        self.broadcast(Some(to), value, &[])
    }

    fn send_token(&self, token: Address, to: Address, value: u128) -> Result<String, WalletError> {
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(rpc_err)?;
        if eth < self.min_gas() / 4 {
            return Err(WalletError::Fail("need ETH for gas".into()));
        }
        let data = encode_fn_addr_u256("transfer(address,uint256)", to, value);
        self.broadcast(Some(token), 0, &data)
    }

    fn broadcast(&self, to: Option<Address>, value: u128, data: &[u8]) -> Result<String, WalletError> {
        let from = self.key.address.to_hex();
        let nonce = self.rpc.eth_nonce(&from).map_err(rpc_err)?;
        let gas_price = self.rpc.eth_gas_price().unwrap_or(100_000_000);
        let to_hex = to.map(|a| a.to_hex());
        let data_hex = hex0x(data);
        let est = self
            .rpc
            .eth_estimate_gas_value(&from, to_hex.as_deref(), &data_hex, value)
            .unwrap_or(if data.is_empty() { ORBIT_GAS_FLOOR } else { 200_000 });
        let mut gas = est.saturating_mul(13) / 10;
        if data.is_empty() {
            gas = gas.max(ORBIT_GAS_FLOOR);
        }
        let tx = Tx {
            chain_id: self.chain_id,
            nonce,
            max_priority_fee: 1_000_000,
            max_fee: gas_price.saturating_mul(3).max(1_000_000),
            gas,
            to,
            value,
            data: data.to_vec(),
        };
        let raw = self.key.sign_tx(&tx)?;
        let hash = self.rpc.eth_send_raw(&hex0x(&raw)).map_err(rpc_err)?;
        for _ in 0..80 {
            match self.rpc.eth_receipt(&hash) {
                Ok(Some(r)) => {
                    if r.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
                        return Err(WalletError::Fail("tx reverted".into()));
                    }
                    return Ok(hash);
                }
                _ => std::thread::sleep(std::time::Duration::from_millis(400)),
            }
        }
        Ok(hash)
    }
}

fn chain_activity(addr: &str) -> Vec<Value> {
    let raw = vapurr_rhc::scan::api("addr", &format!("a={addr}"));
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let rows = v
        .pointer("/addr/transfers")
        .or_else(|| v.get("transfers"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let me = addr.to_ascii_lowercase();
    rows.into_iter()
        .take(12)
        .map(|row| {
            let to = row
                .get("to")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let tx = row.get("tx").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let inbound = to == me;
            json!({
                "tx": tx,
                "url": if tx.is_empty() {
                    String::new()
                } else {
                    format!("{}/tx/{}", rhc::EXPLORER, tx)
                },
                "from": row.get("from").cloned().unwrap_or(json!("")),
                "to": row.get("to").cloned().unwrap_or(json!("")),
                "amount": row.get("amount").cloned().unwrap_or(json!("")),
                "dir": if inbound { "in" } else { "out" },
                "usdg": row.get("usdg").and_then(|x| x.as_bool()).unwrap_or(false),
            })
        })
        .collect()
}

fn token_bal(rpc: &Rpc, token: &str, holder: &str) -> Result<u128, WalletError> {
    let holder_addr = addr_from_hex(holder).ok_or(WalletError::Rpc)?;
    let data = hex0x(&crate::balance_of_calldata(holder_addr));
    let raw = rpc.eth_call(holder, Some(token), &data).map_err(rpc_err)?;
    let bytes = decode_hex_bytes(&raw).map_err(|_| WalletError::Rpc)?;
    Ok(decode_word_u128(&bytes, 0).unwrap_or(0))
}

fn asset(id: &str, symbol: &str, hint: &str, amount: &str, raw: u128, dollar: bool) -> Value {
    json!({
        "id": id,
        "symbol": symbol,
        "hint": hint,
        "amount": amount,
        "raw": raw.to_string(),
        "zero": raw == 0,
        "dollar": dollar,
    })
}

fn rpc_err(e: impl std::fmt::Display) -> WalletError {
    WalletError::Fail(e.to_string())
}

pub fn parse_units(s: &str, decimals: u8) -> Result<u128, WalletError> {
    let s = s.trim().replace(',', "");
    if s.is_empty() {
        return Err(WalletError::Fail("enter an amount".into()));
    }
    if s.starts_with('-') {
        return Err(WalletError::Fail("amount must be positive".into()));
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s.as_str(), ""),
    };
    if whole.chars().any(|c| !c.is_ascii_digit()) || frac.chars().any(|c| !c.is_ascii_digit()) {
        return Err(WalletError::Fail("bad amount".into()));
    }
    if frac.len() > decimals as usize {
        return Err(WalletError::Fail("too many decimals".into()));
    }
    let mut w: u128 = if whole.is_empty() {
        0
    } else {
        whole
            .parse()
            .map_err(|_| WalletError::Fail("amount too big".into()))?
    };
    let base = 10u128.pow(decimals as u32);
    w = w
        .checked_mul(base)
        .ok_or_else(|| WalletError::Fail("amount too big".into()))?;
    let mut f = 0u128;
    if !frac.is_empty() {
        let mut padded = frac.to_string();
        while padded.len() < decimals as usize {
            padded.push('0');
        }
        f = padded
            .parse()
            .map_err(|_| WalletError::Fail("bad amount".into()))?;
    }
    w.checked_add(f)
        .ok_or_else(|| WalletError::Fail("amount too big".into()))
}

pub fn fmt_units(n: u128, decimals: u8) -> String {
    let base = 10u128.pow(decimals as u32);
    let whole = n / base;
    let frac = n % base;
    if frac == 0 {
        return whole.to_string();
    }
    let mut f = format!("{:0width$}", frac, width = decimals as usize);
    while f.ends_with('0') {
        f.pop();
    }
    format!("{whole}.{f}")
}

#[cfg(test)]
mod tests {
    use super::{fmt_units, parse_units};

    #[test]
    fn units_round_trip() {
        assert_eq!(parse_units("1", 6).unwrap(), 1_000_000);
        assert_eq!(parse_units("1.5", 6).unwrap(), 1_500_000);
        assert_eq!(parse_units(".25", 6).unwrap(), 250_000);
        assert_eq!(parse_units("0.01", 18).unwrap(), 10u128.pow(16));
        assert_eq!(fmt_units(1_500_000, 6), "1.5");
        assert_eq!(fmt_units(10u128.pow(18), 18), "1");
        assert!(parse_units("1.1234567", 6).is_err());
        assert!(parse_units("-1", 6).is_err());
    }

    #[test]
    #[ignore]
    fn live_testnet_snap() {
        let d = super::Desk::open();
        let s = d.snap();
        eprintln!("{}", s);
        assert_eq!(s.get("net").and_then(|v| v.as_str()), Some("testnet"));
        assert_eq!(s.get("chain_id").and_then(|v| v.as_u64()), Some(46630));
        assert_eq!(s.get("ok").and_then(|v| v.as_bool()), Some(true));
        let eth = s.get("eth").and_then(|v| v.as_str()).unwrap_or("0");
        assert_ne!(eth, "0", "device has no testnet ETH: {s}");
    }
}
