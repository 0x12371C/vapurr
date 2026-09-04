//! Robinhood Chain liquidity graph. All Rust. Live RPC only.
//!
//! Tokens are nodes. Pools are edges. Factories on 4663 emit PoolCreated /
//! PairCreated; we crawl those logs, then `balanceOf` the pool for TVL.
//! USDG is $1. Everything else prices through USDG (then WETH). No Gecko,
//! no vis-network, no HTTP market book.
//!
//! The HTTP payload and SVG stay capped so Scan does not freeze.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::rpc::{abi_u128, hex_u64, Rpc};
use crate::{
    CHAIN_ID, CHAIN_NAME, SUSHI_V3_FACTORY, UNI_V2_FACTORY, UNI_V3_FACTORY, USDE, USDG, WETH,
};

mod snapshot;
mod crawl;
mod price;
mod swaps;
mod graph;

pub use snapshot::{warm, snapshot, snapshot_json, cached_ok, stats_if_ready, token_hit, pools_for, token_list};
use crawl::*;
use price::*;
use swaps::*;
use graph::*;


/// Uniswap V3 / Sushi V3 PoolCreated(address,address,uint24,int24,address)
const POOL_CREATED: &str =
    "0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118";
/// Uniswap V2 PairCreated(address,address,address,uint256)
const PAIR_CREATED: &str =
    "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9";
/// Uniswap V2 Swap
const SWAP_V2: &str =
    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
/// Uniswap V3 / Sushi V3 Swap
const SWAP_V3: &str =
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
const SLOT0: &str = "0x3850c7bd";
const VOL_CHUNKS: u64 = 4;

const VIEW_POOLS: usize = 40;
const VIEW_TOKENS: usize = 32;
const VIEW_NODES: usize = 48;
const VIEW_EDGES: usize = 72;
const CHUNK: u64 = 20_000;
const FIRST_SPAN: u64 = 80_000;
const MAX_INDEX: usize = 200;
const MAX_PRICE: usize = 80;

static CACHE: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
static VIEW: Mutex<Option<Value>> = Mutex::new(None);
static LOOP: AtomicBool = AtomicBool::new(false);
static HISTORY: Mutex<Vec<Value>> = Mutex::new(Vec::new());
static SWAPS_DEEP: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use super::*;
    use super::crawl::*;
    use super::graph::*;
    use super::price::*;
    use super::snapshot::*;
    use super::swaps::*;

    #[test]
    fn idle_is_not_a_map() {
        let v = idle("loading");
        assert_eq!(v["ok"], false);
        assert_eq!(v["loading"], true);
        assert!(v["graph"]["nodes"].as_array().unwrap().is_empty());
    }

    #[test]
    fn splits_uniswap_name() {
        let (b, q, f) = split_name("USDG / WETH 0.01%");
        assert_eq!(b, "USDG");
        assert_eq!(q, "WETH");
        assert!(f.contains('%'));
    }

    #[test]
    fn usdg_is_canonical() {
        assert_eq!(canon_sym(USDG, "usd-g"), "USDG");
        assert_eq!(canon_sym(WETH, "weth"), "WETH");
        assert_eq!(token_kind(USDG, "USDG"), "stable");
        assert_eq!(token_kind(WETH, "WETH"), "eth");
        assert_eq!(token_kind("0xabcabcabcabcabcabcabcabcabcabcabcabcabca", "PEPE"), "meme");
        assert!(is_stable(USDG, "USDG"));
        let pool = topic_addr("0x0000000000000000000000005fc5360d0400a0fd4f2af552add042d716f1d168").unwrap();
        assert_eq!(pool, USDG.to_ascii_lowercase());
        assert_eq!(fee_label(3000), "0.30%");
        assert_eq!(
            abi_u128(&json!(
                "0x0000000000000000000000000000000000000000000000000000000000000006"
            )),
            6
        );
        assert_eq!(
            abi_u128(&json!(
                "0x00000000000000000000000000000000000000000000000000000000000f4240"
            )),
            1_000_000
        );
        assert_eq!(known_dec(USDG), Some(6));
        assert_eq!(known_dec(WETH), Some(18));
        let mut h = format!("{:x}", 1u128 << 96);
        while h.len() < 64 {
            h.insert(0, '0');
        }
        let px = slot_price_t1_per_t0(&json!(format!("0x{h}")), 18, 18).unwrap();
        assert!((px - 1.0).abs() < 1e-9, "slot0 1:1 got {px}");
        let neg = "0x".to_string() + &"f".repeat(64);
        assert_eq!(abi_word_signed(&neg, 0), (1, true));
        let pos = "0x".to_string() + &"0".repeat(62) + "64";
        assert_eq!(abi_word_signed(&pos, 0), (0x64, false));
        let (a, b) = sort_pair(USDG, WETH);
        assert!(a < b);
        assert_eq!(a, WETH.to_ascii_lowercase());
    }

    #[test]
    fn parses_v3_pool_created() {
        let log = json!({
            "topics": [
                POOL_CREATED,
                "0x0000000000000000000000005fc5360d0400a0fd4f2af552add042d716f1d168",
                "0x0000000000000000000000000bd7d308f8e1639fab988df18a8011f41eacad73",
                "0x0000000000000000000000000000000000000000000000000000000000000064"
            ],
            "data": "0x000000000000000000000000000000000000000000000000000000000000000100000000000000000000000052e65b17fb6e5ba00ed806f37afcd2daa50271ca"
        });
        let row = parse_created(&log, "uniswap v3").unwrap();
        assert_eq!(row.token0, USDG.to_ascii_lowercase());
        assert_eq!(row.token1, WETH.to_ascii_lowercase());
        assert_eq!(row.fee, 100);
        assert_eq!(row.address, "0x52e65b17fb6e5ba00ed806f37afcd2daa50271ca");
        let p = json!({
            "address": row.address,
            "name": "USDG / WETH 0.01%",
            "dex": "uniswap v3",
            "fee": "0.01%",
            "base": { "address": USDG, "symbol": "USDG", "price_usd": 1.0 },
            "quote": { "address": WETH, "symbol": "WETH", "price_usd": 2400.0 },
            "reserve_usd": 1000.5,
            "vol1_usd": 0.0,
            "vol6_usd": 0.0,
            "vol24_usd": 0.0,
            "mcap_usd": 0.0,
            "change24": "0",
            "buys24": 0,
            "sells24": 0,
            "txns24": 0
        });
        let g = build_graph(vec![p], 1);
        assert_eq!(g["ok"], true);
        assert_eq!(g["source"], "rhc-rpc");
        assert_eq!(g["stats"]["pools"], 1);
        assert_eq!(g["stats"]["hubs"], 2);
        assert_eq!(g["graph"]["edges"].as_array().unwrap().len(), 1);
        let view = slim(&g);
        assert_eq!(view["capped"], true);
        let rows = tokens_from(&g).expect("scan token rows");
        let usdg = rows
            .iter()
            .find(|t| {
                t.get("address")
                    .and_then(|x| x.as_str())
                    .map(|s| s.eq_ignore_ascii_case(USDG))
                    .unwrap_or(false)
            })
            .expect("USDG row");
        assert!(
            usdg.get("holders").is_none() || usdg.get("holders") == Some(&Value::Null),
            "pool degree is not a holder census: {usdg}"
        );
        assert_eq!(usdg["degree"], 1);
        assert_eq!(usdg["source"], "rhc-liq");
        {
            let mut c = CACHE.lock().unwrap();
            *c = Some((Instant::now(), g));
        }
        assert!(token_hit(USDG).is_some());
        assert!(pools_for(USDG).unwrap().len() == 1);
    }

    #[test]
    fn tokens_from_does_not_label_degree_as_holders() {
        let snap = json!({
            "ok": true,
            "tokens": [{
                "address": USDG.to_ascii_lowercase(),
                "symbol": "USDG",
                "degree": 6,
                "price_usd": 1.0,
                "tvl_usd": 1000.0,
                "vol24_usd": 1.0,
                "hub": true,
                "kind": "stable"
            }]
        });
        let rows = tokens_from(&snap).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].get("holders").is_none() || rows[0].get("holders") == Some(&Value::Null),
            "{}",
            rows[0]
        );
        assert_eq!(rows[0]["degree"], 6);
        assert_eq!(rows[0]["symbol"], "USDG");
    }

    #[test]
    fn live_rpc_optional() {
        let rpc = Rpc::liq();
        match rpc.call("eth_blockNumber", json!([])) {
            Ok(v) => {
                let n = hex_u64(&v);
                eprintln!("rpc head {n}");
                if n == 0 {
                    return;
                }
                match rpc.call(
                    "eth_call",
                    json!([{"to": UNI_V2_FACTORY, "data": "0x574f2ba3"}, "latest"]),
                ) {
                    Ok(len) => eprintln!("uni v2 allPairsLength {}", abi_u128(&len)),
                    Err(e) => eprintln!("uni v2 length {e}"),
                }
                match rpc.call(
                    "eth_call",
                    json!([{
                        "to": UNI_V2_FACTORY,
                        "data": format!("0xe6a43905{}{}", pad32(USDG), pad32(WETH))
                    }, "latest"]),
                ) {
                    Ok(v) => eprintln!("getPair USDG/WETH {}", v),
                    Err(e) => eprintln!("getPair {e}"),
                }
                let mut idx = PoolIdx::default();
                seed_hubs(&rpc, &mut idx);
                seed_v2(&rpc, &mut idx);
                eprintln!("hub+v2 pools {}", idx.pools.len());
                for p in idx.pools.iter().take(8) {
                    eprintln!("  {} {} {} {}", p.dex, p.fee, p.token0, p.address);
                }
                let mut rows = price_pools(&rpc, &idx.pools);
                let tvl: f64 = rows
                    .iter()
                    .map(|p| p.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0))
                    .sum();
                eprintln!("priced {} tvl {tvl}", rows.len());
                if let Some(p) = rows.first() {
                    eprintln!("top {}", p);
                }
                let window = fill_swaps(&rpc, &mut rows, n);
                let vol: f64 = rows
                    .iter()
                    .map(|p| p.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0))
                    .sum();
                let tx: u64 = rows
                    .iter()
                    .map(|p| p.get("txns24").and_then(|x| x.as_u64()).unwrap_or(0))
                    .sum();
                eprintln!("swaps window {window}s vol {vol} tx {tx}");
            }
            Err(e) => eprintln!("rpc {e}"),
        }
    }
}
