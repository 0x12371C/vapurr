use super::*;
use super::snapshot::push_hist;


#[cfg(test)]
pub(crate) fn split_name(name: &str) -> (String, String, String) {
    let (pair, fee) = match name.split_once("  ") {
        Some((a, b)) => (a, b.trim().to_string()),
        None => match name.rsplit_once(' ') {
            Some((a, b)) if b.contains('%') => (a, b.to_string()),
            _ => (name, String::new()),
        },
    };
    let (base, quote) = match pair.split_once(" / ") {
        Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
        None => (pair.trim().to_string(), String::new()),
    };
    (base, quote, fee)
}


pub(crate) fn canon_sym(addr: &str, fallback: &str) -> String {
    let a = addr.to_ascii_lowercase();
    if a == USDG.to_ascii_lowercase() {
        return "USDG".into();
    }
    if a == USDE.to_ascii_lowercase() {
        return "USDE".into();
    }
    if a == WETH.to_ascii_lowercase() {
        return "WETH".into();
    }
    if fallback.is_empty() {
        format!("{}…{}", &a[..6], &a[a.len().saturating_sub(4)..])
    } else {
        fallback.to_string()
    }
}


pub(crate) fn is_stable(addr: &str, sym: &str) -> bool {
    let a = addr.to_ascii_lowercase();
    if a == USDG.to_ascii_lowercase() || a == USDE.to_ascii_lowercase() {
        return true;
    }
    let s = sym.to_ascii_uppercase();
    matches!(
        s.as_str(),
        "USDG"
            | "USDE"
            | "USDC"
            | "USDT"
            | "DAI"
            | "PUSD"
            | "USDS"
            | "FRAX"
            | "PYUSD"
            | "GHO"
            | "EURC"
            | "CUSD"
            | "USD"
    ) || s.starts_with("USD")
}


pub(crate) fn is_eth(addr: &str, sym: &str) -> bool {
    let a = addr.to_ascii_lowercase();
    if a == WETH.to_ascii_lowercase() || a == crate::NATIVE.to_ascii_lowercase() {
        return true;
    }
    matches!(
        sym.to_ascii_uppercase().as_str(),
        "WETH" | "ETH" | "WETH.E"
    )
}


pub(crate) fn token_kind(addr: &str, sym: &str) -> &'static str {
    if is_eth(addr, sym) {
        "eth"
    } else if is_stable(addr, sym) {
        "stable"
    } else {
        "meme"
    }
}


pub(crate) fn build_graph(pools: Vec<Value>, pages: u32) -> Value {
    #[derive(Clone)]
    struct Tok {
        address: String,
        symbol: String,
        price_usd: f64,
        tvl: f64,
        vol: f64,
        degree: u32,
        mcap: f64,
        change24: f64,
    }
    let mut tokens: HashMap<String, Tok> = HashMap::new();
    let mut tvl = 0.0;
    let mut vol = 0.0;
    let mut vol1 = 0.0;
    let mut vol6 = 0.0;
    let mut buys = 0u64;
    let mut sells = 0u64;
    let mut dexes: HashMap<String, (u32, f64, f64)> = HashMap::new();

    for p in &pools {
        let reserve = p.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let v24 = p.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        tvl += reserve;
        vol += v24;
        vol1 += p.get("vol1_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        vol6 += p.get("vol6_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        buys += p.get("buys24").and_then(|x| x.as_u64()).unwrap_or(0);
        sells += p.get("sells24").and_then(|x| x.as_u64()).unwrap_or(0);
        let dex = p
            .get("dex")
            .and_then(|x| x.as_str())
            .unwrap_or("dex")
            .to_string();
        let e = dexes.entry(dex).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        e.1 += reserve;
        e.2 += v24;
        for side in ["base", "quote"] {
            let t = p.get(side).unwrap_or(&Value::Null);
            let addr = t
                .get("address")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if addr.is_empty() {
                continue;
            }
            let ent = tokens.entry(addr.clone()).or_insert_with(|| Tok {
                address: addr,
                symbol: t
                    .get("symbol")
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
                    .to_string(),
                price_usd: t.get("price_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
                tvl: 0.0,
                vol: 0.0,
                degree: 0,
                mcap: p.get("mcap_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
                change24: 0.0,
            });
            ent.tvl += reserve;
            ent.vol += v24;
            ent.degree += 1;
            if ent.price_usd == 0.0 {
                ent.price_usd = t.get("price_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            }
            if side == "base" && ent.change24 == 0.0 {
                ent.change24 = p
                    .get("change24")
                    .and_then(|x| x.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
            }
        }
    }

    let mut token_rows: Vec<Value> = tokens
        .values()
        .map(|t| {
            json!({
                "address": t.address,
                "symbol": t.symbol,
                "price_usd": t.price_usd,
                "tvl_usd": t.tvl,
                "vol24_usd": t.vol,
                "mcap_usd": t.mcap,
                "degree": t.degree,
                "change24": t.change24,
                "kind": token_kind(&t.address, &t.symbol),
                "hub": t.address.eq_ignore_ascii_case(USDG) || t.address.eq_ignore_ascii_case(WETH),
            })
        })
        .collect();
    token_rows.sort_by(|a, b| {
        let av = a.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let bv = b.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });

    let nodes: Vec<Value> = token_rows
        .iter()
        .map(|t| {
            json!({
                "id": t.get("address"),
                "label": t.get("symbol"),
                "address": t.get("address"),
                "tvl": t.get("tvl_usd"),
                "vol": t.get("vol24_usd"),
                "price": t.get("price_usd"),
                "change24": t.get("change24"),
                "degree": t.get("degree"),
                "kind": t.get("kind"),
                "hub": t.get("hub"),
            })
        })
        .collect();
    let edges: Vec<Value> = pools
        .iter()
        .filter_map(|p| {
            let base = p.get("base")?.get("address")?.as_str()?;
            let quote = p.get("quote")?.get("address")?.as_str()?;
            Some(json!({
                "from": base,
                "to": quote,
                "pool": p.get("address"),
                "dex": p.get("dex"),
                "name": p.get("name"),
                "reserve": p.get("reserve_usd"),
                "vol1": p.get("vol1_usd"),
                "vol6": p.get("vol6_usd"),
                "vol24": p.get("vol24_usd"),
                "fee": p.get("fee"),
                "txns24": p.get("txns24"),
                "buys24": p.get("buys24"),
                "sells24": p.get("sells24"),
            }))
        })
        .collect();

    let dex_rows: Vec<Value> = {
        let mut v: Vec<_> = dexes
            .into_iter()
            .map(|(id, (n, t, vo))| {
                json!({ "id": id, "pools": n, "tvl_usd": t, "vol24_usd": vo })
            })
            .collect();
        v.sort_by(|a, b| {
            let key = if vol > 0.0 { "vol24_usd" } else { "tvl_usd" };
            let av = a.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0);
            let bv = b.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    };

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let hubs = token_rows
        .iter()
        .filter(|t| t.get("hub").and_then(|x| x.as_bool()) == Some(true))
        .count();
    let mut movers = token_rows.clone();
    movers.sort_by(|a, b| {
        if vol > 0.0 {
            let av = a.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let bv = b.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            let av = a.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let bv = b.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    movers.truncate(8);
    let spark = push_hist(ts, tvl, vol, buys + sells);
    let txns = buys + sells;

    json!({
        "ok": true,
        "live": true,
        "source": "rhc-rpc",
        "chain": CHAIN_NAME,
        "chain_id": CHAIN_ID,
        "pages": pages,
        "ts": ts,
        "stats": {
            "pools": pools.len(),
            "tokens": token_rows.len(),
            "hubs": hubs,
            "tvl_usd": tvl,
            "vol1_usd": vol1,
            "vol6_usd": vol6,
            "vol24_usd": vol,
            "buys24": buys,
            "sells24": sells,
            "txns24": txns,
            "tape": if vol > 0.0 { "vol" } else { "tvl" },
            "vol_window_sec": 0,
        },
        "spark": spark,
        "movers": movers,
        "dexes": dex_rows,
        "tokens": token_rows,
        "pools": pools,
        "graph": { "nodes": nodes, "edges": edges },
    })
}

/// What Scan paints. Stats stay honest. Lists and the SVG stay small.

/// What Scan paints. Stats stay honest. Lists and the SVG stay small.
pub(crate) fn slim(full: &Value) -> Value {
    let mut v = full.clone();
    let Some(obj) = v.as_object_mut() else {
        return full.clone();
    };
    if let Some(tokens) = obj.get_mut("tokens").and_then(|x| x.as_array_mut()) {
        tokens.truncate(VIEW_TOKENS);
    }
    if let Some(pools) = obj.get_mut("pools").and_then(|x| x.as_array_mut()) {
        pools.truncate(VIEW_POOLS);
    }
    let graph = obj.get("graph").cloned().unwrap_or(json!({}));
    let mut nodes: Vec<Value> = graph
        .get("nodes")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    nodes.sort_by(|a, b| {
        let ah = a.get("hub").and_then(|x| x.as_bool()).unwrap_or(false);
        let bh = b.get("hub").and_then(|x| x.as_bool()).unwrap_or(false);
        match (ah, bh) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let av = a.get("tvl").and_then(|x| x.as_f64()).unwrap_or(0.0);
                let bv = b.get("tvl").and_then(|x| x.as_f64()).unwrap_or(0.0);
                bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    });
    nodes.truncate(VIEW_NODES);
    let keep: HashSet<String> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|x| x.as_str()).map(|s| s.to_ascii_lowercase()))
        .collect();
    let mut edges: Vec<Value> = graph
        .get("edges")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            let a = e
                .get("from")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let b = e
                .get("to")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            keep.contains(&a) && keep.contains(&b)
        })
        .collect();
    edges.sort_by(|a, b| {
        let av = a.get("reserve").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let bv = b.get("reserve").and_then(|x| x.as_f64()).unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    edges.truncate(VIEW_EDGES);
    obj.insert("graph".into(), json!({ "nodes": nodes, "edges": edges }));
    obj.insert("capped".into(), json!(true));
    v
}

