use super::*;


pub(crate) fn fill_swaps(rpc: &Rpc, rows: &mut [Value], head: u64) -> u64 {
    if rows.is_empty() || head == 0 {
        return 0;
    }
    let want = block_span(rpc, head, 86_400);
    let chunks = if SWAPS_DEEP.swap(true, Ordering::SeqCst) {
        VOL_CHUNKS
    } else {
        1
    };
    let span = want.min(CHUNK * chunks);
    let from = head.saturating_sub(span).max(1);
    let ts_hi = block_ts(rpc, head);
    let ts_lo = block_ts(rpc, from);
    let window = if ts_hi > ts_lo { ts_hi - ts_lo } else { 0 };
    let span1 = block_span(rpc, head, 3_600).min(span);
    let span6 = block_span(rpc, head, 21_600).min(span);
    let b1 = head.saturating_sub(span1);
    let b6 = head.saturating_sub(span6);
    let n = rows.len().min(24);
    let mut v2 = Vec::new();
    let mut v3 = Vec::new();
    let mut by = HashMap::new();
    for (i, p) in rows.iter().take(n).enumerate() {
        let Some(addr) = p.get("address").and_then(|x| x.as_str()) else {
            continue;
        };
        // House Uni v4 prints live on HouseSwap (testnet) — do not mix into mainnet v2/v3 log crawl.
        if super::house::is_house_pool(addr) {
            continue;
        }
        let a = addr.to_ascii_lowercase();
        by.insert(a.clone(), i);
        if p.get("dex").and_then(|x| x.as_str()).unwrap_or("").contains("v3") {
            v3.push(a);
        } else {
            v2.push(a);
        }
    }
    let mut lo = from;
    let mut calls = 0u32;
    let call_cap = (chunks as u32).saturating_mul(2).saturating_add(2);
    while lo <= head && calls < call_cap {
        let hi = lo.saturating_add(CHUNK).min(head);
        for (addrs, topic) in [(&v3, SWAP_V3), (&v2, SWAP_V2)] {
            if addrs.is_empty() {
                continue;
            }
            calls += 1;
            match get_logs_many(rpc, addrs, topic, lo, hi) {
                Ok(logs) => {
                    for log in logs {
                        apply_swap(rows, &by, &log, b1, b6);
                    }
                }
                Err(e) => eprintln!("liq swaps {lo}-{hi}: {e}"),
            }
        }
        lo = hi.saturating_add(1);
    }
    window
}


pub(crate) fn bump_f(p: &mut Value, key: &str, n: f64) {
    let cur = p.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0);
    if let Some(obj) = p.as_object_mut() {
        obj.insert(key.into(), json!(cur + n));
    }
}


pub(crate) fn bump_u(p: &mut Value, key: &str) {
    let cur = p.get(key).and_then(|x| x.as_u64()).unwrap_or(0);
    if let Some(obj) = p.as_object_mut() {
        obj.insert(key.into(), json!(cur + 1));
    }
}


pub(crate) fn apply_swap(rows: &mut [Value], by: &HashMap<String, usize>, log: &Value, b1: u64, b6: u64) {
    let Some(pool) = log.get("address").and_then(|x| x.as_str()) else {
        return;
    };
    let Some(&i) = by.get(&pool.to_ascii_lowercase()) else {
        return;
    };
    let bn = hex_u64(log.get("blockNumber").unwrap_or(&Value::Null));
    let data = log.get("data").and_then(|x| x.as_str()).unwrap_or("0x");
    let d0 = rows[i]
        .get("base")
        .and_then(|x| x.get("decimals"))
        .and_then(|x| x.as_u64())
        .unwrap_or(18) as u8;
    let d1 = rows[i]
        .get("quote")
        .and_then(|x| x.get("decimals"))
        .and_then(|x| x.as_u64())
        .unwrap_or(18) as u8;
    let px0 = rows[i]
        .get("base")
        .and_then(|x| x.get("price_usd"))
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let px1 = rows[i]
        .get("quote")
        .and_then(|x| x.get("price_usd"))
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let v3 = rows[i]
        .get("dex")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .contains("v3");
    let (vol, buy) = if v3 {
        let a0 = abi_word_human(data, 0, d0);
        let a1 = abi_word_human(data, 1, d1);
        let v = if px0 > 0.0 {
            a0.abs() * px0
        } else {
            a1.abs() * px1
        };
        (v, a0 < 0.0)
    } else {
        let i0 = abi_word_human(data, 0, d0).abs();
        let i1 = abi_word_human(data, 1, d1).abs();
        let o0 = abi_word_human(data, 2, d0).abs();
        (i0 * px0 + i1 * px1, o0 > 0.0)
    };
    if !(vol.is_finite() && vol > 0.0) {
        return;
    }
    let p = &mut rows[i];
    bump_f(p, "vol24_usd", vol);
    if bn >= b6 {
        bump_f(p, "vol6_usd", vol);
    }
    if bn >= b1 {
        bump_f(p, "vol1_usd", vol);
    }
    bump_u(p, "txns24");
    if buy {
        bump_u(p, "buys24");
    } else {
        bump_u(p, "sells24");
    }
}


pub(crate) fn human_amt(v: &Value, dec: u8) -> f64 {
    let n = abi_u128(v);
    if n == 0 {
        return 0.0;
    }
    n as f64 / 10f64.powi(dec as i32)
}


pub(crate) fn decode_sym(v: &Value) -> String {
    let s = v.as_str().unwrap_or("");
    let h = s.trim_start_matches("0x");
    if h.is_empty() {
        return String::new();
    }
    let bytes = hex::decode(h).unwrap_or_default();
    if bytes.len() >= 64 {
        let n = {
            let mut b = [0u8; 8];
            if bytes.len() >= 32 {
                b.copy_from_slice(&bytes[24..32]);
            }
            u64::from_be_bytes(b) as usize
        };
        let start = 32;
        if bytes.len() >= start + n && n < 64 {
            return String::from_utf8_lossy(&bytes[start..start + n])
                .chars()
                .filter(|c| c.is_ascii_graphic())
                .collect();
        }
    }
    if bytes.len() == 32 {
        return bytes
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as char)
            .filter(|c| c.is_ascii_graphic())
            .collect();
    }
    String::new()
}

