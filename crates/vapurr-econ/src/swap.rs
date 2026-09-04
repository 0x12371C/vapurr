//! House Uniswap v4 exact-in swap. $VAPURR / $PUSD only.

use serde_json::Value;

use vapurr_rhc as rhc;
use vapurr_wallet::{addr_from_hex, keccak4, Address};

use crate::{Client, EconError, DEC, MIN_GAS_WEI};

const SWAP_HEX: &str = include_str!("swap.hex");

impl Client {
    pub(crate) fn swap_deploy(&mut self) -> Result<String, EconError> {
        if self.live_swap().is_some() {
            return Ok(self.cfg.swap.clone());
        }
        let market = self.live_market().ok_or(EconError::NotLive)?;
        let pm = addr_from_hex(rhc::UNI_V4_POOL_MANAGER)
            .ok_or_else(|| EconError::Rpc("pool manager".into()))?;
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(crate::econ_rpc)?;
        if eth < MIN_GAS_WEI {
            return Err(EconError::NeedGas);
        }
        let mut bytecode = swap_bytecode()?;
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(market));
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(pm));
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_u256(rhc::UNI_V4_FEE_VOL as u128));
        bytecode.extend_from_slice(&abi_i24(rhc::UNI_V4_TICK_VOL));
        let hash = self.send(None, &bytecode)?;
        let receipt = self.wait(&hash)?;
        if receipt.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
            return Err(EconError::Rpc("swap deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no contractAddress".into()))?;
        let addr = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad ca".into()))?;
        self.cfg.swap = addr.to_checksum();
        self.cfg.save();
        Ok(hash)
    }

    pub(crate) fn house_swap(&mut self, sell_v: bool, amt: u128) -> Result<(), EconError> {
        if amt == 0 {
            return Err(EconError::Tiny);
        }
        let swap = self.live_swap().ok_or(EconError::NeedSwap)?;
        if sell_v {
            self.ensure_vapurr(swap, amt)?;
        } else {
            self.ensure_pusd(swap, amt)?;
        }
        let min = amt.saturating_mul(90) / 100;
        let data = encode_swap_exact(sell_v, amt, min);
        self.send(Some(swap), &data)?;
        Ok(())
    }

    /// House tape first. Euler only to hold a healthy book. Mint is never fatal.
    pub(crate) fn pulse(&mut self) -> Result<Value, EconError> {
        let mut notes: Vec<String> = Vec::new();
        if self.cfg.swap.is_empty() {
            match self.swap_deploy() {
                Ok(_) => notes.push("swap-deploy".into()),
                Err(e) => notes.push(format!("swap-deploy:{e}")),
            }
        }
        if self.cfg.loop_vault.is_empty() {
            match self.euler_deploy() {
                Ok(_) => notes.push("vault-deploy".into()),
                Err(e) => notes.push(format!("vault-deploy:{e}")),
            }
        }
        let vapurr = self.live_vapurr().ok_or(EconError::NotLive)?;
        let pusd = self.live_pusd().ok_or(EconError::NotLive)?;
        let from = self.key.address;

        let v = self.token_raw(vapurr, from);
        if v >= 15 * DEC {
            match self.house_swap(true, 10 * DEC) {
                Ok(()) => notes.push("sellV".into()),
                Err(e) => notes.push(format!("sellV:{e}")),
            }
        }
        let p = self.token_raw(pusd, from);
        if p >= 10 * DEC {
            match self.house_swap(false, 8 * DEC) {
                Ok(()) => notes.push("sellP".into()),
                Err(e) => notes.push(format!("sellP:{e}")),
            }
        }

        let vault = self.euler_snap();
        if vault.get("live").and_then(|x| x.as_bool()).unwrap_or(false) {
            let hf = json_f64(&vault, "health");
            let util = json_f64(&vault, "util");
            let collat_v = json_f64(&vault, "collat_v");
            let debt = json_f64(&vault, "debt");
            let room = json_f64(&vault, "room");
            let cash = json_f64(&vault, "cash");
            let p_bal = self.token_raw(pusd, from);
            let v_bal = self.token_raw(vapurr, from);
            // loop() one step maxes LTV. unwind() one step clears all debt.
            // Small borrow/repay is the util tape.
            if debt > 0.0 && hf > 0.0 && hf < 1.10 {
                match self.euler_op("repay", "15", "") {
                    Ok(_) => notes.push("repay".into()),
                    Err(e) => notes.push(format!("repay:{e}")),
                }
            } else if cash < 80.0 && p_bal >= 30 * DEC {
                match self.euler_op("supply", "20", "") {
                    Ok(_) => notes.push("supply".into()),
                    Err(e) => notes.push(format!("supply:{e}")),
                }
            } else if collat_v < 50.0 && v_bal >= 50 * DEC {
                match self.euler_op("depositV", "50", "") {
                    Ok(_) => notes.push("collatV".into()),
                    Err(e) => notes.push(format!("collatV:{e}")),
                }
            } else if util > 72.0 && debt > 25.0 {
                match self.euler_op("repay", "25", "") {
                    Ok(_) => notes.push("repay".into()),
                    Err(e) => notes.push(format!("repay:{e}")),
                }
            } else if util < 45.0 && cash > 40.0 && room > 40.0 && (debt == 0.0 || hf >= 1.40) {
                match self.euler_op("borrow", "40", "") {
                    Ok(_) => notes.push("borrow".into()),
                    Err(e) => notes.push(format!("borrow:{e}")),
                }
            }
        }

        let p2 = self.token_raw(pusd, from);
        let v2 = self.token_raw(vapurr, from);
        if p2 < 40 * DEC && v2 >= 80 * DEC {
            match self.transact("swapLunaToUst(uint256)", 40 * DEC) {
                Ok(_) => notes.push("mint".into()),
                Err(e) => notes.push(format!("mint:{e}")),
            }
        }

        let mut out = self.book_snap();
        out["notes"] = serde_json::json!(notes.join(" "));
        Ok(out)
    }

    fn live_swap(&self) -> Option<Address> {
        self.live_ca(&self.cfg.swap)
    }
}

fn json_f64(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::String(s)) => s.replace(',', "").parse().unwrap_or(0.0),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn swap_bytecode() -> Result<Vec<u8>, EconError> {
    hex::decode(SWAP_HEX.trim().trim_start_matches("0x"))
        .map_err(|_| EconError::Rpc("swap bytecode".into()))
}

fn abi_i24(t: i32) -> [u8; 32] {
    let mut w = if t < 0 { [0xffu8; 32] } else { [0u8; 32] };
    w[28..].copy_from_slice(&t.to_be_bytes());
    w
}

fn encode_swap_exact(sell_v: bool, amt: u128, min_out: u128) -> Vec<u8> {
    let mut d = Vec::with_capacity(4 + 96);
    d.extend_from_slice(&keccak4("swapExact(bool,uint256,uint256)"));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(if sell_v { 1 } else { 0 }));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(amt));
    d.extend_from_slice(&vapurr_wallet::tx::abi_u256(min_out));
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_bytecode_loads() {
        let b = swap_bytecode().unwrap();
        assert!(b.len() > 500);
        assert!(b[0] == 0x60 || b[0] == 0x61);
    }
}
