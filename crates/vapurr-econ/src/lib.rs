#![recursion_limit = "256"]
//! $VAPURR / $PUSD. Burn V mint P, burn P mint V. Lithe is 9% on $PUSD.

pub mod cfg;
pub mod euler;
pub mod house;
pub mod kelly;
pub mod swap;
pub mod ketlist;
pub mod outbid;
pub mod treasury;

pub(crate) use cfg::{MarketCfg, GEN};

use serde_json::{json, Value};

use vapurr_rhc::{self as rhc, Rpc, USDG};
use vapurr_wallet::tx::{
    decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn, encode_fn_addr,
    encode_fn_addr_addr, encode_fn_addr_u256, encode_fn_u256, hex0x,
    revert_reason, Tx,
};
use vapurr_wallet::{addr_from_hex, Address, DeviceKey};

pub const DEC: u128 = 1_000_000_000_000_000_000;
const MARKET_HEX: &str = include_str!("market.hex");
const MOCK_USDG_HEX: &str = include_str!("mock_usdg.hex");

#[derive(Debug, thiserror::Error)]
pub enum EconError {
    #[error("need more $VAPURR")]
    NeedVapurr,
    #[error("need more $PUSD")]
    NeedPusd,
    #[error("amount too small")]
    Tiny,
    #[error("peg paused â€” book too thin")]
    Thin,
    #[error("{0}")]
    Rpc(String),
    #[error("need ETH for gas â€” faucet.testnet.chain.robinhood.com")]
    NeedGas,
    #[error("need USDG")]
    NeedUsdg,
    #[error("market is not on chain yet")]
    NotLive,
    #[error("open the board")]
    NeedBoard,
    #[error("need a URL or @handle")]
    BadUrl,
    #[error("need a token address")]
    BadToken,
    #[error("need a pool address")]
    BadPool,
    #[error("need a ticker and a name")]
    BadTicker,
    #[error("already listed")]
    Owned,
    #[error("need more $PUSD to take #1")]
    Top,
    #[error("board is full")]
    Full,
    #[error("vault is not on chain yet")]
    NeedLoop,
    #[error("house book is not on chain yet")]
    NeedHouse,
    #[error("house swap is not on chain yet")]
    NeedSwap,
    #[error("tx pending {0}")]
    Pending(String),
}

#[derive(Debug, Clone)]
pub struct EconFail {
    pub which: String,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub enum EconCmd {
    Snap,
    Mint(String),
    Redeem(String),
    Deploy,
    Seed { usdg: String, vapurr: String },
    Outbid,
    OutbidBid {
        url: String,
        title: String,
        amt: String,
    },
    OutbidDeploy,
    KetList,
    KetListPay {
        token: String,
        pool: String,
        symbol: String,
        name: String,
        amt: String,
        meta: String,
    },
    KetListDeploy,
    LoopDeploy,
    LoopReplace,
    LoopOp {
        op: String,
        amt: String,
        steps: String,
    },
    HouseDeploy,
    HouseSeed {
        vapurr: String,
        pusd: String,
    },
    HouseBootstrap,
    SwapDeploy,
    SwapReplace,
    HouseSwap {
        sell_v: bool,
        amt: String,
    },
    Pulse,
}

pub struct Client {
    pub(crate) rpc: Rpc,
    pub(crate) key: DeviceKey,
    pub(crate) cfg: MarketCfg,
    pub(crate) last_tx: String,
    pub(crate) chain_id: u64,
    pub(crate) explorer: String,
}

impl Client {
    pub fn open() -> Self {
        let cfg = MarketCfg::load();
        let testnet = cfg.net != "mainnet";
        let (rpc, chain_id, explorer) = if testnet {
            (
                Rpc::at(rhc::TESTNET_RPC_HTTP),
                rhc::TESTNET_CHAIN_ID,
                rhc::TESTNET_EXPLORER.to_string(),
            )
        } else {
            (Rpc::new(), rhc::CHAIN_ID, rhc::EXPLORER.to_string())
        };
        Self {
            rpc,
            key: DeviceKey::load().unwrap_or_else(DeviceKey::generate),
            cfg,
            last_tx: String::new(),
            chain_id,
            explorer,
        }
    }

    pub(crate) fn explorer(&self) -> &str {
        &self.explorer
    }

    fn is_testnet(&self) -> bool {
        self.chain_id == rhc::TESTNET_CHAIN_ID
    }

    pub fn run(&mut self, cmd: EconCmd) -> Result<Value, EconFail> {
        if let Some(k) = DeviceKey::load() {
            self.key = k;
        }
        self.cfg = MarketCfg::load();
        let which = match &cmd {
            EconCmd::Snap => "snap",
            EconCmd::Mint(_) => "mint",
            EconCmd::Redeem(_) => "redeem",
            EconCmd::Deploy => "deploy",
            EconCmd::Seed { .. } => "seed",
            EconCmd::Outbid | EconCmd::OutbidBid { .. } => "outbid",
            EconCmd::OutbidDeploy => "deploy",
            EconCmd::KetList | EconCmd::KetListPay { .. } => "ketlist",
            EconCmd::KetListDeploy => "deploy",
            EconCmd::LoopDeploy | EconCmd::LoopReplace | EconCmd::LoopOp { .. } => "loop",
            EconCmd::HouseDeploy
            | EconCmd::HouseSeed { .. }
            | EconCmd::HouseBootstrap
            | EconCmd::HouseSwap { .. } => "house",
            EconCmd::SwapDeploy | EconCmd::SwapReplace | EconCmd::Pulse => "pulse",
        };
        match self.run_inner(cmd) {
            Ok(v) => Ok(v),
            Err(e) => Err(EconFail {
                which: which.into(),
                msg: e.to_string(),
            }),
        }
    }

    fn run_inner(&mut self, cmd: EconCmd) -> Result<Value, EconError> {
        match cmd {
            EconCmd::Snap => Ok(self.snapshot()),
            EconCmd::Mint(s) => {
                let n = parse_amt(&s)?;
                self.transact("swapVToPusd(uint256)", n)?;
                Ok(self.snapshot())
            }
            EconCmd::Redeem(s) => {
                let n = parse_amt(&s)?;
                self.transact("swapPusdToV(uint256)", n)?;
                Ok(self.snapshot())
            }
            EconCmd::Deploy => {
                self.deploy()?;
                Ok(self.snapshot())
            }
            EconCmd::Seed { .. } => {
                Err(EconError::Rpc(
                    "Burn $VAPURR to mint $PUSD. No USDG seed.".into(),
                ))
            }
            EconCmd::Outbid => Ok(self.outbid_snap()),
            EconCmd::OutbidBid { url, title, amt } => self.outbid_bid(&url, &title, &amt),
            EconCmd::OutbidDeploy => {
                self.outbid_deploy()?;
                Ok(self.outbid_snap())
            }
            EconCmd::KetList => Ok(self.ketlist_snap()),
            EconCmd::KetListPay {
                token,
                pool,
                symbol,
                name,
                amt,
                meta,
            } => self.ketlist_pay(&token, &pool, &symbol, &name, &amt, &meta),
            EconCmd::KetListDeploy => {
                self.ketlist_deploy()?;
                Ok(self.ketlist_snap())
            }
            EconCmd::LoopDeploy => {
                self.euler_deploy()?;
                Ok(self.snapshot())
            }
            EconCmd::LoopReplace => {
                self.cfg.loop_vault.clear();
                self.euler_deploy()?;
                Ok(self.book_snap())
            }
            EconCmd::LoopOp { op, amt, steps } => self.euler_op(&op, &amt, &steps),
            EconCmd::HouseDeploy => {
                self.house_deploy()?;
                Ok(self.snapshot())
            }
            EconCmd::HouseSeed { vapurr, pusd } => self.house_seed_cmd(&vapurr, &pusd),
            EconCmd::HouseBootstrap => self.house_bootstrap(),
            EconCmd::SwapDeploy => {
                self.swap_deploy()?;
                Ok(self.snapshot())
            }
            EconCmd::SwapReplace => {
                self.cfg.swap.clear();
                self.swap_deploy()?;
                Ok(self.book_snap())
            }
            EconCmd::HouseSwap { sell_v, amt } => {
                let n = parse_amt(&amt)?;
                self.house_swap(sell_v, n)?;
                Ok(self.snapshot())
            }
            EconCmd::Pulse => self.pulse(),
        }
    }

    pub fn snapshot(&self) -> Value {
        let mut v = self.book_snap();
        v["treasury"] = crate::treasury::snap();
        v
    }

    /// Market + vault + house. No treasury, no mainnet liq crawl.
    pub(crate) fn book_snap(&self) -> Value {
        let mut v = match self.snapshot_inner() {
            Ok(v) => v,
            Err(e) => self.base_snap(&e.to_string()),
        };
        v["loop"] = self.euler_snap();
        v["house"] = self.house_snap();
        v
    }

    fn snapshot_inner(&self) -> Result<Value, EconError> {
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).unwrap_or(0);
        let market = self.live_market();
        if market.is_none() {
            let mut v = self.base_snap("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_deploy"] = json!(true);
            v["status"] = json!(if eth < MIN_GAS_WEI {
                "send testnet ETH, then deploy"
            } else {
                "ready"
            });
            return Ok(v);
        }
        let m = market.unwrap();
        let data = encode_fn_addr("snapshot(address)", self.key.address);
        let raw = self
            .rpc
            .eth_call(&from, Some(&m.to_hex()), &hex0x(&data))
            .map_err(econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("call".into()))?;
        let s = decode_snap(&bytes)?;
        Ok(json!({
            "live": true,
            "need_deploy": false,
            "need_eth": eth < MIN_GAS_WEI,
            "address": self.key.address.to_checksum(),
            "market": m.to_checksum(),
            "vapurr_token": s.vapurr_token,
            "pusd_token": s.pusd_token,
            "explorer": format!("{}/address/{}", self.explorer, m.to_hex()),
            "tx": self.last_tx,
            "tx_url": if self.last_tx.is_empty() { String::new() } else { format!("{}/tx/{}", self.explorer, self.last_tx) },
            "chain_id": self.chain_id,
            "net": if self.is_testnet() { "testnet" } else { "mainnet" },
            "faucet": if self.is_testnet() { rhc::TESTNET_FAUCET } else { "" },
            "eth": fmt_eth(eth),
            "status": "live",
            "vapurr": fmt_tok(s.vapurr),
            "pusd": fmt_tok(s.pusd),
            "price": fmt_price(s.px),
            "peg": "1.0000",
            "apy": fmt_bps(s.apy_bps),
            "pusd_supply": fmt_tok(s.pusd_supply),
            "vapurr_supply": fmt_tok(s.vapurr_supply),
            "index": fmt_index(s.index),
            "pool": fmt_tok(s.pool18),
            "yield_reserve": fmt_tok(s.yield_res),
            "min_spread": fmt_spread(s.min_spread),
            "seeded": true,
        }))
    }

    fn base_snap(&self, err: &str) -> Value {
        json!({
            "live": false,
            "need_deploy": true,
            "need_eth": true,
            "address": self.key.address.to_checksum(),
            "market": self.cfg.market,
            "vapurr_token": self.cfg.vapurr,
            "pusd_token": self.cfg.pusd,
            "explorer": "",
            "tx": self.last_tx,
            "tx_url": "",
            "chain_id": self.chain_id,
            "net": if self.is_testnet() { "testnet" } else { "mainnet" },
            "faucet": if self.is_testnet() { rhc::TESTNET_FAUCET } else { "" },
            "eth": "0.000000",
            "status": if err.is_empty() { "not on chain" } else { err },
            "error": err,
            "vapurr": "0.00",
            "pusd": "0.00",
            "price": "0.0000",
            "peg": "1.0000",
            "apy": "0.00",
            "pusd_supply": "0.00",
            "vapurr_supply": "0.00",
            "index": "1.000000",
            "pool": "0.00",
            "yield_reserve": "0.00",
            "min_spread": "2.00",
            "seeded": false,
        })
    }

    pub(crate) fn live_market(&self) -> Option<Address> {
        self.live_ca(&self.cfg.market)
    }

    /// Code at `hex`. RPC errors keep a known CA (do not re-deploy).
    pub(crate) fn live_ca(&self, hex: &str) -> Option<Address> {
        if hex.is_empty() {
            return None;
        }
        let addr = addr_from_hex(hex)?;
        match self.rpc.eth_code(&addr.to_hex()) {
            Ok(code) => {
                let h = code.trim().trim_start_matches("0x").trim();
                if h.len() <= 2 {
                    None
                } else {
                    Some(addr)
                }
            }
            Err(_) => Some(addr),
        }
    }

    fn transact(&mut self, sig: &str, amt: u128) -> Result<String, EconError> {
        if amt == 0 {
            return Err(EconError::Tiny);
        }
        let market = self.live_market().ok_or(EconError::NotLive)?;
        let data = encode_fn_u256(sig, amt);
        self.send(Some(market), &data)
    }

    fn usdg_addr(&self) -> Result<Address, EconError> {
        if !self.cfg.usdg.is_empty() {
            return addr_from_hex(&self.cfg.usdg).ok_or_else(|| EconError::Rpc("usdg".into()));
        }
        let s = if self.is_testnet() {
            rhc::TESTNET_MOCK_USDG
        } else {
            USDG
        };
        addr_from_hex(s).ok_or_else(|| EconError::Rpc("usdg".into()))
    }

    fn ensure_mock_usdg(&mut self) -> Result<Address, EconError> {
        if !self.cfg.usdg.is_empty() {
            if let Ok(a) = self.usdg_addr() {
                let code = self.rpc.eth_code(&a.to_hex()).unwrap_or_default();
                let hex = code.trim().trim_start_matches("0x").trim();
                if hex.len() > 2 {
                    return Ok(a);
                }
            }
        }
        let hash = self.send(None, &mock_usdg_bytecode()?)?;
        let receipt = self.wait(&hash)?;
        if receipt.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
            return Err(EconError::Rpc("usdg deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no usdg ca".into()))?;
        let usdg = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad usdg ca".into()))?;
        self.cfg.usdg = usdg.to_checksum();
        self.cfg.net = "testnet".into();
        self.cfg.save();
        let amt: u128 = 1_000_000 * 1_000_000;
        let mint = encode_fn_addr_u256("mint(address,uint256)", self.key.address, amt);
        self.send(Some(usdg), &mint)?;
        Ok(usdg)
    }

    pub(crate) fn token_raw(&self, token: Address, holder: Address) -> u128 {
        let data = encode_fn_addr("balanceOf(address)", holder);
        let raw = self
            .rpc
            .eth_call(&holder.to_hex(), Some(&token.to_hex()), &hex0x(&data))
            .ok();
        let Some(raw) = raw else { return 0 };
        let bytes = decode_hex_bytes(&raw).unwrap_or_default();
        decode_word_u128(&bytes, 0).unwrap_or(0)
    }

    fn mock_owner(&self, usdg: Address) -> Option<Address> {
        let data = encode_fn("owner()");
        let raw = self
            .rpc
            .eth_call(&self.key.address.to_hex(), Some(&usdg.to_hex()), &hex0x(&data))
            .ok()?;
        let bytes = decode_hex_bytes(&raw).ok()?;
        decode_word_addr(&bytes, 0)
    }

    fn ensure_usdg(&mut self, need_6: u128) -> Result<(), EconError> {
        let usdg = self.usdg_addr()?;
        let market = self.live_market().ok_or(EconError::NotLive)?;
        let from = self.key.address;
        let bal = self.token_raw(usdg, from);
        if bal < need_6 {
            if self.is_testnet() {
                if let Some(owner) = self.mock_owner(usdg) {
                    if owner.0 == from.0 {
                        let amt = need_6.max(1_000_000 * 1_000_000);
                        let mint = encode_fn_addr_u256("mint(address,uint256)", from, amt);
                        self.send(Some(usdg), &mint)?;
                    } else {
                        return Err(EconError::Rpc(format!(
                            "need $USDG. Test mock owner is {}. This device is not that key.",
                            owner.to_checksum()
                        )));
                    }
                } else {
                    return Err(EconError::NeedUsdg);
                }
            } else {
                return Err(EconError::NeedUsdg);
            }
        }
        let data = encode_fn_addr_addr("allowance(address,address)", from, market);
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&usdg.to_hex()), &hex0x(&data))
            .map_err(econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).unwrap_or_default();
        let allow = decode_word_u128(&bytes, 0).unwrap_or(0);
        if allow >= need_6 && need_6 > 0 {
            return Ok(());
        }
        let approve = encode_fn_addr_u256("approve(address,uint256)", market, u128::MAX);
        self.send(Some(usdg), &approve)?;
        Ok(())
    }

    fn deploy(&mut self) -> Result<String, EconError> {
        if self.live_market().is_some() {
            return Ok(self.cfg.market.clone());
        }
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(econ_rpc)?;
        if eth < MIN_GAS_WEI {
            return Err(EconError::NeedGas);
        }
        let mut bytecode = market_bytecode()?;
        // constructor(uint256 vapurrRate_) â€” $1 per V at genesis, first-spot oracle
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_u256(DEC));
        let hash = self.send(None, &bytecode)?;
        let receipt = self.wait(&hash)?;
        if receipt.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
            return Err(EconError::Rpc("deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no contractAddress".into()))?;
        let market = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad ca".into()))?;
        self.cfg.gen = GEN;
        self.cfg.market = market.to_checksum();
        self.cfg.net = if self.is_testnet() {
            "testnet".into()
        } else {
            "mainnet".into()
        };
        if let Ok(raw) = self.rpc.eth_call(
            &from,
            Some(&market.to_hex()),
            &hex0x(&encode_fn_addr("snapshot(address)", self.key.address)),
        ) {
            if let Ok(bytes) = decode_hex_bytes(&raw) {
                if let Some(v) = decode_word_addr(&bytes, 8) {
                    self.cfg.vapurr = v.to_checksum();
                }
                if let Some(p) = decode_word_addr(&bytes, 9) {
                    self.cfg.pusd = p.to_checksum();
                }
            }
        }
        self.cfg.save();
        Ok(hash)
    }

    pub(crate) fn send(&mut self, to: Option<Address>, data: &[u8]) -> Result<String, EconError> {
        vapurr_wallet::require_unlocked().map_err(|e| EconError::Rpc(e.to_string()))?;
        let _signing = vapurr_wallet::transactions::signing_guard().map_err(|e| EconError::Rpc(e.to_string()))?;
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(econ_rpc)?;
        if eth == 0 {
            return Err(EconError::NeedGas);
        }
        vapurr_wallet::transactions::ensure_no_pending(&from, self.chain_id).map_err(|e| EconError::Rpc(e.to_string()))?;
        let nonce = self.rpc.eth_nonce(&from).map_err(econ_rpc)?;
        let gas_price = self.rpc.eth_gas_price().unwrap_or(100_000_000);
        let to_hex = to.map(|a| a.to_hex());
        let data_hex = hex0x(data);
        let est = self
            .rpc
            .eth_estimate_gas(&from, to_hex.as_deref(), &data_hex)
            .unwrap_or(if to.is_none() { 4_000_000 } else { 500_000 });
        let gas = est.saturating_mul(13) / 10;
        let tx = Tx {
            chain_id: self.chain_id,
            nonce,
            max_priority_fee: 1_000_000,
            max_fee: gas_price.saturating_mul(3).max(1_000_000),
            gas,
            to,
            value: 0,
            data: data.to_vec(),
        };
        let raw = self
            .key
            .sign_tx(&tx)
            .map_err(|e| EconError::Rpc(e.to_string()))?;
        let hash = self.rpc.eth_send_raw(&hex0x(&raw)).map_err(econ_rpc)?;
        vapurr_wallet::transactions::record(&hash, self.chain_id, &from, "pending").map_err(|e| EconError::Rpc(e.to_string()))?;
        self.last_tx = hash.clone();
        match self.wait(&hash) {
            Ok(r) => {
                vapurr_wallet::transactions::record(&hash, self.chain_id, &from, vapurr_wallet::transactions::receipt_status(Some(&r))).map_err(|e| EconError::Rpc(e.to_string()))?;
                if r.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
                    return Err(EconError::Rpc("tx reverted".into()));
                }
                Ok(hash)
            }
            Err(EconError::Pending(h)) => Ok(h),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn wait(&self, hash: &str) -> Result<Value, EconError> {
        for _ in 0..80 {
            match self.rpc.eth_receipt(hash) {
                Ok(Some(r)) => return Ok(r),
                _ => std::thread::sleep(std::time::Duration::from_millis(400)),
            }
        }
        Err(EconError::Pending(hash.into()))
    }
}

pub(crate) const MIN_GAS_WEI: u128 = 2_000_000_000_000_000;

fn market_bytecode() -> Result<Vec<u8>, EconError> {
    hex::decode(MARKET_HEX.trim().trim_start_matches("0x")).map_err(|_| EconError::Rpc("bytecode".into()))
}

fn mock_usdg_bytecode() -> Result<Vec<u8>, EconError> {
    hex::decode(MOCK_USDG_HEX.trim().trim_start_matches("0x"))
        .map_err(|_| EconError::Rpc("usdg bytecode".into()))
}

pub(crate) fn econ_rpc(e: rhc::rpc::RpcError) -> EconError {
    let s = e.to_string();
    if let Some(reason) = revert_reason(&s) {
        return map_revert(&reason);
    }
    if s.contains("insufficient funds") {
        return EconError::NeedGas;
    }
    EconError::Rpc(s)
}

fn map_revert(r: &str) -> EconError {
    match r {
        "VAPURR" => EconError::NeedVapurr,
        "PUSD" => EconError::NeedPusd,
        "TINY" => EconError::Tiny,
        "THIN" => EconError::Thin,
        "USDG" => EconError::NeedUsdg,
        "TOP" => EconError::Top,
        "OWNER" => EconError::Owned,
        "URL" | "TITLE" => EconError::BadUrl,
        "TOKEN" => EconError::BadToken,
        "POOL" => EconError::BadPool,
        "SYM" | "NAME" | "META" => EconError::BadTicker,
        "FULL" => EconError::Full,
        "LTV" => EconError::Rpc("over 85% LTV".into()),
        "CASH" => EconError::Rpc("vault cash too thin â€” unwind or wait".into()),
        "LIQ" => EconError::Rpc("not liquidatable".into()),
        "DEBT" => EconError::Rpc("no debt".into()),
        "STEP" => EconError::Rpc("steps 1â€“16".into()),
        _ => EconError::Rpc(r.into()),
    }
}

fn to_usdg6(amt18: u128) -> Result<u128, EconError> {
    let n = amt18 / 1_000_000_000_000;
    if n == 0 {
        Err(EconError::Tiny)
    } else {
        Ok(n)
    }
}

pub fn parse_amt(s: &str) -> Result<u128, EconError> {
    let s = s.trim().replace(',', "");
    if s.is_empty() {
        return Err(EconError::Tiny);
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s.as_str(), ""),
    };
    if whole.is_empty() && frac.is_empty() {
        return Err(EconError::Tiny);
    }
    let w: u128 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| EconError::Tiny)?
    };
    let mut f = frac.chars().take(18).collect::<String>();
    while f.len() < 18 {
        f.push('0');
    }
    let frac_n: u128 = if f.is_empty() {
        0
    } else {
        f.parse().map_err(|_| EconError::Tiny)?
    };
    Ok(w.saturating_mul(DEC).saturating_add(frac_n))
}

pub(crate) fn fmt_tok(v: u128) -> String {
    if v == 0 {
        return "0.00".into();
    }
    // Dust under 0.01 must not round-hide as 0.00 (Oliver cash/supplied).
    if v < DEC / 100 {
        let frac = v / (DEC / 1_000_000);
        let mut s = format!("0.{frac:06}");
        while s.ends_with('0') && s.len() > 4 {
            s.pop();
        }
        return s;
    }
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 100);
    format!("{whole}.{frac:02}")
}

fn fmt_price(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 10_000);
    format!("{whole}.{frac:04}")
}

fn fmt_index(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 1_000_000);
    format!("{whole}.{frac:06}")
}

pub(crate) fn fmt_bps(bps: u128) -> String {
    format!("{}.{:02}", bps / 100, bps % 100)
}

/// `MIN_STABILITY_SPREAD` is 1e18-scaled (2e16 = 2%).
fn fmt_spread(v: u128) -> String {
    let bps = v.saturating_mul(10_000) / DEC;
    format!("{}.{:02}", bps / 100, bps % 100)
}

struct SnapWords {
    vapurr: u128,
    pusd: u128,
    px: u128,
    index: u128,
    vapurr_supply: u128,
    pusd_supply: u128,
    yield_res: u128,
    apy_bps: u128,
    vapurr_token: String,
    pusd_token: String,
    pool18: u128,
    min_spread: u128,
}

/// `PusdMarket.snapshot` ABI: 12 words. Word 6 = yieldReserve, 9 = pusdToken, 11 = minSpread.
fn decode_snap(bytes: &[u8]) -> Result<SnapWords, EconError> {
    if bytes.len() < 12 * 32 {
        return Err(EconError::Rpc("snapshot decode".into()));
    }
    Ok(SnapWords {
        vapurr: decode_word_u128(bytes, 0).unwrap_or(0),
        pusd: decode_word_u128(bytes, 1).unwrap_or(0),
        px: decode_word_u128(bytes, 2).unwrap_or(0),
        index: decode_word_u128(bytes, 3).unwrap_or(DEC),
        vapurr_supply: decode_word_u128(bytes, 4).unwrap_or(0),
        pusd_supply: decode_word_u128(bytes, 5).unwrap_or(0),
        yield_res: decode_word_u128(bytes, 6).unwrap_or(0),
        apy_bps: decode_word_u128(bytes, 7).unwrap_or(0),
        vapurr_token: decode_word_addr(bytes, 8)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        pusd_token: decode_word_addr(bytes, 9)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        pool18: decode_word_u128(bytes, 10).unwrap_or(0),
        min_spread: decode_word_u128(bytes, 11).unwrap_or(0),
    })
}

#[cfg(test)]
fn pack_word_u128(v: u128) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[16..32].copy_from_slice(&v.to_be_bytes());
    w
}

#[cfg(test)]
fn pack_word_addr(a: &[u8; 20]) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[12..32].copy_from_slice(a);
    w
}

fn fmt_eth(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 1_000_000);
    format!("{whole}.{frac:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_decimal() {
        assert_eq!(parse_amt("1").unwrap(), DEC);
        assert_eq!(parse_amt("1.5").unwrap(), DEC + DEC / 2);
        assert_eq!(parse_amt("0.25").unwrap(), DEC / 4);
    }

    #[test]
    fn bytecode_loads() {
        let b = market_bytecode().unwrap();
        assert!(b.len() > 1000);
        assert!(b[0] == 0x60 || b[0] == 0x61, "expected PUSH, got {:#x}", b[0]);
        let u = mock_usdg_bytecode().unwrap();
        assert!(u.len() > 200);
    }

    #[test]
    fn snap_words_are_market_not_usdg_amm() {
        let pusd = [0x11u8; 20];
        let vap = [0x22u8; 20];
        let mut bytes = vec![0u8; 12 * 32];
        bytes[0 * 32..1 * 32].copy_from_slice(&pack_word_u128(DEC)); // vapurr bal 1
        bytes[1 * 32..2 * 32].copy_from_slice(&pack_word_u128(2 * DEC));
        bytes[2 * 32..3 * 32].copy_from_slice(&pack_word_u128(DEC)); // px 1.0
        bytes[3 * 32..4 * 32].copy_from_slice(&pack_word_u128(DEC));
        bytes[4 * 32..5 * 32].copy_from_slice(&pack_word_u128(1_000_000 * DEC));
        bytes[5 * 32..6 * 32].copy_from_slice(&pack_word_u128(3 * DEC));
        bytes[6 * 32..7 * 32].copy_from_slice(&pack_word_u128(5 * DEC)); // yieldReserve
        bytes[7 * 32..8 * 32].copy_from_slice(&pack_word_u128(900));
        bytes[8 * 32..9 * 32].copy_from_slice(&pack_word_addr(&vap));
        bytes[9 * 32..10 * 32].copy_from_slice(&pack_word_addr(&pusd));
        bytes[10 * 32..11 * 32].copy_from_slice(&pack_word_u128(1_000_000 * DEC));
        bytes[11 * 32..12 * 32].copy_from_slice(&pack_word_u128(2 * 10u128.pow(16))); // 2%
        let s = decode_snap(&bytes).unwrap();
        assert_eq!(s.vapurr, DEC);
        assert_eq!(s.yield_res, 5 * DEC);
        assert_eq!(s.apy_bps, 900);
        assert_eq!(s.min_spread, 2 * 10u128.pow(16));
        assert_eq!(fmt_spread(s.min_spread), "2.00");
        let p = vapurr_wallet::tx::decode_word_addr(&bytes, 9).unwrap();
        assert_eq!(p.0, pusd);
        assert!(vapurr_wallet::tx::decode_word_addr(&bytes, 12).is_none());
        assert_eq!(fmt_bps(900), "9.00");
    }
}
