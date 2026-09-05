//! House Uni v4 book for Ketcharts: pool mid from PoolManager.extsload, not Lithe feed().

use super::*;
use sha3::{Digest, Keccak256};

/// Uniswap v4 Pool.State mapping slot (StateLibrary.POOLS_SLOT).
const POOLS_SLOT: u8 = 6;
/// extsload(bytes32)
const EXTSLOAD: &str = "0x1e2eaeaf";
/// poolId()
const POOL_ID: &str = "0x3e0dc34e";
/// HouseSwap Swap(address,bool,uint256,uint256)
const HOUSE_SWAP_TOPIC: &str =
    "0xbfd50a04f1e6e4aee344f5d0e7f15d74d0dbb58cd1f711daa6463094ca9508cd";
const MIN_SQRT: f64 = 4_295_128_740.0;
const MAX_SQRT: f64 = 1.461_446_703_485_210_1e48;

#[derive(Clone, Debug)]
pub(crate) struct HouseMid {
    pub pool_id: String,
    pub sqrt_price_x96: f64,
    pub tick: i32,
    /// Human token1/token0 from slot0.
    pub t1_per_t0: f64,
    /// $VAPURR (equity) USD from pool mid with $PUSD = $1. None if mid at limit.
    pub v_usd: Option<f64>,
    pub token0_is_pusd: bool,
}

pub(crate) fn is_house_pool(addr: &str) -> bool {
    !crate::TESTNET_HOUSE.is_empty() && addr.eq_ignore_ascii_case(crate::TESTNET_HOUSE)
}

pub(crate) fn house_mid(rpc: &Rpc) -> Option<HouseMid> {
    if crate::TESTNET_HOUSE.is_empty() || crate::UNI_V4_POOL_MANAGER.is_empty() {
        return None;
    }
    let house = crate::TESTNET_HOUSE;
    let pm = crate::UNI_V4_POOL_MANAGER;
    let pool_id = match rpc.eth_call(
        "0x0000000000000000000000000000000000000001",
        Some(house),
        POOL_ID,
    ) {
        Ok(raw) => normalize_word(&raw)?,
        Err(_) => return None,
    };
    if pool_id.chars().all(|c| c == '0') {
        return None;
    }
    let slot = pool_state_slot(&pool_id);
    let data_hex = format!("{EXTSLOAD}{slot}");
    let raw = rpc
        .eth_call(
            "0x0000000000000000000000000000000000000001",
            Some(pm),
            &data_hex,
        )
        .ok()?;
    let word = normalize_word(&raw)?;
    let sqrt = abi_uint160_f64(&json!(format!("0x{word}")))?;
    let tick = parse_tick24(&word);
    let v = crate::TESTNET_VAPURR.to_ascii_lowercase();
    let p = crate::TESTNET_PUSD.to_ascii_lowercase();
    let token0_is_pusd = p < v;
    let t1_per_t0 = slot_price_t1_per_t0(&json!(format!("0x{word}")), 18, 18)?;
    let at_limit = sqrt <= MIN_SQRT + 16.0 || sqrt >= MAX_SQRT * 0.999;
    let v_usd = if at_limit || !(t1_per_t0.is_finite() && t1_per_t0 > 0.0) {
        None
    } else if token0_is_pusd {
        // token1/token0 = V per PUSD → V USD = t1_per_t0 when PUSD=$1
        sane_px(t1_per_t0)
    } else {
        // token1/token0 = PUSD per V → V USD = 1 / ratio
        sane_px(1.0 / t1_per_t0)
    };
    Some(HouseMid {
        pool_id: format!("0x{pool_id}"),
        sqrt_price_x96: sqrt,
        tick,
        t1_per_t0,
        v_usd,
        token0_is_pusd,
    })
}

/// Public helper for econ snap / FE: pool-mid $V USD, never Lithe feed.
pub fn house_v_usd_mid() -> Option<f64> {
    let rpc = Rpc::at_timeout(crate::TESTNET_RPC_HTTP, 8);
    house_mid(&rpc).and_then(|m| m.v_usd)
}

pub(crate) fn apply_house_mid(rows: &mut [Value]) {
    let Some(i) = rows.iter().position(|p| {
        p.get("address")
            .and_then(|x| x.as_str())
            .map(is_house_pool)
            .unwrap_or(false)
    }) else {
        return;
    };
    let rpc = Rpc::at_timeout(crate::TESTNET_RPC_HTTP, 8);
    let Some(mid) = house_mid(&rpc) else {
        return;
    };
    let v = crate::TESTNET_VAPURR.to_ascii_lowercase();
    let p = crate::TESTNET_PUSD.to_ascii_lowercase();
    let v_px = mid.v_usd.unwrap_or(0.0);
    let (base, quote) = (
        json!({ "address": v, "symbol": "VAPURR", "price_usd": v_px, "decimals": 18 }),
        json!({ "address": p, "symbol": "PUSD", "price_usd": 1.0, "decimals": 18 }),
    );
    if let Some(obj) = rows[i].as_object_mut() {
        obj.insert("base".into(), base);
        obj.insert("quote".into(), quote);
        obj.insert("name".into(), json!("VAPURR / PUSD 0.30%"));
        obj.insert("dex".into(), json!("uniswap v4 house"));
        obj.insert("fee".into(), json!("0.30%"));
        obj.insert("pool_id".into(), json!(mid.pool_id));
        obj.insert("pool_mid".into(), json!(v_px));
        obj.insert("pool_tick".into(), json!(mid.tick));
        obj.insert("mid_ok".into(), json!(mid.v_usd.is_some()));
        let liq = obj
            .get("reserve_usd")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        if liq < 1.0 {
            obj.insert("reserve_usd".into(), json!(1.0));
        }
    }
}

pub(crate) fn fetch_house_trades(rpc: &Rpc, from: u64, head: u64) -> Vec<Value> {
    if crate::TESTNET_SWAP.is_empty() {
        return Vec::new();
    }
    let logs =
        get_logs(rpc, crate::TESTNET_SWAP, HOUSE_SWAP_TOPIC, from, head).unwrap_or_default();
    let ts_hi = block_ts(rpc, head);
    let sample = head.saturating_sub(from).max(1);
    let dt = if ts_hi > 0 {
        2.0_f64.max(8_000.0 / sample as f64).min(4.0)
    } else {
        2.0
    };
    let mut out: Vec<Value> = logs
        .iter()
        .filter_map(|log| map_house_swap(log, ts_hi, head, dt))
        .collect();
    out.sort_by(|a, b| {
        let at = a.get("block").and_then(|x| x.as_u64()).unwrap_or(0);
        let bt = b.get("block").and_then(|x| x.as_u64()).unwrap_or(0);
        bt.cmp(&at)
    });
    out.truncate(80);
    out
}

fn map_house_swap(log: &Value, ts_hi: u64, head: u64, dt: f64) -> Option<Value> {
    let bn = hex_u64(log.get("blockNumber").unwrap_or(&Value::Null));
    let tx = log
        .get("transactionHash")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if tx.is_empty() {
        return None;
    }
    let data = log.get("data").and_then(|x| x.as_str()).unwrap_or("0x");
    let sell_v = abi_word_human(data, 0, 0) > 0.0;
    let amount_in = abi_word_human(data, 1, 18).abs();
    let amount_out = abi_word_human(data, 2, 18).abs();
    if !(amount_in > 0.0 && amount_out > 0.0) {
        return None;
    }
    let px = if sell_v {
        amount_out / amount_in
    } else {
        amount_in / amount_out
    };
    if !px.is_finite() || px <= 0.0 {
        return None;
    }
    let vol = if sell_v { amount_out } else { amount_in };
    let behind = head.saturating_sub(bn) as f64;
    let ts = if ts_hi > 0 {
        ts_hi.saturating_sub((behind * dt) as u64)
    } else {
        0
    };
    Some(json!({
        "tx": tx,
        "block": bn,
        "time": ts.saturating_mul(1000),
        "buy": !sell_v,
        "px": px,
        "vol": vol,
        "base_amt": if sell_v { amount_in } else { amount_out },
        "quote_amt": if sell_v { amount_out } else { amount_in },
        "src": "house",
    }))
}

fn pool_state_slot(pool_id_hex: &str) -> String {
    let mut pid = [0u8; 32];
    let h = pool_id_hex.trim_start_matches("0x");
    if let Ok(bytes) = hex::decode(h) {
        let n = bytes.len().min(32);
        pid[32 - n..].copy_from_slice(&bytes[bytes.len() - n..]);
    }
    let mut pools = [0u8; 32];
    pools[31] = POOLS_SLOT;
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(&pid);
    buf[32..].copy_from_slice(&pools);
    let dig = Keccak256::digest(buf);
    hex::encode(dig)
}

fn normalize_word(raw: &str) -> Option<String> {
    let h = raw.trim().trim_start_matches("0x");
    if h.is_empty() {
        return None;
    }
    if h.len() >= 64 {
        Some(h[h.len() - 64..].to_ascii_lowercase())
    } else {
        Some(format!("{h:0>64}"))
    }
}

fn parse_tick24(word: &str) -> i32 {
    // bits 160..184 of the 256-bit word → hex chars [18..24) from the left (64 hex chars).
    if word.len() < 64 {
        return 0;
    }
    let tick_hex = &word[18..24];
    let tick_raw = i32::from_str_radix(tick_hex, 16).unwrap_or(0);
    if tick_raw >= 0x80_0000 {
        tick_raw - 0x100_0000
    } else {
        tick_raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_state_slot_matches_cast_index() {
        let id = "4480433a3e442ad8d328422d099be4dc2217492c727e13a188a0d0a7b807d97b";
        let slot = pool_state_slot(id);
        assert_eq!(
            slot,
            "c7ee15e7bbb3711b9bcbadfa21d0af3b181f2ac63ce5d2b2f7087e7ec4e2c9dc"
        );
    }

    #[test]
    fn detects_house_address() {
        assert!(is_house_pool(crate::TESTNET_HOUSE));
        assert!(!is_house_pool(crate::TESTNET_PUSD));
    }

    #[test]
    fn parses_min_tick_word() {
        let word = "000000000bb8000000f2761800000000000000000000000000000001000276a4";
        assert_eq!(parse_tick24(word), -887272);
    }
}
