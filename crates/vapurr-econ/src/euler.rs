//! Isolated $PUSD credit vault. Euler-shaped. Not deployed until this device does it.

use serde_json::{json, Value};

use vapurr_wallet::tx::{
    decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn_addr, encode_fn_addr_addr,
    encode_fn_addr_u256, encode_fn_two_u256, encode_fn_u256, hex0x,
};
use vapurr_wallet::{addr_from_hex, Address};

use crate::{fmt_bps, fmt_tok, parse_amt, Client, EconError, DEC, MIN_GAS_WEI};

const LOOP_HEX: &str = include_str!("loop.hex");

impl Client {
    pub(crate) fn euler_snap(&self) -> Value {
        match self.euler_snap_inner() {
            Ok(v) => v,
            Err(e) => self.euler_base(&e.to_string()),
        }
    }

    pub(crate) fn euler_deploy(&mut self) -> Result<String, EconError> {
        if self.live_loop().is_some() {
            return Ok(self.cfg.loop_vault.clone());
        }
        let market = self.live_market().ok_or(EconError::NotLive)?;
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(crate::econ_rpc)?;
        if eth < MIN_GAS_WEI {
            return Err(EconError::NeedGas);
        }
        let mut bytecode = loop_bytecode()?;
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(market));
        let hash = self.send(None, &bytecode)?;
        let receipt = self.wait(&hash)?;
        let status = receipt
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0");
        if status != "0x1" {
            return Err(EconError::Rpc("vault deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no contractAddress".into()))?;
        let addr = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad ca".into()))?;
        self.cfg.loop_vault = addr.to_checksum();
        self.cfg.save();
        Ok(hash)
    }

    pub(crate) fn euler_op(
        &mut self,
        op: &str,
        amt: &str,
        steps: &str,
    ) -> Result<Value, EconError> {
        let vault = self.live_loop().ok_or(EconError::NeedLoop)?;
        let op = op.trim().to_ascii_lowercase();
        match op.as_str() {
            "supply" => {
                let n = parse_amt(amt)?;
                self.ensure_pusd(vault, n)?;
                let data = encode_fn_u256("supply(uint256)", n);
                self.send(Some(vault), &data)?;
            }
            "withdraw" => {
                let n = parse_amt(amt)?;
                let data = encode_fn_u256("withdraw(uint256)", n);
                self.send(Some(vault), &data)?;
            }
            "depositv" | "deposit_v" => {
                let n = parse_amt(amt)?;
                self.ensure_vapurr(vault, n)?;
                let data = encode_fn_u256("depositV(uint256)", n);
                self.send(Some(vault), &data)?;
            }
            "withdrawv" | "withdraw_v" => {
                let n = parse_amt(amt)?;
                let data = encode_fn_u256("withdrawV(uint256)", n);
                self.send(Some(vault), &data)?;
            }
            "borrow" => {
                let n = parse_amt(amt)?;
                let data = encode_fn_u256("borrow(uint256)", n);
                self.send(Some(vault), &data)?;
            }
            "repay" => {
                let n = parse_amt(amt)?;
                self.ensure_pusd(vault, n)?;
                let data = encode_fn_u256("repay(uint256)", n);
                self.send(Some(vault), &data)?;
            }
            "loop" => {
                let n = if amt.trim().is_empty() {
                    0
                } else {
                    parse_amt(amt)?
                };
                if n > 0 {
                    self.ensure_pusd(vault, n)?;
                }
                let steps = parse_steps(steps);
                let data = encode_fn_two_u256("loop(uint256,uint256)", n, steps);
                self.send(Some(vault), &data)?;
            }
            "unwind" => {
                let steps = parse_steps(steps);
                let data = encode_fn_u256("unwind(uint256)", steps);
                self.send(Some(vault), &data)?;
            }
            _ => return Err(EconError::Rpc("unknown vault op".into())),
        }
        Ok(self.snapshot())
    }

    fn euler_snap_inner(&self) -> Result<Value, EconError> {
        let from = self.key.address;
        let eth = self.rpc.eth_balance(&from.to_hex()).unwrap_or(0);
        let market = self.live_market();
        let vault = self.live_loop();
        if market.is_none() {
            let mut v = self.euler_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(true);
            v["need_deploy"] = json!(true);
            v["status"] = json!("Mint $PUSD first.");
            return Ok(v);
        }
        if vault.is_none() {
            let mut v = self.euler_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(false);
            v["need_deploy"] = json!(true);
            v["address"] = json!(from.to_checksum());
            v["market"] = json!(market.unwrap().to_checksum());
            v["status"] = json!(if eth < MIN_GAS_WEI {
                "Need gas."
            } else {
                "ready"
            });
            return Ok(v);
        }
        let b = vault.unwrap();
        let data = encode_fn_addr("snapshot(address)", from);
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&b.to_hex()), &hex0x(&data))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("vault snapshot".into()))?;
        let s = decode_loop_snap(&bytes)?;
        Ok(json!({
            "live": true,
            "need_deploy": false,
            "need_market": false,
            "need_eth": eth < MIN_GAS_WEI,
            "address": from.to_checksum(),
            "vault": b.to_checksum(),
            "market": s.market,
            "vapurr_token": s.vapurr_token,
            "pusd_token": s.pusd_token,
            "explorer": format!("{}/address/{}", self.explorer(), b.to_hex()),
            "tx": self.last_tx,
            "tx_url": if self.last_tx.is_empty() {
                String::new()
            } else {
                format!("{}/tx/{}", self.explorer(), self.last_tx)
            },
            "eth": fmt_eth(eth),
            "cash": fmt_tok(s.cash),
            "supplied_total": fmt_tok(s.total_supply),
            "borrowed_total": fmt_tok(s.total_borrow),
            "util": fmt_pct_wad(s.util),
            "borrow_apy": fmt_bps(s.borrow_apy_bps),
            "supply_apy": fmt_bps(s.supply_apy_bps),
            "ltv": fmt_bps(s.ltv_bps),
            "lltv": fmt_bps(s.lltv_bps),
            "px": fmt_price(s.px),
            "supplied": fmt_tok(s.supplied),
            "collat_v": fmt_tok(s.collat_v),
            "debt": fmt_tok(s.debt),
            "collat_value": fmt_tok(s.collat_value),
            "health": fmt_hf(s.health),
            "vapurr": fmt_tok(s.vapurr_bal),
            "pusd": fmt_tok(s.pusd_bal),
            "room": fmt_tok(s.room),
            "boot_kink": fmt_bps(s.boot_bps),
            "flow": fmt_pct_wad(s.flow_wad),
            "cash_target": fmt_tok(s.cash_target),
            "max_steps": 16,
            "status": "live",
        }))
    }

    fn euler_base(&self, err: &str) -> Value {
        json!({
            "live": false,
            "need_deploy": true,
            "need_market": self.live_market().is_none(),
            "need_eth": true,
            "address": self.key.address.to_checksum(),
            "vault": self.cfg.loop_vault,
            "market": self.cfg.market,
            "vapurr_token": self.cfg.vapurr,
            "pusd_token": self.cfg.pusd,
            "explorer": "",
            "tx": self.last_tx,
            "tx_url": "",
            "eth": "0.000000",
            "cash": "0.00",
            "supplied_total": "0.00",
            "borrowed_total": "0.00",
            "util": "0.00",
            "borrow_apy": "0.00",
            "supply_apy": "0.00",
            "ltv": "85.00",
            "lltv": "90.00",
            "px": "0.0000",
            "supplied": "0.00",
            "collat_v": "0.00",
            "debt": "0.00",
            "collat_value": "0.00",
            "health": "100.00",
            "vapurr": "0.00",
            "pusd": "0.00",
            "room": "0.00",
            "boot_kink": "150.00",
            "flow": "0.00",
            "cash_target": "100000.00",
            "max_steps": 16,
            "status": if err.is_empty() { "not on chain" } else { err },
            "error": err,
        })
    }

    pub(crate) fn live_loop(&self) -> Option<Address> {
        self.live_ca(&self.cfg.loop_vault)
    }

    pub(crate) fn live_vapurr(&self) -> Option<Address> {
        if !self.cfg.vapurr.is_empty() {
            if let Some(a) = addr_from_hex(&self.cfg.vapurr) {
                return Some(a);
            }
        }
        let m = self.live_market()?;
        let data = encode_fn_addr("snapshot(address)", self.key.address);
        let raw = self
            .rpc
            .eth_call(&self.key.address.to_hex(), Some(&m.to_hex()), &hex0x(&data))
            .ok()?;
        let bytes = decode_hex_bytes(&raw).ok()?;
        decode_word_addr(&bytes, 8)
    }

    pub(crate) fn ensure_vapurr(&mut self, spender: Address, need: u128) -> Result<(), EconError> {
        let vapurr = self.live_vapurr().ok_or(EconError::NotLive)?;
        let from = self.key.address;
        let data = encode_fn_addr("balanceOf(address)", from);
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&vapurr.to_hex()), &hex0x(&data))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).unwrap_or_default();
        let bal = decode_word_u128(&bytes, 0).unwrap_or(0);
        if bal < need {
            return Err(EconError::NeedVapurr);
        }
        let allow_data = encode_fn_addr_addr("allowance(address,address)", from, spender);
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&vapurr.to_hex()), &hex0x(&allow_data))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).unwrap_or_default();
        let allow = decode_word_u128(&bytes, 0).unwrap_or(0);
        if allow >= need {
            return Ok(());
        }
        let approve = encode_fn_addr_u256("approve(address,uint256)", spender, u128::MAX);
        self.send(Some(vapurr), &approve)?;
        Ok(())
    }
}

fn loop_bytecode() -> Result<Vec<u8>, EconError> {
    hex::decode(LOOP_HEX.trim().trim_start_matches("0x"))
        .map_err(|_| EconError::Rpc("vault bytecode".into()))
}

fn parse_steps(s: &str) -> u128 {
    let t = s.trim();
    if t.is_empty() {
        return 8;
    }
    t.parse::<u128>()
        .ok()
        .filter(|n| *n > 0)
        .unwrap_or(8)
        .min(16)
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

fn fmt_pct_wad(v: u128) -> String {
    let bps = v.saturating_mul(10_000) / DEC;
    format!("{}.{:02}", bps / 100, bps % 100)
}

fn fmt_hf(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 100);
    format!("{whole}.{frac:02}")
}

struct LoopSnap {
    cash: u128,
    total_supply: u128,
    total_borrow: u128,
    util: u128,
    borrow_apy_bps: u128,
    supply_apy_bps: u128,
    ltv_bps: u128,
    lltv_bps: u128,
    px: u128,
    supplied: u128,
    collat_v: u128,
    debt: u128,
    collat_value: u128,
    health: u128,
    vapurr_bal: u128,
    pusd_bal: u128,
    vapurr_token: String,
    pusd_token: String,
    market: String,
    room: u128,
    boot_bps: u128,
    flow_wad: u128,
    cash_target: u128,
}

/// `PusdLoop.snapshot` ABI: 20 words, boot fields from word 20.
fn decode_loop_snap(bytes: &[u8]) -> Result<LoopSnap, EconError> {
    if bytes.len() < 20 * 32 {
        return Err(EconError::Rpc("vault snapshot decode".into()));
    }
    Ok(LoopSnap {
        cash: decode_word_u128(bytes, 0).unwrap_or(0),
        total_supply: decode_word_u128(bytes, 1).unwrap_or(0),
        total_borrow: decode_word_u128(bytes, 2).unwrap_or(0),
        util: decode_word_u128(bytes, 3).unwrap_or(0),
        borrow_apy_bps: decode_word_u128(bytes, 4).unwrap_or(0),
        supply_apy_bps: decode_word_u128(bytes, 5).unwrap_or(0),
        ltv_bps: decode_word_u128(bytes, 6).unwrap_or(0),
        lltv_bps: decode_word_u128(bytes, 7).unwrap_or(0),
        px: decode_word_u128(bytes, 8).unwrap_or(0),
        supplied: decode_word_u128(bytes, 9).unwrap_or(0),
        collat_v: decode_word_u128(bytes, 10).unwrap_or(0),
        debt: decode_word_u128(bytes, 11).unwrap_or(0),
        collat_value: decode_word_u128(bytes, 12).unwrap_or(0),
        health: decode_word_u128(bytes, 13).unwrap_or(0),
        vapurr_bal: decode_word_u128(bytes, 14).unwrap_or(0),
        pusd_bal: decode_word_u128(bytes, 15).unwrap_or(0),
        vapurr_token: decode_word_addr(bytes, 16)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        pusd_token: decode_word_addr(bytes, 17)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        market: decode_word_addr(bytes, 18)
            .map(|a| a.to_checksum())
            .unwrap_or_default(),
        room: decode_word_u128(bytes, 19).unwrap_or(0),
        boot_bps: decode_word_u128(bytes, 20).unwrap_or(0),
        flow_wad: decode_word_u128(bytes, 21).unwrap_or(0),
        cash_target: decode_word_u128(bytes, 22).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vapurr_wallet::tx::{decode_word_addr, decode_word_u128};

    fn pack_word_u128(v: u128) -> [u8; 32] {
        let mut w = [0u8; 32];
        w[16..32].copy_from_slice(&v.to_be_bytes());
        w
    }

    fn pack_word_addr(a: &[u8; 20]) -> [u8; 32] {
        let mut w = [0u8; 32];
        w[12..32].copy_from_slice(a);
        w
    }

    #[test]
    fn loop_bytecode_loads() {
        let b = loop_bytecode().unwrap();
        assert!(b.len() > 1000);
        assert!(
            b[0] == 0x60 || b[0] == 0x61,
            "expected PUSH, got {:#x}",
            b[0]
        );
    }

    #[test]
    fn steps_cap_at_sixteen() {
        assert_eq!(parse_steps(""), 8);
        assert_eq!(parse_steps("3"), 3);
        assert_eq!(parse_steps("99"), 16);
        assert_eq!(parse_steps("0"), 8);
    }

    #[test]
    fn snap_words_are_vault_not_usdg() {
        let vap = [0x22u8; 20];
        let pusd = [0x11u8; 20];
        let mkt = [0x33u8; 20];
        let mut bytes = vec![0u8; 20 * 32];
        bytes[0 * 32..1 * 32].copy_from_slice(&pack_word_u128(10 * DEC));
        bytes[1 * 32..2 * 32].copy_from_slice(&pack_word_u128(40 * DEC));
        bytes[2 * 32..3 * 32].copy_from_slice(&pack_word_u128(30 * DEC));
        bytes[3 * 32..4 * 32].copy_from_slice(&pack_word_u128(DEC * 85 / 100));
        bytes[4 * 32..5 * 32].copy_from_slice(&pack_word_u128(566));
        bytes[5 * 32..6 * 32].copy_from_slice(&pack_word_u128(433));
        bytes[6 * 32..7 * 32].copy_from_slice(&pack_word_u128(8500));
        bytes[7 * 32..8 * 32].copy_from_slice(&pack_word_u128(9000));
        bytes[8 * 32..9 * 32].copy_from_slice(&pack_word_u128(DEC));
        bytes[9 * 32..10 * 32].copy_from_slice(&pack_word_u128(40 * DEC));
        bytes[10 * 32..11 * 32].copy_from_slice(&pack_word_u128(5 * DEC));
        bytes[11 * 32..12 * 32].copy_from_slice(&pack_word_u128(30 * DEC));
        bytes[12 * 32..13 * 32].copy_from_slice(&pack_word_u128(45 * DEC));
        bytes[13 * 32..14 * 32].copy_from_slice(&pack_word_u128(DEC + DEC / 2));
        bytes[14 * 32..15 * 32].copy_from_slice(&pack_word_u128(DEC));
        bytes[15 * 32..16 * 32].copy_from_slice(&pack_word_u128(2 * DEC));
        bytes[16 * 32..17 * 32].copy_from_slice(&pack_word_addr(&vap));
        bytes[17 * 32..18 * 32].copy_from_slice(&pack_word_addr(&pusd));
        bytes[18 * 32..19 * 32].copy_from_slice(&pack_word_addr(&mkt));
        bytes[19 * 32..20 * 32].copy_from_slice(&pack_word_u128(8 * DEC));
        let s = decode_loop_snap(&bytes).unwrap();
        assert_eq!(s.cash, 10 * DEC);
        assert_eq!(s.ltv_bps, 8500);
        assert_eq!(s.lltv_bps, 9000);
        assert_eq!(s.room, 8 * DEC);
        assert_eq!(fmt_pct_wad(s.util), "85.00");
        assert_eq!(fmt_hf(s.health), "1.50");
        assert_eq!(fmt_bps(s.borrow_apy_bps), "5.66");
        let p = decode_word_addr(&bytes, 17).unwrap();
        assert_eq!(p.0, pusd);
        assert!(decode_word_addr(&bytes, 20).is_none());
        assert_eq!(decode_word_u128(&bytes, 6).unwrap(), 8500);
    }
}
