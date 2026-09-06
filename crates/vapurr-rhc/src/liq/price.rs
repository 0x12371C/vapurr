use super::*;


pub(crate) fn price_pools(rpc: &Rpc, rows: &[PoolRow]) -> Vec<Value> {
    let mut house: Vec<&PoolRow> = Vec::new();
    let mut chosen: Vec<&PoolRow> = Vec::new();
    let mut rest: Vec<&PoolRow> = Vec::new();
    for p in rows {
        if super::house::is_house_pool(&p.address) {
            house.push(p);
            continue;
        }
        let hub = p.token0.eq_ignore_ascii_case(USDG)
            || p.token1.eq_ignore_ascii_case(USDG)
            || p.token0.eq_ignore_ascii_case(WETH)
            || p.token1.eq_ignore_ascii_case(WETH);
        if hub {
            chosen.push(p);
        } else {
            rest.push(p);
        }
    }
    chosen.extend(rest);
    let room = MAX_PRICE.saturating_sub(house.len()).max(1);
    chosen.truncate(room);
    // House first so tape always carries V/PUSD mid; other hubs keep their own slot0 mids.
    let mut pinned = house;
    pinned.append(&mut chosen);
    let chosen = pinned;
    if chosen.is_empty() {
        return Vec::new();
    }
    let mut tokens: Vec<String> = Vec::new();
    for p in &chosen {
        for t in [&p.token0, &p.token1] {
            if !tokens.iter().any(|x| x.eq_ignore_ascii_case(t)) {
                tokens.push(t.to_ascii_lowercase());
            }
        }
    }
    let mut reqs = Vec::new();
    let mut id = 1u64;
    for t in &tokens {
        reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": t, "data": "0x313ce567"}, "latest"]}));
        id += 1;
        reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": t, "data": "0x95d89b41"}, "latest"]}));
        id += 1;
    }
    for p in &chosen {
        reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": p.token0, "data": pad_addr(&p.address)}, "latest"]}));
        id += 1;
        reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": p.token1, "data": pad_addr(&p.address)}, "latest"]}));
        id += 1;
    }
    let mut v3_idx: Vec<usize> = Vec::new();
    for (i, p) in chosen.iter().enumerate() {
        if p.dex.contains("v3") {
            v3_idx.push(i);
            reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": p.address, "data": SLOT0}, "latest"]}));
            id += 1;
        }
    }
    let parts = match batch_all(rpc, &reqs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("liq batch: {e}");
            return Vec::new();
        }
    };
    let mut meta: HashMap<String, (u8, String)> = HashMap::new();
    for (i, t) in tokens.iter().enumerate() {
        let dec_v = parts.get(i * 2).cloned().unwrap_or(Value::Null);
        let sym_v = parts.get(i * 2 + 1).cloned().unwrap_or(Value::Null);
        let dec = known_dec(t).unwrap_or_else(|| {
            let n = abi_u128(&dec_v);
            if n == 0 || n > 18 {
                18
            } else {
                n as u8
            }
        });
        let sym = decode_sym(&sym_v);
        meta.insert(t.clone(), (dec, canon_sym(t, &sym)));
    }
    let token_calls = tokens.len() * 2;
    let mut reserves: Vec<(f64, f64)> = Vec::new();
    for i in 0..chosen.len() {
        let a = parts
            .get(token_calls + i * 2)
            .cloned()
            .unwrap_or(Value::Null);
        let b = parts
            .get(token_calls + i * 2 + 1)
            .cloned()
            .unwrap_or(Value::Null);
        let t0 = chosen[i].token0.to_ascii_lowercase();
        let t1 = chosen[i].token1.to_ascii_lowercase();
        let d0 = meta.get(&t0).map(|m| m.0).unwrap_or(18);
        let d1 = meta.get(&t1).map(|m| m.0).unwrap_or(18);
        reserves.push((human_amt(&a, d0), human_amt(&b, d1)));
    }
    let slot_base = token_calls + chosen.len() * 2;
    let mut slots: Vec<Option<f64>> = vec![None; chosen.len()];
    for (k, i) in v3_idx.iter().enumerate() {
        let t0 = chosen[*i].token0.to_ascii_lowercase();
        let t1 = chosen[*i].token1.to_ascii_lowercase();
        let d0 = meta.get(&t0).map(|m| m.0).unwrap_or(18);
        let d1 = meta.get(&t1).map(|m| m.0).unwrap_or(18);
        slots[*i] = slot_price_t1_per_t0(parts.get(slot_base + k).unwrap_or(&Value::Null), d0, d1);
    }
    let mut prices: HashMap<String, f64> = HashMap::new();
    prices.insert(USDG.to_ascii_lowercase(), 1.0);
    prices.insert(USDE.to_ascii_lowercase(), 1.0);
    let mut deep_usdg: Option<(f64, usize)> = None;
    for (i, p) in chosen.iter().enumerate() {
        if slots[i].is_none() {
            continue;
        }
        let t0 = p.token0.to_ascii_lowercase();
        let t1 = p.token1.to_ascii_lowercase();
        let usdg = USDG.to_ascii_lowercase();
        if t0 != usdg && t1 != usdg {
            continue;
        }
        let (r0, r1) = reserves[i];
        let depth = if t0 == usdg { r0 } else { r1 };
        if deep_usdg.map(|(d, _)| depth > d).unwrap_or(true) {
            deep_usdg = Some((depth, i));
        }
    }
    if let Some((_, i)) = deep_usdg {
        let t0 = chosen[i].token0.to_ascii_lowercase();
        let t1 = chosen[i].token1.to_ascii_lowercase();
        if let Some(ratio) = slots[i] {
            set_ratio(&mut prices, &t0, &t1, ratio, true);
        }
    }
    for _ in 0..8 {
        for (i, p) in chosen.iter().enumerate() {
            let t0 = p.token0.to_ascii_lowercase();
            let t1 = p.token1.to_ascii_lowercase();
            if let Some(ratio) = slots[i] {
                set_ratio(&mut prices, &t0, &t1, ratio, false);
            }
        }
        for (i, p) in chosen.iter().enumerate() {
            let t0 = p.token0.to_ascii_lowercase();
            let t1 = p.token1.to_ascii_lowercase();
            let (r0, r1) = reserves[i];
            if r0 <= 0.0 || r1 <= 0.0 {
                continue;
            }
            set_ratio(&mut prices, &t0, &t1, r1 / r0, false);
        }
    }
    // Last write: ETH is not a dollar. A 1:1 slot0 / reserve print must not stick.
    pin_weth_from_usdg(&mut prices, &chosen, &reserves, &slots);
    let mut out = Vec::new();
    for (i, p) in chosen.iter().enumerate() {
        let t0 = p.token0.to_ascii_lowercase();
        let t1 = p.token1.to_ascii_lowercase();
        let (r0, r1) = reserves[i];
        let px0 = prices.get(&t0).copied().and_then(sane_px).unwrap_or(0.0);
        let px1 = prices.get(&t1).copied().and_then(sane_px).unwrap_or(0.0);
        let tvl = r0 * px0 + r1 * px1;
        let d0 = meta.get(&t0).map(|m| m.0).unwrap_or(18);
        let d1 = meta.get(&t1).map(|m| m.0).unwrap_or(18);
        let s0 = meta.get(&t0).map(|m| m.1.clone()).unwrap_or_else(|| canon_sym(&t0, ""));
        let s1 = meta.get(&t1).map(|m| m.1.clone()).unwrap_or_else(|| canon_sym(&t1, ""));
        let fee = fee_label(p.fee);
        out.push(json!({
            "address": p.address,
            "name": if fee.is_empty() { format!("{s0} / {s1}") } else { format!("{s0} / {s1} {fee}") },
            "dex": p.dex,
            "fee": fee,
            "base": { "address": t0, "symbol": s0, "price_usd": px0, "decimals": d0 },
            "quote": { "address": t1, "symbol": s1, "price_usd": px1, "decimals": d1 },
            "reserve_usd": tvl,
            "vol1_usd": 0.0,
            "vol6_usd": 0.0,
            "vol24_usd": 0.0,
            "mcap_usd": 0.0,
            "fdv_usd": 0.0,
            "change1": "0",
            "change24": "0",
            "buys24": 0,
            "sells24": 0,
            "txns24": 0,
        }));
    }
    out.sort_by(|a, b| {
        let av = a.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let bv = b.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    // Honest pool_mid from priced base when the graph produced a USD mid (non-house).
    for row in out.iter_mut() {
        if row.get("pool_mid").and_then(|x| x.as_f64()).is_some() {
            continue;
        }
        let px = row
            .pointer("/base/price_usd")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        if let Some(obj) = row.as_object_mut() {
            if px > 0.0 {
                obj.insert("pool_mid".into(), json!(px));
                obj.insert("mid_ok".into(), json!(true));
            } else {
                obj.insert("mid_ok".into(), json!(false));
            }
        }
    }
    // House Uni v4: chart pool mid (V USD), never Lithe oracle feed().
    super::house::apply_house_mid(&mut out);
    out.sort_by(|a, b| {
        let ah = a.get("dex").and_then(|x| x.as_str()).unwrap_or("").contains("house");
        let bh = b.get("dex").and_then(|x| x.as_str()).unwrap_or("").contains("house");
        match (ah, bh) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let av = a.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
                let bv = b.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    });
    out
}


pub(crate) fn sane_px(x: f64) -> Option<f64> {
    if x.is_finite() && x > 0.0 && x < 1.0e8 {
        Some(x)
    } else {
        None
    }
}

/// Native ETH / WETH is not a dollar. $1.00 is a 1:1 slot or a peg leak.
pub(crate) fn sane_eth_px(x: f64) -> Option<f64> {
    let x = sane_px(x)?;
    if (50.0..100_000.0).contains(&x) {
        Some(x)
    } else {
        None
    }
}

fn is_weth(addr: &str) -> bool {
    addr.eq_ignore_ascii_case(WETH) || addr.eq_ignore_ascii_case(crate::NATIVE)
}

fn put_px(prices: &mut HashMap<String, f64>, addr: &str, px: f64) {
    let px = if is_weth(addr) {
        sane_eth_px(px)
    } else {
        sane_px(px)
    };
    if let Some(px) = px {
        prices.insert(addr.to_ascii_lowercase(), px);
    }
}

/// USDG/WETH hubs only. Slot0 if sane, else reserve ratio. Deepest USDG side wins.
pub(crate) fn pin_weth_from_usdg(
    prices: &mut HashMap<String, f64>,
    chosen: &[&PoolRow],
    reserves: &[(f64, f64)],
    slots: &[Option<f64>],
) {
    let weth = WETH.to_ascii_lowercase();
    let usdg = USDG.to_ascii_lowercase();
    prices.remove(&weth);
    let mut best: Option<(f64, f64)> = None;
    for (i, p) in chosen.iter().enumerate() {
        let t0 = p.token0.to_ascii_lowercase();
        let t1 = p.token1.to_ascii_lowercase();
        let w0 = t0 == weth;
        let w1 = t1 == weth;
        let u0 = t0 == usdg;
        let u1 = t1 == usdg;
        if !(w0 || w1) || !(u0 || u1) {
            continue;
        }
        let (r0, r1) = reserves.get(i).copied().unwrap_or((0.0, 0.0));
        let (weth_amt, usdg_amt) = if w0 { (r0, r1) } else { (r1, r0) };
        let mut px = None;
        if let Some(ratio) = slots.get(i).copied().flatten() {
            px = if w0 {
                sane_eth_px(ratio)
            } else {
                sane_eth_px(1.0 / ratio)
            };
        }
        if px.is_none() && weth_amt > 1e-4 && usdg_amt > 10.0 {
            px = sane_eth_px(usdg_amt / weth_amt);
        }
        let Some(px) = px else {
            continue;
        };
        let depth = usdg_amt.max(0.0);
        if best.map(|(d, _)| depth > d).unwrap_or(true) {
            best = Some((depth, px));
        }
    }
    if let Some((_, px)) = best {
        prices.insert(weth, px);
    }
}

pub(crate) fn is_peg(addr: &str) -> bool {
    addr.eq_ignore_ascii_case(USDG) || addr.eq_ignore_ascii_case(USDE)
}


pub(crate) fn set_ratio(prices: &mut HashMap<String, f64>, t0: &str, t1: &str, t1_per_t0: f64, force: bool) {
    if !(t1_per_t0.is_finite() && t1_per_t0 > 0.0) {
        return;
    }
    if is_peg(t0) {
        prices.insert(t0.to_ascii_lowercase(), 1.0);
        if !is_peg(t1) {
            put_px(prices, t1, 1.0 / t1_per_t0);
        }
        return;
    }
    if is_peg(t1) {
        prices.insert(t1.to_ascii_lowercase(), 1.0);
        if !is_peg(t0) {
            put_px(prices, t0, t1_per_t0);
        }
        return;
    }
    let p0 = prices.get(&t0.to_ascii_lowercase()).copied();
    let p1 = prices.get(&t1.to_ascii_lowercase()).copied();
    match (p0, p1) {
        (Some(a), None) => {
            put_px(prices, t1, a / t1_per_t0);
        }
        (None, Some(b)) => {
            put_px(prices, t0, b * t1_per_t0);
        }
        (Some(a), Some(_)) if force => {
            put_px(prices, t1, a / t1_per_t0);
        }
        _ => {}
    }
}


pub(crate) fn slot_price_t1_per_t0(v: &Value, d0: u8, d1: u8) -> Option<f64> {
    let sqrt = abi_uint160_f64(v)?;
    if sqrt <= 0.0 {
        return None;
    }
    let ratio = sqrt / 2f64.powi(96);
    let raw = ratio * ratio;
    let human = raw * 10f64.powi(d0 as i32 - d1 as i32);
    if human.is_finite() && human > 0.0 {
        Some(human)
    } else {
        None
    }
}


pub(crate) fn abi_uint160_f64(v: &Value) -> Option<f64> {
    let s = v.as_str()?.trim_start_matches("0x");
    if s.is_empty() {
        return None;
    }
    let word = if s.len() >= 64 { &s[..64] } else { s };
    let take = if word.len() > 40 { &word[word.len() - 40..] } else { word };
    if take.len() <= 32 {
        return Some(u128::from_str_radix(take, 16).ok()? as f64);
    }
    let hi = u128::from_str_radix(&take[..take.len() - 32], 16).ok()? as f64;
    let lo = u128::from_str_radix(&take[take.len() - 32..], 16).ok()? as f64;
    Some(hi * 2f64.powi(128) + lo)
}


pub(crate) fn block_ts(rpc: &Rpc, n: u64) -> u64 {
    match rpc.call("eth_getBlockByNumber", json!([hex_n(n), false])) {
        Ok(v) => hex_u64(v.get("timestamp").unwrap_or(&Value::Null)),
        Err(_) => 0,
    }
}


pub(crate) fn block_span(rpc: &Rpc, head: u64, secs: u64) -> u64 {
    let sample = 8_000u64.min(head.saturating_sub(1));
    if sample == 0 {
        return 40_000;
    }
    let lo = head.saturating_sub(sample);
    let ts_hi = block_ts(rpc, head);
    let ts_lo = block_ts(rpc, lo);
    if ts_hi <= ts_lo {
        return 40_000;
    }
    let dt = (ts_hi - ts_lo) as f64 / sample as f64;
    if dt < 0.05 {
        return 120_000;
    }
    ((secs as f64 / dt).round() as u64).clamp(2_000, 120_000)
}


pub(crate) fn abi_word_signed(data: &str, word: usize) -> (u128, bool) {
    let h = data.trim_start_matches("0x");
    let start = word.saturating_mul(64);
    let Some(slice) = h.get(start..start.saturating_add(64)) else {
        return (0, false);
    };
    let neg = u8::from_str_radix(slice.get(..1).unwrap_or("0"), 16).unwrap_or(0) >= 8;
    let low_s = if slice.len() > 32 {
        &slice[slice.len() - 32..]
    } else {
        slice
    };
    let low = u128::from_str_radix(low_s, 16).unwrap_or(0);
    if neg {
        (0u128.wrapping_sub(low), true)
    } else {
        (low, false)
    }
}


pub(crate) fn abi_word_human(data: &str, word: usize, dec: u8) -> f64 {
    let (mag, neg) = abi_word_signed(data, word);
    let v = mag as f64 / 10f64.powi(dec as i32);
    if neg {
        -v
    } else {
        v
    }
}

