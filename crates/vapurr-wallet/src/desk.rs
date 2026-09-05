//! This-device wallet. Signs with DeviceKey. Follows market.json net (testnet until mainnet).

use serde_json::{json, Value};

use vapurr_rhc::{self as rhc, Rpc, USDG, USDG_DECIMALS, WETH};

use crate::{
    addr_from_hex, decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn_addr,
    encode_fn_addr_u256, encode_fn_str, hex0x, Address, DeviceKey, Tx, WalletError,
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
    LockSession,
    PasscodeUnlock {
        code: String,
    },
    PasscodeSet {
        a: String,
        b: String,
    },
    SetNet(String),
    RevealSeed,
    ExportKey,
    Resolve {
        to: String,
    },
    Exec {
        to: String,
        data: String,
        value: String,
        chain_id: u64,
        gas: u64,
    },
}

struct Net {
    rpc: Rpc,
    chain_id: u64,
    explorer: String,
    rpc_url: String,
    usdg: String,
    pusd: String,
    vapurr: String,
    loop_vault: String,
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

fn normalize_net(net: &str) -> &'static str {
    if net.trim().eq_ignore_ascii_case("mainnet") {
        "mainnet"
    } else {
        "testnet"
    }
}

fn write_net(net: &str) -> Result<(), WalletError> {
    let n = normalize_net(net);
    let path = crate::data_dir().join("market.json");
    let mut v = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .unwrap_or_else(|| json!({}));
    if !v.is_object() {
        v = json!({});
    }
    if let Some(obj) = v.as_object_mut() {
        obj.insert("net".into(), json!(n));
    }
    let bytes = serde_json::to_vec_pretty(&v).map_err(|_| WalletError::Io)?;
    let _ = std::fs::create_dir_all(crate::data_dir());
    std::fs::write(path, bytes).map_err(|_| WalletError::Io)?;
    Ok(())
}

fn load_net() -> Net {
    let v = std::fs::read(crate::data_dir().join("market.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .unwrap_or(Value::Null);
    let testnet = v.get("net").and_then(|x| x.as_str()).unwrap_or("testnet") != "mainnet";
    let mut pusd = cfg_str(&v, "pusd");
    let mut vapurr = cfg_str(&v, "vapurr");
    let mut loop_vault = cfg_str(&v, "loop");
    if testnet {
        if pusd.is_empty() {
            pusd = rhc::TESTNET_PUSD.to_string();
        }
        if vapurr.is_empty() {
            vapurr = rhc::TESTNET_VAPURR.to_string();
        }
        if loop_vault.is_empty() {
            loop_vault = rhc::TESTNET_LOOP.to_string();
        }
        let usdg = {
            let s = cfg_str(&v, "usdg");
            if s.is_empty() {
                rhc::TESTNET_MOCK_USDG.to_string()
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
            loop_vault,
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
            loop_vault,
            testnet: false,
            faucet: String::new(),
        }
    }
}

pub struct Desk {
    rpc: Rpc,
    key: DeviceKey,
    last_tx: String,
    last_tx_chain: u64,
    chain_id: u64,
    explorer: String,
    rpc_url: String,
    usdg: String,
    pusd: String,
    vapurr: String,
    loop_vault: String,
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
            last_tx_chain: n.chain_id,
            chain_id: n.chain_id,
            explorer: n.explorer,
            rpc_url: n.rpc_url,
            usdg: n.usdg,
            pusd: n.pusd,
            vapurr: n.vapurr,
            loop_vault: n.loop_vault,
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
        self.loop_vault = n.loop_vault;
        self.testnet = n.testnet;
        self.faucet = n.faucet;
    }

    pub fn run(&mut self, cmd: WalletCmd) -> Result<Value, WalletError> {
        self.reload_net();
        if matches!(&cmd, WalletCmd::Send { .. } | WalletCmd::Exec { .. } | WalletCmd::RevealSeed | WalletCmd::ExportKey) {
            crate::require_unlocked()?;
            self.key = DeviceKey::load_result()?.ok_or_else(|| WalletError::Fail("No wallet on this PC".into()))?;
        }
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
            WalletCmd::LockSession => Ok(crate::session::lock_session()),
            WalletCmd::PasscodeUnlock { code } => {
                crate::session::unlock_with_pin(&code)?;
                self.reload_key();
                Ok(self.snap())
            }
            WalletCmd::PasscodeSet { a, b } => {
                crate::session::set_passcode(&a, &b)?;
                self.reload_key();
                Ok(self.snap())
            }
            WalletCmd::SetNet(net) => {
                write_net(&net)?;
                self.reload_net();
                Ok(self.snap())
            }
            WalletCmd::RevealSeed => crate::session::reveal_seed(),
            WalletCmd::ExportKey => crate::session::export_key(),
            WalletCmd::Resolve { to } => resolve_preview(&to),
            WalletCmd::Exec {
                to,
                data,
                value,
                chain_id,
                gas,
            } => self.exec_route(&to, &data, &value, chain_id, gas),
        }
    }

    fn reload_key(&mut self) {
        if let Some(k) = DeviceKey::load() {
            self.key = k;
        }
    }

    fn min_gas(&self) -> u128 {
        if self.testnet {
            MIN_GAS_TEST
        } else {
            MIN_GAS_MAIN
        }
    }

    pub fn snap(&self) -> Value {
        let logged_in = crate::session::is_logged_in();
        let has_key = crate::session::has_key();
        let has_pin = crate::session::has_pin();
        let needs_pin = crate::session::needs_passcode_setup();
        if has_key && DeviceKey::load_result().ok().flatten().is_none() {
            return json!({"ok":false,"has_key":true,"logged_in":false,"assets":[],"address":"","error":"Wallet storage could not be opened. Restore access to the encrypted wallet; no new key was created."});
        }
        if !has_key {
            return json!({
                "ok": true,
                "live": false,
                "logged_in": false,
                "has_key": false,
                "has_pin": false,
                "needs_pin": false,
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
                "pusd_token": "",
                "pusd_supplied": "0",
                "pusd_debt": "0",
                "loop": "",
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
                "signer": "device",
                "has_seed": false,
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
            match token_bal(&self.rpc, &self.usdg, &from) {
                Ok(v) => v,
                Err(e) => {
                    if err.is_empty() {
                        err = e.to_string();
                    }
                    0
                }
            }
        };
        let pusd = if self.pusd.is_empty() {
            0
        } else {
            match token_bal(&self.rpc, &self.pusd, &from) {
                Ok(v) => v,
                Err(e) => {
                    if err.is_empty() {
                        err = e.to_string();
                    }
                    0
                }
            }
        };
        let vapurr = if self.vapurr.is_empty() {
            0
        } else {
            match token_bal(&self.rpc, &self.vapurr, &from) {
                Ok(v) => v,
                Err(e) => {
                    if err.is_empty() {
                        err = e.to_string();
                    }
                    0
                }
            }
        };
        let weth = if self.testnet {
            0
        } else {
            token_bal(&self.rpc, WETH, &from).unwrap_or(0)
        };
        let loop_pos = loop_pos(&self.rpc, &self.loop_vault, self.key.address);
        if !loop_pos.err.is_empty() && err.is_empty() {
            err = loop_pos.err.clone();
        }
        let nonce = self.rpc.eth_nonce(&from).unwrap_or(0);
        let usdg_s = fmt_units(usdg, USDG_DECIMALS);
        let pusd_s = fmt_units(pusd, 18);
        let supplied_s = fmt_units(loop_pos.supplied, 18);
        let debt_s = fmt_units(loop_pos.debt, 18);
        let collat_v_s = fmt_units(loop_pos.collat_v, 18);
        let loop_cash_s = fmt_units(loop_pos.cash, 18);
        let vapurr_s = fmt_units(vapurr, 18);
        let eth_s = fmt_units(eth, 18);
        let weth_s = fmt_units(weth, 18);
        let activity = chain_activity(&from, &self.explorer, self.testnet);
        let mut assets = Vec::new();
        if !self.pusd.is_empty() {
            assets.push(asset(
                "pusd",
                "PUSD",
                "The dollar. Lithe 9%",
                &pusd_s,
                pusd,
                true,
                &self.pusd,
                18,
                false,
            ));
        }
        if loop_pos.supplied > 0 || loop_pos.debt > 0 || loop_pos.collat_v > 0 || !loop_pos.err.is_empty() {
            assets.push(asset(
                "pusd-loop",
                "PUSD",
                &loop_hint(&loop_pos),
                &supplied_s,
                loop_pos.supplied,
                true,
                &self.loop_vault,
                18,
                true,
            ));
        }
        if loop_pos.collat_v > 0 {
            assets.push(asset(
                "oliver-v",
                "VAPURR",
                "Oliver collateral",
                &collat_v_s,
                loop_pos.collat_v,
                false,
                &self.loop_vault,
                18,
                true,
            ));
        }
        if !self.usdg.is_empty() {
            assets.push(asset(
                "usdg",
                "USDG",
                if self.testnet {
                    "Robinhood dollar (test)"
                } else {
                    "Robinhood dollar"
                },
                &usdg_s,
                usdg,
                true,
                &self.usdg,
                USDG_DECIMALS,
                false,
            ));
        }
        if !self.vapurr.is_empty() {
            assets.push(asset(
                "vapurr",
                "VAPURR",
                "Equity. Book burns it to mint $PUSD",
                &vapurr_s,
                vapurr,
                false,
                &self.vapurr,
                18,
                false,
            ));
        }
        assets.push(asset(
            "eth", "ETH", "Gas", &eth_s, eth, false, "", 18, false,
        ));
        if !self.testnet {
            assets.push(asset(
                "weth",
                "WETH",
                "Wrapped ether",
                &weth_s,
                weth,
                false,
                WETH,
                18,
                false,
            ));
        }
        let total = fmt_usd_bag(pusd, loop_pos.supplied, loop_pos.debt, usdg);
        let mut result = json!({
            "ok": err.is_empty(),
            "live": err.is_empty(),
            "logged_in": logged_in,
            "has_key": has_key,
            "has_pin": has_pin,
            "needs_pin": needs_pin,
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
            "pusd_token": self.pusd,
            "pusd_supplied": supplied_s,
            "pusd_debt": debt_s,
            "collat_v": collat_v_s,
            "loop_cash": loop_cash_s,
            "loop_error": loop_pos.err,
            "loop": self.loop_vault,
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
            "signer": "device",
            "has_seed": crate::session::has_seed(),
        });
        let chain = if self.last_tx.is_empty() { self.chain_id } else { self.last_tx_chain };
        match crate::transactions::latest(&self.key.address.to_hex(), chain) {
            Ok(Some(tx)) => {
                let tx = crate::transactions::refresh(tx);
                result["tx"] = json!(tx.hash);
                result["tx_status"] = json!(tx.status);
                result["tx_chain_id"] = json!(tx.chain_id);
                result["tx_url"] = json!(format!("vapurr://scan?q={}", tx.hash));
            }
            Ok(None) => { result["tx_status"] = json!("none"); }
            Err(e) => { result["tx_status"] = json!("unknown"); result["error"] = json!(e.to_string()); }
        }
        result
    }

    pub fn send(&mut self, asset: &str, to: &str, amt: &str) -> Result<Value, WalletError> {
        crate::require_unlocked()?;
        let to = dest_addr(to)?;
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
                if !self.testnet {
                    return Err(WalletError::Fail(
                        "$PUSD spend is testnet 46630 only".into(),
                    ));
                }
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                let token = addr_from_hex(&self.pusd)
                    .ok_or_else(|| WalletError::Fail("no $PUSD on this net".into()))?;
                self.send_token(token, to, value)?
            }
            "vapurr" | "v" => {
                if !self.testnet {
                    return Err(WalletError::Fail(
                        "$VAPURR spend is testnet 46630 only".into(),
                    ));
                }
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                let token = addr_from_hex(&self.vapurr).ok_or_else(|| {
                    WalletError::Fail("no $VAPURR on this net ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â deploy the market".into())
                })?;
                self.send_token(token, to, value)?
            }
            "weth" => {
                let value = parse_units(amt, 18)?;
                if value == 0 {
                    return Err(WalletError::Fail("enter an amount".into()));
                }
                self.send_token(addr_from_hex(WETH).ok_or(WalletError::Rpc)?, to, value)?
            }
            _ => return Err(WalletError::Fail("pick VAPURR, PUSD, USDG, or ETH".into())),
        };
        self.last_tx = hash;
        self.last_tx_chain = self.chain_id;
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

    fn exec_route(
        &mut self,
        to: &str,
        data: &str,
        value: &str,
        chain_id: u64,
        gas: u64,
    ) -> Result<Value, WalletError> {
        let _signing = crate::transactions::signing_guard()?;
        let rpc_url = rhc::rpc_http(chain_id).ok_or_else(|| {
            WalletError::Fail("unsupported chain".into())
        })?;
        let to_addr = addr_from_hex(to).ok_or_else(|| WalletError::Fail("bad to".into()))?;
        let data_b = decode_hex_bytes(data).map_err(|_| WalletError::Fail("Invalid transaction data".into()))?;
        if data_b.is_empty() && value.trim().is_empty() {
            return Err(WalletError::Fail("empty route tx".into()));
        }
        let value_n = crate::parse_hex_u128(value)?;
        let rpc = Rpc::at(rpc_url);
        let from = self.key.address.to_hex();
        let eth = rpc.eth_balance(&from).map_err(rpc_err)?;
        let floor = if chain_id == rhc::TESTNET_CHAIN_ID {
            MIN_GAS_TEST
        } else {
            MIN_GAS_MAIN
        };
        if eth < value_n.saturating_add(floor / 4) {
            return Err(WalletError::Fail("not enough ETH for gas".into()));
        }
        crate::transactions::ensure_no_pending(&from, chain_id)?;
        let nonce = rpc.eth_nonce(&from).map_err(rpc_err)?;
        let gas_price = rpc.eth_gas_price().unwrap_or(100_000_000);
        let to_hex = to_addr.to_hex();
        let data_hex = hex0x(&data_b);
        let _ = gas;
        let est = rpc.eth_estimate_gas_value(&from, Some(&to_hex), &data_hex, value_n).map_err(rpc_err)?;
        let gas_use = est.saturating_mul(13) / 10;
        let tx = Tx {
            chain_id,
            nonce,
            max_priority_fee: 1_000_000,
            max_fee: gas_price.saturating_mul(3).max(1_000_000),
            gas: gas_use.max(80_000),
            to: Some(to_addr),
            value: value_n,
            data: data_b,
        };
        let raw = self.key.sign_tx(&tx)?;
        let hash = rpc.eth_send_raw(&hex0x(&raw)).map_err(rpc_err)?;
        crate::transactions::record(&hash, chain_id, &from, "pending")?;
        for _ in 0..80 {
            match rpc.eth_receipt(&hash) {
                Ok(Some(r)) => {
                    crate::transactions::record(&hash, chain_id, &from, crate::transactions::receipt_status(Some(&r)))?;
                    if r.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
                        return Err(WalletError::Fail("tx reverted".into()));
                    }
                    break;
                }
                _ => std::thread::sleep(std::time::Duration::from_millis(400)),
            }
        }
        self.last_tx = hash.clone();
        self.last_tx_chain = chain_id;
        let mut snap = self.snap();
        snap["tx"] = json!(hash);
        snap["tx_url"] = json!(if chain_id == rhc::CHAIN_ID || chain_id == rhc::TESTNET_CHAIN_ID
        {
            format!("vapurr://scan?q={hash}")
        } else {
            String::new()
        });
        snap["ok"] = json!(true);
        Ok(snap)
    }

    fn broadcast(
        &self,
        to: Option<Address>,
        value: u128,
        data: &[u8],
    ) -> Result<String, WalletError> {
        let _signing = crate::transactions::signing_guard()?;
        let from = self.key.address.to_hex();
        crate::transactions::ensure_no_pending(&from, self.chain_id)?;
        let nonce = self.rpc.eth_nonce(&from).map_err(rpc_err)?;
        let gas_price = self.rpc.eth_gas_price().unwrap_or(100_000_000);
        let to_hex = to.map(|a| a.to_hex());
        let data_hex = hex0x(data);
        let est = self
            .rpc
            .eth_estimate_gas_value(&from, to_hex.as_deref(), &data_hex, value)
            .unwrap_or(if data.is_empty() {
                ORBIT_GAS_FLOOR
            } else {
                200_000
            });
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
        crate::transactions::record(&hash, self.chain_id, &from, "pending")?;
        for _ in 0..80 {
            match self.rpc.eth_receipt(&hash) {
                Ok(Some(r)) => {
                    crate::transactions::record(&hash, self.chain_id, &from, crate::transactions::receipt_status(Some(&r)))?;
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

fn resolve_preview(raw: &str) -> Result<Value, WalletError> {
    let t = raw.trim();
    if t.is_empty() {
        return Err(WalletError::Fail(
            "paste a 0x address or a .hood name".into(),
        ));
    }
    let addr = dest_addr(t)?;
    let name = if looks_like_hood(t) {
        let n = t.trim().trim_start_matches('@').to_ascii_lowercase();
        if n.ends_with(".hood") {
            n
        } else {
            format!("{n}.hood")
        }
    } else {
        String::new()
    };
    Ok(json!({
        "ok": true,
        "resolved": addr.to_checksum(),
        "name": name,
        "address": crate::session::peek_address().unwrap_or_default(),
    }))
}

fn looks_like_hood(raw: &str) -> bool {
    let n = raw.trim().trim_start_matches('@').to_ascii_lowercase();
    n.ends_with(".hood")
        || (n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && n.len() >= 2
            && !n.starts_with("0x"))
}

fn dest_addr(raw: &str) -> Result<Address, WalletError> {
    let t = raw.trim().trim_start_matches('@');
    if let Some(a) = addr_from_hex(t) {
        return Ok(a);
    }
    let n = t.to_ascii_lowercase();
    if n.ends_with(".hood")
        || (n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && n.len() >= 2)
    {
        return resolve_pns(&n);
    }
    Err(WalletError::Fail(
        "paste a 0x address or a .hood name".into(),
    ))
}

fn resolve_pns(name: &str) -> Result<Address, WalletError> {
    let name = if name.ends_with(".hood") {
        name.to_string()
    } else {
        format!("{name}.hood")
    };
    let rpc = Rpc::at(rhc::TESTNET_RPC_HTTP);
    let data = encode_fn_str("resolveName(string)", &name);
    let raw = rpc
        .eth_call(
            "0x0000000000000000000000000000000000000000",
            Some(rhc::TESTNET_PNS),
            &hex0x(&data),
        )
        .map_err(rpc_err)?;
    let bytes = decode_hex_bytes(&raw).map_err(|_| WalletError::Rpc)?;
    let addr = decode_word_addr(&bytes, 1)
        .ok_or_else(|| WalletError::Fail(format!("{name} is not on PNS")))?;
    if addr.0.iter().all(|&b| b == 0) {
        return Err(WalletError::Fail(format!("{name} is not on PNS")));
    }
    Ok(addr)
}

fn chain_activity(addr: &str, explorer: &str, testnet: bool) -> Vec<Value> {
    if let Some(rows) = explorer_activity(addr, explorer) {
        return rows;
    }
    if testnet {
        return Vec::new();
    }
    let raw = vapurr_rhc::scan::api("addr", &format!("a={addr}"));
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let rows = v
        .pointer("/addr/transfers")
        .or_else(|| v.get("token_transfers"))
        .or_else(|| v.get("transfers"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| map_scan_row(&row, addr, explorer))
        .take(12)
        .collect()
}

fn explorer_activity(addr: &str, explorer: &str) -> Option<Vec<Value>> {
    let base = explorer.trim_end_matches('/');
    let url = format!("{base}/api/v2/addresses/{addr}/token-transfers");
    let http = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .user_agent("vapurr/0.1")
        .build()
        .ok()?;
    let v: Value = http.get(&url).send().ok()?.json().ok()?;
    let items = v.get("items").and_then(|x| x.as_array())?;
    let rows: Vec<Value> = items
        .iter()
        .filter_map(|row| map_blockscout_xfer(row, addr, explorer))
        .take(12)
        .collect();
    Some(rows)
}

fn hash_field(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(obj) if obj.is_object() => obj
            .get("hash")
            .or_else(|| obj.get("address_hash"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn map_blockscout_xfer(row: &Value, me: &str, explorer: &str) -> Option<Value> {
    let from = hash_field(row.get("from"));
    let to = hash_field(row.get("to"));
    let tx = row
        .get("transaction_hash")
        .or_else(|| row.get("tx_hash"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if tx.is_empty() {
        return None;
    }
    let token = row.get("token");
    let symbol = token
        .and_then(|t| t.get("symbol"))
        .and_then(|x| x.as_str())
        .unwrap_or("token")
        .to_string();
    let dec = token
        .and_then(|t| t.get("decimals"))
        .and_then(|x| match x {
            Value::String(s) => s.parse::<u8>().ok(),
            Value::Number(n) => n.as_u64().map(|v| v as u8),
            _ => None,
        })
        .unwrap_or(18)
        .min(18);
    let raw_amt = row
        .get("total")
        .and_then(|t| t.get("value"))
        .and_then(|x| x.as_str())
        .unwrap_or("0");
    let amt_u = raw_amt.parse::<u128>().unwrap_or(0);
    let ty = row.get("type").and_then(|x| x.as_str()).unwrap_or("");
    let method = row.get("method").and_then(|x| x.as_str()).unwrap_or("");
    let me_l = me.to_ascii_lowercase();
    let to_l = to.to_ascii_lowercase();
    let inbound = to_l == me_l;
    let kind = if ty.contains("mint") || method == "mint" {
        "mint"
    } else if ty.contains("burn") || method == "redeem" {
        "burn"
    } else if method == "stake" {
        "stake"
    } else if method == "unstake" {
        "unstake"
    } else if inbound {
        "in"
    } else {
        "out"
    };
    let peer = if inbound { from.clone() } else { to.clone() };
    Some(json!({
        "tx": tx,
        "url": format!("{}/tx/{}", explorer.trim_end_matches('/'), tx),
        "from": from,
        "to": to,
        "peer": peer,
        "amount": fmt_units(amt_u, dec),
        "symbol": symbol,
        "dir": if inbound { "in" } else { "out" },
        "kind": kind,
        "ts": row.get("timestamp").cloned().unwrap_or(json!("")),
    }))
}

fn map_scan_row(row: &Value, me: &str, explorer: &str) -> Option<Value> {
    let from = hash_field(row.get("from"));
    let to = hash_field(row.get("to"));
    let tx = row
        .get("tx")
        .or_else(|| row.get("hash"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if tx.is_empty() {
        return None;
    }
    let inbound = to.to_ascii_lowercase() == me.to_ascii_lowercase();
    let peer = if inbound { from.clone() } else { to.clone() };
    Some(json!({
        "tx": tx,
        "url": format!("{}/tx/{}", explorer.trim_end_matches('/'), tx),
        "from": from,
        "to": to,
        "peer": peer,
        "amount": row.get("amount").cloned().unwrap_or(json!("")),
        "symbol": row.get("symbol").cloned().unwrap_or(json!("")),
        "dir": if inbound { "in" } else { "out" },
        "kind": if inbound { "in" } else { "out" },
    }))
}

fn token_bal(rpc: &Rpc, token: &str, holder: &str) -> Result<u128, WalletError> {
    let holder_addr = addr_from_hex(holder).ok_or(WalletError::Rpc)?;
    let data = hex0x(&crate::balance_of_calldata(holder_addr));
    let raw = rpc.eth_call(holder, Some(token), &data).map_err(rpc_err)?;
    let bytes = decode_hex_bytes(&raw).map_err(|_| WalletError::Rpc)?;
    Ok(decode_word_u128(&bytes, 0).unwrap_or(0))
}

struct LoopPos {
    supplied: u128,
    debt: u128,
    collat_v: u128,
    cash: u128,
    room: u128,
    apy_bps: u128,
    err: String,
}

/// `PusdLoop.snapshot` ABI: cash@0 room@19; supplied@9 collat_v@10 debt@11; supply APY bps@5.
fn decode_loop_pos(bytes: &[u8]) -> Result<LoopPos, String> {
    if bytes.len() < 20 * 32 {
        return Err("vault snapshot decode".into());
    }
    let cash = decode_word_u128(bytes, 0).unwrap_or(0);
    let room = decode_word_u128(bytes, 19).unwrap_or(0);
    Ok(LoopPos {
        supplied: decode_word_u128(bytes, 9).unwrap_or(0),
        debt: decode_word_u128(bytes, 11).unwrap_or(0),
        collat_v: decode_word_u128(bytes, 10).unwrap_or(0),
        cash,
        room: room.min(cash),
        apy_bps: decode_word_u128(bytes, 5).unwrap_or(0),
        err: String::new(),
    })
}

fn loop_pos(rpc: &Rpc, vault: &str, holder: Address) -> LoopPos {
    let empty = |err: String| LoopPos {
        supplied: 0,
        debt: 0,
        collat_v: 0,
        cash: 0,
        room: 0,
        apy_bps: 0,
        err,
    };
    if vault.is_empty() {
        return empty(String::new());
    }
    let data = hex0x(&encode_fn_addr("snapshot(address)", holder));
    let raw = match rpc.eth_call(&holder.to_hex(), Some(vault), &data) {
        Ok(s) => s,
        Err(e) => return empty(format!("Oliver RPC: {e}")),
    };
    let bytes = match decode_hex_bytes(&raw) {
        Ok(b) => b,
        Err(_) => return empty("Oliver decode".into()),
    };
    match decode_loop_pos(&bytes) {
        Ok(p) => p,
        Err(e) => empty(e),
    }
}

fn loop_hint(p: &LoopPos) -> String {
    if !p.err.is_empty() {
        return format!("Oliver error | {}", p.err);
    }
    let mut h = "Oliver".to_string();
    if p.collat_v > 0 {
        h.push_str(" | ");
        h.push_str(&fmt_units(p.collat_v, 18));
        h.push_str(" V collat");
    }
    if p.supplied > 0 {
        h.push_str(" | supplied");
        if p.apy_bps > 0 {
            h.push(' ');
            h.push_str(&fmt_apy_bps(p.apy_bps));
            h.push_str(" APY");
        }
    } else if p.apy_bps > 0 {
        h.push_str(" | ");
        h.push_str(&fmt_apy_bps(p.apy_bps));
        h.push_str(" APY");
    }
    if p.debt > 0 {
        h.push_str(" | ");
        h.push_str(&fmt_units(p.debt, 18));
        h.push_str(" debt | prefer unwind");
    }
    if p.debt == 0 && (p.collat_v > 0 || p.supplied > 0) {
        h.push_str(" | withdraw needs cash");
        if p.supplied > p.room && p.room > 0 {
            h.push_str(" | room ");
            h.push_str(&fmt_units(p.room, 18));
        }
    }
    h
}

fn fmt_apy_bps(bps: u128) -> String {
    let whole = bps / 100;
    let frac = bps % 100;
    if frac == 0 {
        format!("{whole}%")
    } else {
        format!("{whole}.{frac:02}%")
    }
}

fn asset(
    id: &str,
    symbol: &str,
    hint: &str,
    amount: &str,
    raw: u128,
    dollar: bool,
    token: &str,
    decimals: u8,
    locked: bool,
) -> Value {
    json!({
        "id": id,
        "symbol": symbol,
        "hint": hint,
        "amount": amount,
        "raw": raw.to_string(),
        "zero": raw == 0,
        "dollar": dollar,
        "token": token,
        "decimals": decimals,
        "locked": locked,
        "spendable": !locked,
    })
}

fn fmt_usd_bag(pusd: u128, supplied: u128, debt: u128, usdg: u128) -> String {
    let p = (pusd as f64 + supplied as f64 - debt as f64) / 1e18;
    let u = usdg as f64 / 1e6;
    let t = p + u;
    if !t.is_finite() {
        return "0.00".into();
    }
    if t <= 0.0 {
        return "0.00".into();
    }
    format!("{t:.2}")
}

fn rpc_err(e: impl std::fmt::Display) -> WalletError {
    WalletError::Fail(e.to_string())
}

pub(crate) fn parse_hex_u128(s: &str) -> u128 {
    let t = s.trim();
    if t.is_empty() || t == "0x" || t == "0x0" || t == "0" {
        return 0;
    }
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u128::from_str_radix(h, 16).unwrap_or(0)
    } else {
        t.parse().unwrap_or(0)
    }
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
    use super::{fmt_units, fmt_usd_bag, map_blockscout_xfer, normalize_net, parse_units};
    use serde_json::json;

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
        assert_eq!(super::parse_hex_u128("0x0"), 0);
        assert_eq!(super::parse_hex_u128("0x10"), 16);
        assert_eq!(super::parse_hex_u128("100"), 100);
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

    #[test]
    fn net_normalizes() {
        assert_eq!(normalize_net("mainnet"), "mainnet");
        assert_eq!(normalize_net("MAINNET"), "mainnet");
        assert_eq!(normalize_net("testnet"), "testnet");
        assert_eq!(normalize_net("whatever"), "testnet");
    }

    #[test]
    fn blockscout_xfer_maps_mint() {
        let row = json!({
            "transaction_hash": "0xabc",
            "from": { "hash": "0x0000000000000000000000000000000000000000" },
            "to": { "hash": "0xc9371911D6b5a6e36306334Ab56D27Cb35E669c9" },
            "token": { "symbol": "PUSD", "decimals": "18", "address_hash": "0x59bb" },
            "total": { "value": "1500000000000000000" },
            "type": "token_minting",
            "method": "mint",
            "timestamp": "2026-09-03T04:56:50.000000Z"
        });
        let v = map_blockscout_xfer(
            &row,
            "0xc9371911d6b5a6e36306334ab56d27cb35e669c9",
            "https://explorer.testnet.chain.robinhood.com",
        )
        .unwrap();
        assert_eq!(v.get("kind").and_then(|x| x.as_str()), Some("mint"));
        assert_eq!(v.get("dir").and_then(|x| x.as_str()), Some("in"));
        assert_eq!(v.get("symbol").and_then(|x| x.as_str()), Some("PUSD"));
        assert_eq!(v.get("amount").and_then(|x| x.as_str()), Some("1.5"));
        assert!(v
            .get("url")
            .and_then(|x| x.as_str())
            .unwrap()
            .contains("/tx/0xabc"));
    }

    #[test]
    fn dest_hex_ok_hood_needs_rpc() {
        let a = super::dest_addr("0xc9371911d6b5a6e36306334ab56d27cb35e669c9").unwrap();
        assert_eq!(hex::encode(a.0), "c9371911d6b5a6e36306334ab56d27cb35e669c9");
        let e = super::dest_addr("not a name!!").unwrap_err().to_string();
        assert!(e.contains("0x") || e.contains("hood"));
    }

    #[test]
    fn usd_bag_sums_dollars() {
        assert_eq!(fmt_usd_bag(0, 0, 0, 0), "0.00");
        assert_eq!(fmt_usd_bag(2 * 10u128.pow(18), 0, 0, 1_500_000), "3.50");
        assert_eq!(
            fmt_usd_bag(10u128.pow(18), 2 * 10u128.pow(18), 5 * 10u128.pow(17), 0),
            "2.50"
        );
    }

    #[test]
    fn loop_snap_reads_supplied_and_debt() {
        let mut bytes = vec![0u8; 20 * 32];
        let supplied = 7 * 10u128.pow(18);
        let debt = 3 * 10u128.pow(18);
        let collat = 485_846 * 10u128.pow(18);
        let cash = 10u128.pow(15); // 0.001
        let apy = 412u128;
        bytes[0 * 32 + 16..0 * 32 + 32].copy_from_slice(&cash.to_be_bytes());
        bytes[5 * 32 + 16..5 * 32 + 32].copy_from_slice(&apy.to_be_bytes());
        bytes[9 * 32 + 16..9 * 32 + 32].copy_from_slice(&supplied.to_be_bytes());
        bytes[10 * 32 + 16..10 * 32 + 32].copy_from_slice(&collat.to_be_bytes());
        bytes[11 * 32 + 16..11 * 32 + 32].copy_from_slice(&debt.to_be_bytes());
        bytes[19 * 32 + 16..19 * 32 + 32].copy_from_slice(&(5 * 10u128.pow(18)).to_be_bytes());
        let p = super::decode_loop_pos(&bytes).unwrap();
        assert_eq!(p.supplied, supplied);
        assert_eq!(p.debt, debt);
        assert_eq!(p.collat_v, collat);
        assert_eq!(p.cash, cash);
        assert_eq!(p.room, cash); // room capped to cash
        assert_eq!(p.apy_bps, apy);
        let h = super::loop_hint(&p);
        assert!(h.contains("Oliver"), "{h}");
        assert!(h.contains("V collat"), "{h}");
        assert!(h.contains("4.12% APY"), "{h}");
        assert!(h.contains("3 debt"), "{h}");
        assert!(h.contains("prefer unwind"), "{h}");
        assert!(super::decode_loop_pos(&[]).is_err());
    }

    #[test]
    fn resolve_hex_preview() {
        let v = super::resolve_preview("0xc9371911d6b5a6e36306334ab56d27cb35e669c9").unwrap();
        assert_eq!(
            v.get("resolved")
                .and_then(|x| x.as_str())
                .unwrap()
                .to_ascii_lowercase(),
            "0xc9371911d6b5a6e36306334ab56d27cb35e669c9"
        );
        assert_eq!(v.get("name").and_then(|x| x.as_str()), Some(""));
    }
}
