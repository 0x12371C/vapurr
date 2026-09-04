use super::*;
use super::crawl::fetch_rpc;
use super::graph::slim;


pub(crate) fn disk_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData").join("Local"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("vapurr").join("liq-cache.json")
}


pub(crate) fn load_disk() -> Option<Value> {
    let raw = fs::read(disk_path()).ok()?;
    let v: Value = serde_json::from_slice(&raw).ok()?;
    if v.get("ok").and_then(|x| x.as_bool()) == Some(true)
        && v.get("source").and_then(|x| x.as_str()) == Some("rhc-rpc")
    {
        Some(v)
    } else {
        None
    }
}


pub(crate) fn save_disk(v: &Value) {
    if cfg!(test) {
        return;
    }
    if v.get("ok").and_then(|x| x.as_bool()) != Some(true) {
        return;
    }
    if let Some(dir) = disk_path().parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(bytes) = serde_json::to_vec(v) {
        let _ = fs::write(disk_path(), bytes);
    }
}


pub(crate) fn seed_from_disk() {
    let Some(v) = load_disk() else {
        return;
    };
    if let Ok(g) = CACHE.lock() {
        if let Some((_, old)) = g.as_ref() {
            if old.get("ok").and_then(|x| x.as_bool()) == Some(true) {
                return;
            }
        }
    }
    remember(v);
}


pub fn warm() {
    if LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("rhc-liq".into())
        .spawn(|| {
            seed_from_disk();
            loop {
                match catch_unwind(AssertUnwindSafe(|| fetch())) {
                    Ok(v) => remember(v),
                    Err(_) => {
                        eprintln!("liq panic");
                        LOOP.store(false, Ordering::SeqCst);
                        return;
                    }
                }
                std::thread::sleep(Duration::from_secs(16));
            }
        });
    if spawned.is_err() {
        LOOP.store(false, Ordering::SeqCst);
    }
}


pub fn snapshot() -> Value {
    warm();
    if let Ok(g) = VIEW.lock() {
        if let Some(v) = g.as_ref() {
            if v.get("ok").and_then(|x| x.as_bool()) == Some(true) || has_graph(v) {
                return v.clone();
            }
        }
    }
    idle("loading")
}


pub(crate) fn has_graph(v: &Value) -> bool {
    v.get("graph")
        .and_then(|g| g.get("nodes"))
        .and_then(|n| n.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}


pub fn snapshot_json() -> String {
    snapshot().to_string()
}

/// Last successful snapshot, if the warm loop has filled the cache.

/// Last successful snapshot, if the warm loop has filled the cache.
pub fn cached_ok() -> Option<Value> {
    let g = CACHE.lock().ok()?;
    let (_, v) = g.as_ref()?;
    if v.get("ok").and_then(|x| x.as_bool()) == Some(true) {
        Some(v.clone())
    } else {
        None
    }
}


pub fn stats_if_ready() -> Option<Value> {
    cached_ok()?.get("stats").cloned()
}


pub fn token_hit(addr: &str) -> Option<Value> {
    let a = addr.to_ascii_lowercase();
    let snap = cached_ok()?;
    snap.get("tokens")?
        .as_array()?
        .iter()
        .find(|t| t.get("address").and_then(|x| x.as_str()) == Some(a.as_str()))
        .cloned()
}


pub fn pools_for(addr: &str) -> Option<Vec<Value>> {
    let a = addr.to_ascii_lowercase();
    let snap = cached_ok()?;
    let rows: Vec<Value> = snap
        .get("pools")?
        .as_array()?
        .iter()
        .filter(|p| {
            side_addr(p, "base").is_some_and(|s| s.eq_ignore_ascii_case(&a))
                || side_addr(p, "quote").is_some_and(|s| s.eq_ignore_ascii_case(&a))
        })
        .cloned()
        .collect();
    if rows.is_empty() {
        None
    } else {
        Some(rows)
    }
}


pub(crate) fn tokens_from(snap: &Value) -> Option<Vec<Value>> {
    let arr = snap.get("tokens")?.as_array()?;
    if arr.is_empty() {
        return None;
    }
    Some(
        arr.iter()
            .take(VIEW_TOKENS)
            .map(|t| {
                json!({
                    "address": t.get("address"),
                    "symbol": t.get("symbol"),
                    "name": t.get("symbol"),
                    "supply": "",
                    "price_usd": t.get("price_usd"),
                    "tvl_usd": t.get("tvl_usd"),
                    "vol24_usd": t.get("vol24_usd"),
                    "degree": t.get("degree"),
                    "hub": t.get("hub"),
                    "kind": t.get("kind"),
                    "source": "rhc-liq",
                })
            })
            .collect(),
    )
}

/// Token rows shaped for Scan's Tokens tab when the history index is quiet.

/// Token rows shaped for Scan's Tokens tab when the history index is quiet.
pub fn token_list() -> Option<Vec<Value>> {
    if let Ok(g) = VIEW.lock() {
        if let Some(v) = g.as_ref() {
            if let Some(rows) = tokens_from(v) {
                return Some(rows);
            }
        }
    }
    let g = CACHE.lock().ok()?;
    tokens_from(&g.as_ref()?.1)
}


pub(crate) fn side_addr<'a>(p: &'a Value, side: &str) -> Option<&'a str> {
    p.get(side)?.get("address")?.as_str()
}


fn scrub_eth_dollar(v: &mut Value) {
    let weth = WETH.to_ascii_lowercase();
    let scrub = |t: &mut Value, keys: &[&str]| {
        let addr = t
            .get("address")
            .or_else(|| t.get("id"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        if !addr.eq_ignore_ascii_case(&weth) {
            return;
        }
        let px = keys
            .iter()
            .find_map(|k| t.get(*k).and_then(|x| x.as_f64()))
            .unwrap_or(0.0);
        if sane_eth_px(px).is_some() {
            return;
        }
        if let Some(obj) = t.as_object_mut() {
            for k in keys {
                obj.insert((*k).into(), json!(0.0));
            }
        }
    };
    if let Some(arr) = v.get_mut("tokens").and_then(|x| x.as_array_mut()) {
        for t in arr {
            scrub(t, &["price_usd"]);
        }
    }
    if let Some(arr) = v.get_mut("pools").and_then(|x| x.as_array_mut()) {
        for p in arr {
            for side in ["base", "quote"] {
                if let Some(t) = p.get_mut(side) {
                    scrub(t, &["price_usd"]);
                }
            }
        }
    }
    if let Some(arr) = v
        .pointer_mut("/graph/nodes")
        .and_then(|x| x.as_array_mut())
    {
        for n in arr {
            scrub(n, &["price", "price_usd"]);
        }
    }
}

pub(crate) fn remember(v: Value) {
    let mut v = v;
    scrub_eth_dollar(&mut v);
    let incoming_ok = v.get("ok").and_then(|x| x.as_bool()) == Some(true);
    {
        let Ok(mut g) = CACHE.lock() else {
            return;
        };
        if !incoming_ok {
            if let Some((_, old)) = g.as_ref() {
                if old.get("ok").and_then(|x| x.as_bool()) == Some(true) {
                    return;
                }
            }
        }
        *g = Some((Instant::now(), v.clone()));
    }
    if incoming_ok {
        save_disk(&v);
    }
    let view = if incoming_ok { slim(&v) } else { v };
    if let Ok(mut g) = VIEW.lock() {
        *g = Some(view);
    }
}


pub(crate) fn fetch() -> Value {
    match fetch_rpc() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("liq rpc: {e}");
            idle("rpc wait")
        }
    }
}

pub(crate) fn idle(why: &str) -> Value {
    json!({
        "ok": false,
        "loading": why == "loading" || why == "rpc wait",
        "error": why,
        "source": "rhc-rpc",
        "chain": CHAIN_NAME,
        "chain_id": CHAIN_ID,
        "stats": { "pools": 0, "tokens": 0, "tvl_usd": 0.0, "vol24_usd": 0.0 },
        "tokens": [],
        "pools": [],
        "graph": { "nodes": [], "edges": [] },
    })
}

pub(crate) fn push_hist(ts: u64, tvl: f64, vol: f64, txns: u64) -> Vec<Value> {
    let Ok(mut h) = HISTORY.lock() else {
        return Vec::new();
    };
    let same = h.last().is_some_and(|last| {
        (last.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0) - tvl).abs() < 1.0
            && last.get("txns24").and_then(|x| x.as_u64()).unwrap_or(0) == txns
    });
    if !same {
        h.push(json!({
            "ts": ts,
            "tvl_usd": tvl,
            "vol24_usd": vol,
            "txns24": txns,
        }));
        if h.len() > 36 {
            h.remove(0);
        }
    }
    h.clone()
}
