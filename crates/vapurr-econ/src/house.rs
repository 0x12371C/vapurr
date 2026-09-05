//! House Uniswap v4 CL book. $VAPURR / $PUSD only. Empty until this device deploys it.

use serde_json::{json, Value};

use vapurr_rhc as rhc;
use vapurr_wallet::tx::{
    decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn, hex0x,
};
use vapurr_wallet::{addr_from_hex, keccak4, Address};

use crate::{fmt_tok, parse_amt, Client, EconError, DEC, MIN_GAS_WEI};

const HOUSE_HEX: &str = include_str!("house.hex");
const Q96: u128 = 1u128 << 96;
/// ±20% around spot, aligned to tick spacing 60.
const TICK_BAND: i32 = 1860;

impl Client {
    pub(crate) fn house_snap(&self) -> Value {
        match self.house_snap_inner() {
            Ok(v) => v,
            Err(e) => self.house_base(&e.to_string()),
        }
    }

    pub(crate) fn house_deploy(&mut self) -> Result<String, EconError> {
        if self.live_house().is_some() {
            return Ok(self.cfg.house.clone());
        }
        let market = self.live_market().ok_or(EconError::NotLive)?;
        let posm = addr_from_hex(rhc::UNI_V4_POSITION_MANAGER)
            .ok_or_else(|| EconError::Rpc("posm".into()))?;
        let permit2 =
            addr_from_hex(rhc::PERMIT2).ok_or_else(|| EconError::Rpc("permit2".into()))?;
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(crate::econ_rpc)?;
        if eth < MIN_GAS_WEI {
            return Err(EconError::NeedGas);
        }
        let mut bytecode = house_bytecode()?;
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(market));
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(posm));
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(permit2));
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_u256(rhc::UNI_V4_FEE_VOL as u128));
        bytecode.extend_from_slice(&abi_i24(rhc::UNI_V4_TICK_VOL));
        let hash = self.send(None, &bytecode)?;
        let receipt = self.wait(&hash)?;
        let status = receipt
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0");
        if status != "0x1" {
            return Err(EconError::Rpc("house deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no contractAddress".into()))?;
        let addr = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad ca".into()))?;
        self.cfg.house = addr.to_checksum();
        self.cfg.save();
        Ok(hash)
    }

    pub(crate) fn house_seed_cmd(&mut self, vapurr: &str, pusd: &str) -> Result<Value, EconError> {
        let v = parse_amt(vapurr)?;
        let p = parse_amt(pusd)?;
        self.house_seed(v, p)?;
        Ok(self.snapshot())
    }

    /// 50% treasury (kept). Of the LP 50%: burn half → $PUSD, keep half as $VAPURR, seed the CL.
    pub(crate) fn house_bootstrap(&mut self) -> Result<Value, EconError> {
        if self.live_house().is_none() {
            self.house_deploy()?;
        }
        let vapurr = self.live_vapurr().ok_or(EconError::NotLive)?;
        let from = self.key.address;
        let bal = self.token_raw(vapurr, from);
        if bal < 4 * DEC {
            return Err(EconError::NeedVapurr);
        }
        let lp_budget = bal / 2;
        let burn = lp_budget / 2;
        let seed_v = lp_budget - burn;
        self.transact("swapVToPusd(uint256)", burn)?;
        let pusd = self.live_pusd().ok_or(EconError::NotLive)?;
        let pbal = self.token_raw(pusd, from);
        if pbal == 0 || seed_v == 0 {
            return Err(EconError::Tiny);
        }
        self.house_seed(seed_v, pbal)?;
        Ok(self.snapshot())
    }

    fn house_seed(&mut self, vapurr_amt: u128, pusd_amt: u128) -> Result<(), EconError> {
        if vapurr_amt == 0 || pusd_amt == 0 {
            return Err(EconError::Tiny);
        }
        let house = self.live_house().ok_or(EconError::NeedHouse)?;
        self.ensure_vapurr(house, vapurr_amt)?;
        self.ensure_pusd(house, pusd_amt)?;
        let v_addr = self
            .live_vapurr()
            .map(|a| a.to_hex())
            .unwrap_or_else(|| rhc::TESTNET_VAPURR.to_string());
        let p_addr = self
            .live_pusd()
            .map(|a| a.to_hex())
            .unwrap_or_else(|| rhc::TESTNET_PUSD.to_string());
        let px = self.vapurr_rate().unwrap_or(DEC);
        let sqrt_p = sqrt_price_x96(px, &v_addr, &p_addr);
        let tick_l = -TICK_BAND;
        let tick_u = TICK_BAND;
        let sqrt_a = tick_to_sqrt_x96(tick_l);
        let sqrt_b = tick_to_sqrt_x96(tick_u);
        let (a0, a1) = sorted_amounts(&v_addr, &p_addr, vapurr_amt, pusd_amt);
        let liq = liquidity_for_amounts(sqrt_p, sqrt_a, sqrt_b, a0, a1);
        if liq == 0 {
            return Err(EconError::Tiny);
        }
        let data = encode_seed(vapurr_amt, pusd_amt, tick_l, tick_u, liq, sqrt_p);
        self.send(Some(house), &data)?;
        Ok(())
    }

    fn house_snap_inner(&self) -> Result<Value, EconError> {
        let from = self.key.address;
        let eth = self.rpc.eth_balance(&from.to_hex()).unwrap_or(0);
        let vapurr = self.live_vapurr();
        let pusd = self.live_pusd();
        let v_bal = vapurr.map(|t| self.token_raw(t, from)).unwrap_or(0);
        let p_bal = pusd.map(|t| self.token_raw(t, from)).unwrap_or(0);
        let stocks = self.stock_bals(from);
        if self.live_market().is_none() {
            let mut v = self.house_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(true);
            v["vapurr"] = json!(fmt_tok(v_bal));
            v["pusd"] = json!(fmt_tok(p_bal));
            v["stocks"] = json!(stocks);
            v["status"] = json!("Mint $PUSD first.");
            return Ok(v);
        }
        let house = self.live_house();
        if house.is_none() {
            let mut v = self.house_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(false);
            v["need_deploy"] = json!(true);
            v["vapurr"] = json!(fmt_tok(v_bal));
            v["pusd"] = json!(fmt_tok(p_bal));
            v["stocks"] = json!(stocks);
            v["lp_v"] = json!(fmt_tok(v_bal / 4));
            v["treasury_v"] = json!(fmt_tok(v_bal / 2));
            v["status"] = json!(if eth < MIN_GAS_WEI {
                "Need gas."
            } else if v_bal == 0 {
                "This device has 0 $VAPURR. Send V and P here, then deploy and seed."
            } else {
                "ready"
            });
            return Ok(v);
        }
        let h = house.unwrap();
        let data = encode_fn("snapshot()");
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&h.to_hex()), &hex0x(&data))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("house snapshot".into()))?;
        let s = decode_house_snap(&bytes)?;
        Ok(json!({
            "live": s.token_id > 0,
            "need_deploy": false,
            "need_market": false,
            "need_eth": eth < MIN_GAS_WEI,
            "address": from.to_checksum(),
            "house": h.to_checksum(),
            "swap": self.cfg.swap,
            "token_id": s.token_id.to_string(),
            "pool_id": s.pool_id,
            "tick_lower": s.tick_lower,
            "tick_upper": s.tick_upper,
            "liquidity": s.liquidity.to_string(),
            "fee": "0.30",
            "spacing": 60,
            "band": "±20%",
            "vapurr": fmt_tok(v_bal),
            "pusd": fmt_tok(p_bal),
            "px": fmt_price(s.px),
            "vapurr_token": s.vapurr_token,
            "pusd_token": s.pusd_token,
            "posm": s.posm,
            "owner": s.owner,
            "explorer": format!("{}/address/{}", self.explorer(), h.to_hex()),
            "tx": self.last_tx,
            "tx_url": if self.last_tx.is_empty() {
                String::new()
            } else {
                format!("{}/tx/{}", self.explorer(), self.last_tx)
            },
            "eth": fmt_eth(eth),
            "stocks": stocks,
            "lp_v": fmt_tok(v_bal / 4),
            "treasury_v": fmt_tok(v_bal / 2),
            "status": if s.token_id > 0 { "live" } else { "deployed, not seeded" },
        }))
    }

    fn house_base(&self, err: &str) -> Value {
        json!({
            "live": false,
            "need_deploy": true,
            "need_market": self.live_market().is_none(),
            "need_eth": true,
            "address": self.key.address.to_checksum(),
            "house": self.cfg.house,
            "swap": self.cfg.swap,
            "token_id": "0",
            "pool_id": "",
            "tick_lower": -TICK_BAND,
            "tick_upper": TICK_BAND,
            "liquidity": "0",
            "fee": "0.30",
            "spacing": 60,
            "band": "±20%",
            "vapurr": "0.00",
            "pusd": "0.00",
            "px": "0.0000",
            "vapurr_token": self.cfg.vapurr,
            "pusd_token": self.cfg.pusd,
            "posm": rhc::UNI_V4_POSITION_MANAGER,
            "owner": "",
            "explorer": "",
            "tx": self.last_tx,
            "tx_url": "",
            "eth": "0.000000",
            "stocks": [],
            "lp_v": "0.00",
            "treasury_v": "0.00",
            "status": if err.is_empty() { "not on chain" } else { err },
            "error": err,
        })
    }

    fn live_house(&self) -> Option<Address> {
        self.live_ca(&self.cfg.house)
    }

    fn vapurr_rate(&self) -> Option<u128> {
        let m = self.live_market()?;
        let data = encode_fn("vapurrRate()");
        let raw = self
            .rpc
            .eth_call(&self.key.address.to_hex(), Some(&m.to_hex()), &hex0x(&data))
            .ok()?;
        let bytes = decode_hex_bytes(&raw).ok()?;
        decode_word_u128(&bytes, 0)
    }

    fn stock_bals(&self, holder: Address) -> Vec<Value> {
        let mut out = Vec::new();
        for (sym, addr) in rhc::TESTNET_STOCKS {
            let Some(t) = addr_from_hex(addr) else { continue };
            let n = self.token_raw(t, holder);
            if n == 0 {
                continue;
            }
            out.push(json!({
                "symbol": sym,
                "token": addr,
                "bal": fmt_tok(n),
            }));
        }
        out
    }
}

fn house_bytecode() -> Result<Vec<u8>, EconError> {
    hex::decode(HOUSE_HEX.trim().trim_start_matches("0x"))
        .map_err(|_| EconError::Rpc("house bytecode".into()))
}

fn abi_i24(t: i32) -> [u8; 32] {
    let mut w = if t < 0 { [0xffu8; 32] } else { [0u8; 32] };
    w[28..].copy_from_slice(&t.to_be_bytes());
    w
}

fn encode_seed(
    vapurr: u128,
    pusd: u128,
    tick_l: i32,
    tick_u: i32,
    liq: u128,
    sqrt_p: u128,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(4 + 6 * 32);
    d.extend_from_slice(&keccak4(
        "seed(uint256,uint256,int24,int24,uint128,uint160)",
    ));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(vapurr));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(pusd));
    d.extend_from_slice(&abi_i24(tick_l));
    d.extend_from_slice(&abi_i24(tick_u));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(liq));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(sqrt_p));
    d
}

fn sorted_amounts(vapurr_addr: &str, pusd_addr: &str, vapurr: u128, pusd: u128) -> (u128, u128) {
    let v = vapurr_addr.to_ascii_lowercase();
    let p = pusd_addr.to_ascii_lowercase();
    if v < p {
        (vapurr, pusd)
    } else {
        (pusd, vapurr)
    }
}

fn sqrt_price_x96(vapurr_rate: u128, vapurr_addr: &str, pusd_addr: &str) -> u128 {
    // Uniswap sqrtPrice is sqrt(token1/token0) * 2^96. vapurrRate is P per V.
    let s = isqrt(vapurr_rate.max(1));
    let v_is_token0 = vapurr_addr.to_ascii_lowercase() < pusd_addr.to_ascii_lowercase();
    if v_is_token0 {
        s.saturating_mul(Q96) / 1_000_000_000
    } else if s == 0 {
        0
    } else {
        Q96.saturating_mul(1_000_000_000) / s
    }
}

fn isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.saturating_add(1) / 2;
    while y < x {
        x = y;
        y = x.saturating_add(n / x) / 2;
    }
    x
}

fn tick_to_sqrt_x96(tick: i32) -> u128 {
    if tick == 0 {
        return Q96;
    }
    let price = (1.0001f64).powi(tick);
    let sqrt = price.sqrt();
    (sqrt * Q96 as f64) as u128
}

fn liquidity_for_amounts(sqrt_p: u128, sqrt_a: u128, sqrt_b: u128, a0: u128, a1: u128) -> u128 {
    let q96 = Q96 as f64;
    let mut sa = sqrt_a as f64;
    let mut sb = sqrt_b as f64;
    let sp = sqrt_p as f64;
    if sa > sb {
        std::mem::swap(&mut sa, &mut sb);
    }
    let l = if sp <= sa {
        liq0(sa, sb, a0 as f64, q96)
    } else if sp >= sb {
        liq1(sa, sb, a1 as f64, q96)
    } else {
        liq0(sp, sb, a0 as f64, q96).min(liq1(sa, sp, a1 as f64, q96))
    };
    if !l.is_finite() || l <= 0.0 {
        return 0;
    }
    (l * 0.98) as u128
}

fn liq0(sa: f64, sb: f64, a0: f64, q96: f64) -> f64 {
    let den = sb - sa;
    if den <= 0.0 {
        return 0.0;
    }
    a0 * sa / q96 * sb / den
}

fn liq1(sa: f64, sb: f64, a1: f64, q96: f64) -> f64 {
    let den = sb - sa;
    if den <= 0.0 {
        return 0.0;
    }
    a1 * q96 / den
}

fn fmt_eth(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 1_000_000);
    format!("{whole}.{frac:06}")
}

fn fmt_price(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 10_000);
    format!("{whole}.{frac:04}")
}

fn decode_i24(bytes: &[u8], word: usize) -> i32 {
    let start = word * 32;
    if bytes.len() < start + 32 {
        return 0;
    }
    i32::from_be_bytes(bytes[start + 28..start + 32].try_into().unwrap_or([0; 4]))
}

fn word_hex32(bytes: &[u8], word: usize) -> String {
    let start = word * 32;
    if bytes.len() < start + 32 {
        return String::new();
    }
    format!("0x{}", hex::encode(&bytes[start..start + 32]))
}

struct HouseSnap {
    token_id: u128,
    pool_id: String,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    px: u128,
    vapurr_token: String,
    pusd_token: String,
    posm: String,
    owner: String,
}

fn decode_house_snap(bytes: &[u8]) -> Result<HouseSnap, EconError> {
    if bytes.len() < 14 * 32 {
        return Err(EconError::Rpc("house snapshot decode".into()));
    }
    Ok(HouseSnap {
        token_id: decode_word_u128(bytes, 0).unwrap_or(0),
        pool_id: word_hex32(bytes, 1),
        tick_lower: decode_i24(bytes, 2),
        tick_upper: decode_i24(bytes, 3),
        liquidity: decode_word_u128(bytes, 4).unwrap_or(0),
        px: decode_word_u128(bytes, 7).unwrap_or(0),
        vapurr_token: decode_word_addr(bytes, 8)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        pusd_token: decode_word_addr(bytes, 9)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        posm: decode_word_addr(bytes, 10)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        owner: decode_word_addr(bytes, 13)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_bytecode_loads() {
        let b = house_bytecode().unwrap();
        assert!(b.len() > 1000);
        assert!(b[0] == 0x60 || b[0] == 0x61, "expected PUSH, got {:#x}", b[0]);
    }

    #[test]
    fn one_to_one_sqrt_is_q96() {
        assert_eq!(
            sqrt_price_x96(DEC, "0x41", "0x59"),
            Q96
        );
        assert_eq!(
            sqrt_price_x96(DEC, "0x59", "0x41"),
            Q96
        );
        assert_eq!(tick_to_sqrt_x96(0), Q96);
    }

    #[test]
    fn band_liquidity_nonzero() {
        let l = liquidity_for_amounts(Q96, tick_to_sqrt_x96(-1860), tick_to_sqrt_x96(1860), DEC, DEC);
        assert!(l > 0, "liq {l}");
    }

    #[test]
    fn vapurr_is_token0() {
        let (a0, a1) = sorted_amounts(
            "0x4100000000000000000000000000000000000000",
            "0x5900000000000000000000000000000000000000",
            10,
            20,
        );
        assert_eq!((a0, a1), (10, 20));
    }
}
