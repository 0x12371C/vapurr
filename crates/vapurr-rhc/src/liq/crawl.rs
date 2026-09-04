use super::*;
use super::snapshot::{disk_path, idle};


#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct PoolRow {
    pub(crate) address: String,
    pub(crate) token0: String,
    pub(crate) token1: String,
    pub(crate) fee: u32,
    pub(crate) dex: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct PoolIdx {
    pub(crate) last_block: u64,
    #[serde(default)]
    pub(crate) first_block: u64,
    pub(crate) pools: Vec<PoolRow>,
}


pub(crate) fn idx_path() -> PathBuf {
    let mut p = disk_path();
    p.set_file_name("liq-pools.json");
    p
}


pub(crate) fn load_idx() -> PoolIdx {
    fs::read(idx_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}


pub(crate) fn save_idx(idx: &PoolIdx) {
    if cfg!(test) {
        return;
    }
    if let Some(dir) = idx_path().parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(b) = serde_json::to_vec(idx) {
        let _ = fs::write(idx_path(), b);
    }
}


pub(crate) fn fetch_rpc() -> Result<Value, String> {
    let rpc = Rpc::liq();
    let head = hex_u64(&rpc.call("eth_blockNumber", json!([])).map_err(|e| e.to_string())?);
    if head == 0 {
        return Ok(idle("rpc wait"));
    }
    let mut idx = load_idx();
    seed_hubs(&rpc, &mut idx);
    seed_v2(&rpc, &mut idx);
    if idx.last_block == 0 {
        // Hubs + v2 sample paint first. The warm loop walks logs backward.
        idx.last_block = head;
        if idx.first_block == 0 {
            idx.first_block = head.max(1);
        }
    } else {
        let fwd = idx.last_block.saturating_add(1).min(head);
        if fwd <= head {
            crawl_range(&rpc, &mut idx, fwd, head);
            idx.last_block = head;
        }
        if idx.first_block > 1 {
            let first = idx.first_block;
            let back_to = first.saturating_sub(FIRST_SPAN).max(1);
            if back_to < first {
                crawl_range(&rpc, &mut idx, back_to, first - 1);
                idx.first_block = back_to;
            }
        }
    }
    prune_idx(&mut idx);
    save_idx(&idx);
    if idx.pools.is_empty() {
        return Ok(idle("empty"));
    }
    let mut rows = price_pools(&rpc, &idx.pools);
    if rows.is_empty() {
        return Ok(idle("empty"));
    }
    let window = fill_swaps(&rpc, &mut rows, head);
    let mut g = build_graph(rows, 1);
    if let Some(st) = g.get_mut("stats").and_then(|x| x.as_object_mut()) {
        st.insert("vol_window_sec".into(), json!(window));
        let vol = st.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        st.insert("tape".into(), json!(if vol > 0.0 { "vol" } else { "tvl" }));
    }
    Ok(g)
}


pub(crate) fn factories() -> [(&'static str, &'static str, &'static str); 3] {
    [
        (UNI_V3_FACTORY, POOL_CREATED, "uniswap v3"),
        (SUSHI_V3_FACTORY, POOL_CREATED, "sushi v3"),
        (UNI_V2_FACTORY, PAIR_CREATED, "uniswap v2"),
    ]
}


pub(crate) fn sort_pair(a: &str, b: &str) -> (String, String) {
    let a = a.to_ascii_lowercase();
    let b = b.to_ascii_lowercase();
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}


pub(crate) fn ingest(idx: &mut PoolIdx, mut row: PoolRow) {
    let (a, b) = sort_pair(&row.token0, &row.token1);
    row.token0 = a;
    row.token1 = b;
    row.address = row.address.to_ascii_lowercase();
    if let Some(old) = idx.pools.iter_mut().find(|p| p.address == row.address) {
        *old = row;
        return;
    }
    idx.pools.push(row);
}


pub(crate) fn is_hub(p: &PoolRow) -> bool {
    p.token0.eq_ignore_ascii_case(USDG)
        || p.token1.eq_ignore_ascii_case(USDG)
        || p.token0.eq_ignore_ascii_case(WETH)
        || p.token1.eq_ignore_ascii_case(WETH)
        || p.token0.eq_ignore_ascii_case(USDE)
        || p.token1.eq_ignore_ascii_case(USDE)
}


pub(crate) fn prune_idx(idx: &mut PoolIdx) {
    if idx.pools.len() <= MAX_INDEX {
        return;
    }
    idx.pools.sort_by(|a, b| is_hub(b).cmp(&is_hub(a)));
    idx.pools.truncate(MAX_INDEX);
}


pub(crate) fn pad32(addr: &str) -> String {
    let h = addr.trim_start_matches("0x").trim_start_matches("0X").to_ascii_lowercase();
    format!("{h:0>64}")
}


pub(crate) fn seed_known(idx: &mut PoolIdx) {
    if !crate::TESTNET_HOUSE.is_empty()
        && !crate::TESTNET_VAPURR.is_empty()
        && !crate::TESTNET_PUSD.is_empty()
    {
        let v = crate::TESTNET_VAPURR;
        let p = crate::TESTNET_PUSD;
        let (t0, t1) = if v.to_ascii_lowercase() < p.to_ascii_lowercase() {
            (v, p)
        } else {
            (p, v)
        };
        ingest(
            idx,
            PoolRow {
                address: crate::TESTNET_HOUSE.into(),
                token0: t0.into(),
                token1: t1.into(),
                fee: crate::UNI_V4_FEE_VOL,
                dex: "uniswap v4 house".into(),
            },
        );
    }
    // Live 4663 USDG/WETH hubs. getPool batch sometimes returns empty; pin the ones we have seen.
    for (address, fee, dex) in [
        (
            "0x52e65b17fb6e5ba00ed806f37afcd2daa50271ca",
            100u32,
            "uniswap v3",
        ),
        (
            "0x8803c117ccae7b5146297876c2a25df135141c4d",
            3000,
            "uniswap v2",
        ),
        (
            "0x69bfaf19c9f377bb306a89aed9f6b07e2c1a8d9a",
            500,
            "uniswap v3",
        ),
        (
            "0xa9188730fe85be88ad499d7d52b099e800fb0334",
            3000,
            "uniswap v3",
        ),
    ] {
        ingest(
            idx,
            PoolRow {
                address: address.into(),
                token0: WETH.into(),
                token1: USDG.into(),
                fee,
                dex: dex.into(),
            },
        );
    }
}


pub(crate) fn seed_hubs(rpc: &Rpc, idx: &mut PoolIdx) {
    seed_known(idx);
    let tokens = [USDG, WETH, USDE];
    let fees = [100u32, 500, 3000, 10_000];
    let mut reqs = Vec::new();
    let mut meta: Vec<(&str, String, String, u32)> = Vec::new();
    let mut id = 1u64;
    for i in 0..tokens.len() {
        for j in (i + 1)..tokens.len() {
            let a = tokens[i];
            let b = tokens[j];
            reqs.push(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "eth_call",
                "params": [{"to": UNI_V2_FACTORY, "data": format!("0xe6a43905{}{}", pad32(a), pad32(b))}, "latest"]
            }));
            meta.push(("uniswap v2", a.into(), b.into(), 3000));
            id += 1;
            for fee in fees {
                for (factory, dex) in [
                    (UNI_V3_FACTORY, "uniswap v3"),
                    (SUSHI_V3_FACTORY, "sushi v3"),
                ] {
                    reqs.push(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "method": "eth_call",
                        "params": [{"to": factory, "data": format!("0x1698ee82{}{}{fee:064x}", pad32(a), pad32(b))}, "latest"]
                    }));
                    meta.push((dex, a.into(), b.into(), fee));
                    id += 1;
                }
            }
        }
    }
    let parts = match batch_all(rpc, &reqs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("liq hubs: {e}");
            return;
        }
    };
    for (i, (dex, t0, t1, fee)) in meta.into_iter().enumerate() {
        let mut pool = pool_addr(parts.get(i).unwrap_or(&Value::Null));
        if pool.is_none() {
            let r = &reqs[i];
            let method = r.get("method").and_then(|x| x.as_str()).unwrap_or("eth_call");
            let params = r.get("params").cloned().unwrap_or(json!([]));
            if let Ok(v) = rpc.call(method, params) {
                pool = pool_addr(&v);
            }
        }
        let Some(pool) = pool else {
            continue;
        };
        ingest(
            idx,
            PoolRow {
                address: pool,
                token0: t0,
                token1: t1,
                fee,
                dex: dex.into(),
            },
        );
    }
}


pub(crate) fn pool_addr(v: &Value) -> Option<String> {
    let a = data_addr(v.as_str().unwrap_or(""), 0)?;
    if a.chars().any(|c| c != '0' && c != 'x') {
        Some(a)
    } else {
        None
    }
}


pub(crate) fn seed_v2(rpc: &Rpc, idx: &mut PoolIdx) {
    let n = match rpc.call(
        "eth_call",
        json!([{"to": UNI_V2_FACTORY, "data": "0x574f2ba3"}, "latest"]),
    ) {
        Ok(v) => abi_u128(&v) as u64,
        Err(e) => {
            eprintln!("liq v2 length: {e}");
            return;
        }
    };
    if n == 0 {
        return;
    }
    let take = n.min(32);
    let start = n.saturating_sub(take);
    let mut reqs = Vec::new();
    for i in start..n {
        reqs.push(json!({
            "jsonrpc": "2.0",
            "id": i - start + 1,
            "method": "eth_call",
            "params": [{"to": UNI_V2_FACTORY, "data": format!("0x1e3dd18b{i:064x}")}, "latest"]
        }));
    }
    let pairs = match batch_all(rpc, &reqs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("liq v2 pairs: {e}");
            return;
        }
    };
    let mut addrs = Vec::new();
    for v in &pairs {
        if let Some(a) = data_addr(v.as_str().unwrap_or(""), 0) {
            if a.len() == 42 {
                addrs.push(a);
            }
        }
    }
    if addrs.is_empty() {
        return;
    }
    let mut t_reqs = Vec::new();
    let mut id = 1u64;
    for a in &addrs {
        t_reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": a, "data": "0x0dfe1681"}, "latest"]}));
        id += 1;
        t_reqs.push(json!({"jsonrpc":"2.0","id":id,"method":"eth_call","params":[{"to": a, "data": "0xd21220a7"}, "latest"]}));
        id += 1;
    }
    let toks = match batch_all(rpc, &t_reqs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("liq v2 tokens: {e}");
            return;
        }
    };
    for (i, a) in addrs.iter().enumerate() {
        let t0 = data_addr(toks.get(i * 2).and_then(|x| x.as_str()).unwrap_or(""), 0);
        let t1 = data_addr(toks.get(i * 2 + 1).and_then(|x| x.as_str()).unwrap_or(""), 0);
        if let (Some(token0), Some(token1)) = (t0, t1) {
            ingest(
                idx,
                PoolRow {
                    address: a.clone(),
                    token0,
                    token1,
                    fee: 3000,
                    dex: "uniswap v2".into(),
                },
            );
        }
    }
}


pub(crate) fn crawl_range(rpc: &Rpc, idx: &mut PoolIdx, from: u64, to: u64) {
    let mut lo = from;
    while lo <= to {
        let hi = lo.saturating_add(CHUNK).min(to);
        for (factory, topic, dex) in factories() {
            match get_logs(rpc, factory, topic, lo, hi) {
                Ok(logs) => {
                    for log in logs {
                        if let Some(row) = parse_created(&log, dex) {
                            ingest(idx, row);
                        }
                    }
                }
                Err(e) => eprintln!("liq logs {dex} {lo}-{hi}: {e}"),
            }
        }
        lo = hi.saturating_add(1);
    }
}


pub(crate) fn get_logs_raw(
    rpc: &Rpc,
    address: &Value,
    topic: &str,
    from: u64,
    to: u64,
) -> Result<Vec<Value>, String> {
    let filter = json!([{
        "fromBlock": hex_n(from),
        "toBlock": hex_n(to),
        "address": address,
        "topics": [topic],
    }]);
    let v = rpc.call("eth_getLogs", filter.clone()).map_err(|e| e.to_string());
    match v {
        Ok(Value::Array(a)) => Ok(a),
        Ok(_) => Err("decode".into()),
        Err(e) => {
            let l = e.to_ascii_lowercase();
            if l.contains("too many") || l.contains("rate") || l.contains("timeout") {
                std::thread::sleep(Duration::from_millis(400));
                let again = rpc.call("eth_getLogs", filter).map_err(|e| e.to_string())?;
                return match again {
                    Value::Array(a) => Ok(a),
                    _ => Err("decode".into()),
                };
            }
            Err(e)
        }
    }
}


pub(crate) fn get_logs_split(
    rpc: &Rpc,
    address: &Value,
    topic: &str,
    from: u64,
    to: u64,
) -> Result<Vec<Value>, String> {
    match get_logs_raw(rpc, address, topic, from, to) {
        Ok(v) => Ok(v),
        Err(_) if from < to => {
            let mid = from + (to - from) / 2;
            let mut a = get_logs_split(rpc, address, topic, from, mid)?;
            let b = get_logs_split(rpc, address, topic, mid + 1, to)?;
            a.extend(b);
            Ok(a)
        }
        Err(e) => Err(e),
    }
}


pub(crate) fn get_logs(
    rpc: &Rpc,
    address: &str,
    topic: &str,
    from: u64,
    to: u64,
) -> Result<Vec<Value>, String> {
    get_logs_split(rpc, &json!(address), topic, from, to)
}


pub(crate) fn get_logs_many(
    rpc: &Rpc,
    addrs: &[String],
    topic: &str,
    from: u64,
    to: u64,
) -> Result<Vec<Value>, String> {
    if addrs.is_empty() {
        return Ok(Vec::new());
    }
    let addr = if addrs.len() == 1 {
        json!(addrs[0])
    } else {
        json!(addrs)
    };
    get_logs_split(rpc, &addr, topic, from, to)
}


pub(crate) fn parse_created(log: &Value, dex: &str) -> Option<PoolRow> {
    let topics = log.get("topics")?.as_array()?;
    let token0 = topic_addr(topics.get(1)?.as_str()?)?;
    let token1 = topic_addr(topics.get(2)?.as_str()?)?;
    let data = log.get("data")?.as_str().unwrap_or("0x");
    let (fee, pool) = if topics.len() >= 4 {
        let fee = topic_u24(topics.get(3)?.as_str()?);
        let pool = data_addr(data, 1)?;
        (fee, pool)
    } else {
        (3000u32, data_addr(data, 0)?)
    };
    if pool.len() != 42 {
        return None;
    }
    Some(PoolRow {
        address: pool,
        token0,
        token1,
        fee,
        dex: dex.into(),
    })
}


pub(crate) fn topic_addr(t: &str) -> Option<String> {
    let h = t.trim_start_matches("0x");
    if h.len() < 40 {
        return None;
    }
    Some(format!("0x{}", &h[h.len() - 40..]).to_ascii_lowercase())
}


pub(crate) fn topic_u24(t: &str) -> u32 {
    let h = t.trim_start_matches("0x");
    let take = if h.len() > 8 { &h[h.len() - 8..] } else { h };
    u32::from_str_radix(take, 16).unwrap_or(0)
}


pub(crate) fn data_addr(data: &str, word: usize) -> Option<String> {
    let h = data.trim_start_matches("0x");
    let start = word.checked_mul(64)?;
    let slice = h.get(start..start + 64)?;
    if slice.len() < 40 {
        return None;
    }
    Some(format!("0x{}", &slice[slice.len() - 40..]).to_ascii_lowercase())
}


pub(crate) fn hex_n(n: u64) -> String {
    format!("0x{n:x}")
}


pub(crate) fn fee_label(fee: u32) -> String {
    if fee == 0 {
        return String::new();
    }
    format!("{:.2}%", fee as f64 / 10_000.0)
}


pub(crate) fn pad_addr(a: &str) -> String {
    let h = a
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_ascii_lowercase();
    format!("0x70a08231000000000000000000000000{h}")
}


pub(crate) fn known_dec(addr: &str) -> Option<u8> {
    let a = addr.to_ascii_lowercase();
    if a == USDG.to_ascii_lowercase() {
        Some(crate::USDG_DECIMALS)
    } else if a == WETH.to_ascii_lowercase() || a == USDE.to_ascii_lowercase() {
        Some(18)
    } else {
        None
    }
}


pub(crate) fn batch_all(rpc: &Rpc, reqs: &[Value]) -> Result<Vec<Value>, String> {
    if reqs.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(reqs.len());
    for chunk in reqs.chunks(20) {
        match rpc.batch(chunk) {
            Ok(mut part) => out.append(&mut part),
            Err(_) => {
                for r in chunk {
                    let method = r.get("method").and_then(|x| x.as_str()).unwrap_or("");
                    let params = r.get("params").cloned().unwrap_or(json!([]));
                    match rpc.call(method, params) {
                        Ok(v) => out.push(v),
                        Err(_) => out.push(Value::Null),
                    }
                }
            }
        }
    }
    Ok(out)
}

