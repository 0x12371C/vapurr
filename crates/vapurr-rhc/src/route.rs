//! Swap and bridge router.
//!
//! LI.FI lists candidate routers. vapurr scores them on **full output minus
//! gas**. We do not cut the route. Protocol 25 bps buys `$VAPURR`; a small
//! slice is refunded to the user in `$VAPURR`; the rest burns to mint `$PUSD`.
//! A route is payable only after a **real** RPC `eth_call` + `eth_estimateGas`.
//! LI.FI returning a tx is not a simulation.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::{
    CHAIN_ID, NATIVE, PUSD_TOKEN, ROUTE_FEE_BPS, ROUTE_FEE_MINT_SPREAD_BPS, ROUTE_INTEGRATOR,
    HOUSE_REFUND_BPS, ROUTE_REFUND_BPS, STOCKS, TESTNET_CHAIN_ID, TESTNET_PUSD, TESTNET_STOCKS, TESTNET_SWAP,
    TESTNET_USDG, TESTNET_VAPURR, USDG, USDG_DECIMALS, VAPURR_TOKEN, WETH,
};

const LIFI: &str = "https://li.quest/v1";
const QUOTE_ADDR: &str = "0x552008c0f6870c2f77e5cC1d2eb9bdff03e30Ea0";
const AVAX_USDC: &str = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E";
const ETH_USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

#[allow(dead_code)]
struct Cache<T> {
    val: T,
}

#[allow(dead_code)]
static TOKENS: Mutex<Option<Cache<Value>>> = Mutex::new(None);
#[allow(dead_code)]
static TOKEN_LOOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static QUOTE_CACHE: Mutex<Option<(Instant, String, Value)>> = Mutex::new(None);
static GAS_CACHE: Mutex<Option<(Instant, u64, u128)>> = Mutex::new(None);

pub fn scoop(amount: u128, bps: u32) -> u128 {
    amount.saturating_mul(bps as u128) / 10_000
}

pub fn tokens_json(query: &str) -> String {
    let chain = param(query, "chain");
    serde_json::to_string(&tokens(chain.as_deref())).unwrap_or_else(|_| "{}".into())
}

pub fn quote_json(query: &str) -> String {
    if let Some(hit) = cache_get(query) {
        return hit.to_string();
    }
    match quote(query) {
        Ok(v) => {
            cache_put(query, v.clone());
            v.to_string()
        }
        Err(e) => json!({ "ok": false, "error": e, "fee_bps": ROUTE_FEE_BPS }).to_string(),
    }
}

fn cache_key(query: &str) -> String {
    let bits = [
        param(query, "fromChain").unwrap_or_default(),
        param(query, "toChain").unwrap_or_default(),
        param(query, "fromToken").unwrap_or_default().to_ascii_lowercase(),
        param(query, "toToken").unwrap_or_default().to_ascii_lowercase(),
        param(query, "amount").unwrap_or_default(),
        param(query, "fromAddress").unwrap_or_default().to_ascii_lowercase(),
    ];
    bits.join("|")
}

fn cache_get(query: &str) -> Option<Value> {
    let key = cache_key(query);
    let g = QUOTE_CACHE.lock().ok()?;
    let (at, k, v) = g.as_ref()?;
    if k == &key && at.elapsed() < Duration::from_secs(8) {
        Some(v.clone())
    } else {
        None
    }
}

fn cache_put(query: &str, v: Value) {
    if v.get("ok").and_then(|x| x.as_bool()) != Some(true) {
        return;
    }
    // Don't freeze an approve-needed quote — the next poll must see the new allowance.
    if v.get("payable").and_then(|x| x.as_bool()) != Some(true) {
        return;
    }
    if let Ok(mut g) = QUOTE_CACHE.lock() {
        *g = Some((Instant::now(), cache_key(query), v));
    }
}

pub fn tokens(chain: Option<&str>) -> Value {
    let mut out = rail_tokens();
    if let Some(c) = chain.and_then(|s| s.parse::<u64>().ok()) {
        out.retain(|t| t.get("chain_id").and_then(|x| x.as_u64()) == Some(c));
    }
    json!({
        "ok": true,
        "fee_bps": ROUTE_FEE_BPS,
        "fee": "0.25%",
        "refund_bps": ROUTE_REFUND_BPS,
        "integrator": ROUTE_INTEGRATOR,
        "tokens": out,
        "chains": chains(),
    })
}

fn chains() -> Value {
    json!([
        { "id": 4663, "name": "Robinhood Chain", "native": "ETH" },
        { "id": 1, "name": "Ethereum", "native": "ETH" },
        { "id": 43114, "name": "Avalanche", "native": "AVAX" },
        { "id": 8453, "name": "Base", "native": "ETH" },
        { "id": 42161, "name": "Arbitrum", "native": "ETH" }
    ])
}

fn rail_tokens() -> Vec<Value> {
    let mut out = Vec::new();
    push_tok(&mut out, CHAIN_ID, NATIVE, "ETH", "Ether", 18);
    push_tok(&mut out, CHAIN_ID, VAPURR_TOKEN, "VAPURR", "VAPURR", 18);
    push_tok(&mut out, CHAIN_ID, PUSD_TOKEN, "PUSD", "PUSD", 18);
    push_tok(&mut out, CHAIN_ID, USDG, "USDG", "USDG", USDG_DECIMALS as u32);
    for (sym, name, addr) in STOCKS {
        push_tok(&mut out, CHAIN_ID, addr, sym, name, 18);
    }
    push_tok(&mut out, TESTNET_CHAIN_ID, NATIVE, "ETH", "Ether", 18);
    push_tok(&mut out, TESTNET_CHAIN_ID, TESTNET_VAPURR, "VAPURR", "VAPURR", 18);
    push_tok(&mut out, TESTNET_CHAIN_ID, TESTNET_PUSD, "PUSD", "PUSD", 18);
    push_tok(&mut out, TESTNET_CHAIN_ID, TESTNET_USDG, "USDG", "USDG", 6);
    for (sym, addr) in TESTNET_STOCKS {
        push_tok(&mut out, TESTNET_CHAIN_ID, addr, sym, sym, 18);
    }
    push_tok(&mut out, 1, NATIVE, "ETH", "Ether", 18);
    push_tok(&mut out, 1, ETH_USDC, "USDC", "USD Coin", 6);
    push_tok(&mut out, 43114, NATIVE, "AVAX", "Avalanche", 18);
    push_tok(&mut out, 43114, AVAX_USDC, "USDC", "USD Coin", 6);
    push_tok(&mut out, 8453, NATIVE, "ETH", "Ether", 18);
    push_tok(&mut out, 42161, NATIVE, "ETH", "Ether", 18);
    out
}

fn push_tok(
    out: &mut Vec<Value>,
    chain: u64,
    address: &str,
    symbol: &str,
    name: &str,
    decimals: u32,
) {
    if address.is_empty() {
        return;
    }
    out.push(tok(chain, address, symbol, name, decimals));
}

fn tok(chain: u64, address: &str, symbol: &str, name: &str, decimals: u32) -> Value {
    json!({
        "chain_id": chain,
        "address": address,
        "symbol": symbol,
        "name": name,
        "decimals": decimals,
        "native": address.eq_ignore_ascii_case(NATIVE),
    })
}

#[allow(dead_code)]
fn kick_tokens() {
    use std::sync::atomic::Ordering;
    if TOKEN_LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("route-tokens".into())
        .spawn(|| {
            let _ = fetch_lifi_tokens();
        });
}

#[allow(dead_code)]
fn lifi_tokens() -> Value {
    kick_tokens();
    if let Ok(g) = TOKENS.lock() {
        if let Some(c) = g.as_ref() {
            return c.val.clone();
        }
    }
    json!([])
}

#[allow(dead_code)]
fn fetch_lifi_tokens() -> Value {
    let http = match client() {
        Some(c) => c,
        None => return json!([]),
    };
    let v: Value = match http
        .get(format!("{LIFI}/tokens"))
        .query(&[("chains", "4663,43114,1,8453,42161")])
        .send()
        .and_then(|r| r.json())
    {
        Ok(v) => v,
        Err(_) => return json!([]),
    };
    let mut out = Vec::new();
    if let Some(map) = v.get("tokens").and_then(|x| x.as_object()) {
        for (cid, arr) in map {
            let chain: u64 = cid.parse().unwrap_or(0);
            if let Some(list) = arr.as_array() {
                for t in list.iter().take(24) {
                    let addr = t.get("address").and_then(|x| x.as_str()).unwrap_or("");
                    let sym = t.get("symbol").and_then(|x| x.as_str()).unwrap_or("");
                    if addr.is_empty() || sym.is_empty() {
                        continue;
                    }
                    out.push(json!({
                        "chain_id": chain,
                        "address": addr,
                        "symbol": sym,
                        "name": t.get("name").and_then(|x| x.as_str()).unwrap_or(sym),
                        "decimals": t.get("decimals").and_then(|x| x.as_u64()).unwrap_or(18),
                        "native": addr.eq_ignore_ascii_case(NATIVE),
                    }));
                }
            }
        }
    }
    let val = Value::Array(out);
    if let Ok(mut g) = TOKENS.lock() {
        *g = Some(Cache { val: val.clone() });
    }
    val
}

#[derive(Clone)]
struct QuoteReq {
    kind: &'static str,
    from_chain: u64,
    to_chain: u64,
    from_token: String,
    to_token: String,
    from_sym: String,
    to_sym: String,
    from_dec: u32,
    to_dec: u32,
    from_amount: u128,
    from_address: String,
}

#[derive(Clone, Default)]
struct SimReport {
    ran: bool,
    ok: bool,
    source: String,
    rpc: String,
    chain_id: u64,
    from: String,
    to: String,
    gas: u64,
    gas_price: u128,
    revert: String,
    ret: String,
}

#[derive(Clone)]
struct Cand {
    id: String,
    provider: String,
    tool: String,
    hops: Vec<Value>,
    gross_out: u128,
    net_out: u128,
    fee_out: u128,
    to_min_net: u128,
    gas_usd: f64,
    to_usd: f64,
    from_usd: f64,
    duration: u64,
    tx: Value,
    step: Option<Value>,
    sim: SimReport,
}

pub fn net_after_fee(gross: u128) -> (u128, u128) {
    let fee = scoop(gross, ROUTE_FEE_BPS);
    (gross.saturating_sub(fee), fee)
}

/// `bps` of notional, paid in $VAPURR (18 dec). $1 genesis until the live book feeds a px.
pub fn vapurr_refund_wei_bps(bps: u32, from_usd: f64, from_amount: u128, from_dec: u32) -> u128 {
    if from_usd > 0.0 {
        let usd = from_usd * (bps as f64) / 10_000.0;
        if !usd.is_finite() || usd <= 0.0 {
            return 0;
        }
        return (usd * 1_000_000_000_000_000_000.0).round() as u128;
    }
    let as_18 = if from_dec >= 18 {
        from_amount / 10u128.pow(from_dec - 18)
    } else {
        from_amount.saturating_mul(10u128.pow(18 - from_dec))
    };
    scoop(as_18, bps)
}

/// LiFi/integrator path: 5 bps $VAPURR rebate.
pub fn vapurr_refund_wei(from_usd: f64, from_amount: u128, from_dec: u32) -> u128 {
    vapurr_refund_wei_bps(ROUTE_REFUND_BPS, from_usd, from_amount, from_dec)
}

/// User net in output units: full route + $VAPURR refund − gas. The route is not haircut.
pub fn route_score(net_out: u128, gas_out_units: u128) -> i128 {
    net_out as i128 - gas_out_units as i128
}

pub fn refund_out_units(from_usd: f64, to_usd: f64, gross_out: u128) -> u128 {
    if from_usd <= 0.0 || to_usd <= 0.0 || gross_out == 0 {
        return 0;
    }
    let usd = from_usd * (ROUTE_REFUND_BPS as f64) / 10_000.0;
    let u = usd / to_usd * (gross_out as f64);
    if !u.is_finite() || u <= 0.0 {
        0
    } else {
        u.round() as u128
    }
}

pub fn user_score(net_out: u128, refund_units: u128, gas_out_units: u128) -> i128 {
    route_score(net_out.saturating_add(refund_units), gas_out_units)
}

pub fn gas_in_out_units(gas_usd: f64, to_usd: f64, gross_out: u128) -> u128 {
    if gas_usd <= 0.0 || to_usd <= 0.0 || gross_out == 0 {
        return 0;
    }
    let u = (gas_usd / to_usd) * (gross_out as f64);
    if !u.is_finite() || u <= 0.0 {
        0
    } else {
        u.round() as u128
    }
}

fn quote(query: &str) -> Result<Value, String> {
    let t0 = Instant::now();
    let req = parse_req(query)?;
    if let Some(mut house) = house_cand(&req) {
        let bag = house_bag(&req);
        simulate_house(&mut house, &req);
        let mut v = pack_ranked(&req, std::slice::from_ref(&house), None);
        house_pay_flags(&mut v, &req, &bag);
        if let Some(obj) = v.as_object_mut() {
            obj.insert("ms".into(), json!(t0.elapsed().as_millis() as u64));
        }
        return Ok(v);
    }
    if req.from_chain == TESTNET_CHAIN_ID && req.to_chain == TESTNET_CHAIN_ID {
        return Ok(fallback_quote(
            &req,
            "no house book for this pair — $VAPURR / $PUSD only",
        ));
    }
    let amt = req.from_amount.to_string();
    let bridge = req.from_chain != req.to_chain;
    let (quote_res, cheap_res, fast_res) = std::thread::scope(|s| {
        let q = s.spawn(|| {
            lifi_quote(
                req.from_chain,
                req.to_chain,
                &req.from_token,
                &req.to_token,
                &amt,
                &req.from_address,
            )
        });
        let cheap = s.spawn(|| {
            lifi_routes(
                req.from_chain,
                req.to_chain,
                &req.from_token,
                &req.to_token,
                &amt,
                &req.from_address,
                "CHEAPEST",
            )
        });
        let fast = s.spawn(|| {
            if !bridge {
                return Ok(Vec::new());
            }
            lifi_routes(
                req.from_chain,
                req.to_chain,
                &req.from_token,
                &req.to_token,
                &amt,
                &req.from_address,
                "FASTEST",
            )
        });
        (
            q.join().unwrap_or(Err("quote thread".into())),
            cheap.join().unwrap_or(Err("routes thread".into())),
            fast.join().unwrap_or(Err("fast thread".into())),
        )
    });

    let mut cands: Vec<Cand> = Vec::new();
    let mut why = String::new();
    match quote_res {
        Ok(raw) => cands.push(cand_from_lifi_quote(&raw, &req)),
        Err(e) => why = e,
    }
    for (res, tag) in [(cheap_res, "cheap"), (fast_res, "fast")] {
        let _ = tag;
        match res {
            Ok(routes) => {
                for raw in routes {
                    let c = cand_from_lifi_route(&raw, &req);
                    if cands.iter().any(|x| same_cand(x, &c)) {
                        continue;
                    }
                    cands.push(c);
                }
            }
            Err(e) => {
                if why.is_empty() {
                    why = e;
                }
            }
        }
    }
    if cands.is_empty() {
        return Ok(fallback_quote(&req, &why));
    }

    let baseline = cands.first().cloned();
    cands.sort_by_key(|c| std::cmp::Reverse(score_of(c, &req)));
    cands.truncate(5);
    simulate_top(&mut cands, &req.from_address, 3);
    cands.sort_by(|a, b| cmp_best(a, b, &req));
    let mut v = pack_ranked(&req, &cands, baseline.as_ref());
    if let Some(obj) = v.as_object_mut() {
        obj.insert("ms".into(), json!(t0.elapsed().as_millis() as u64));
    }
    Ok(v)
}

fn parse_req(query: &str) -> Result<QuoteReq, String> {
    let from_chain: u64 = param(query, "fromChain")
        .or_else(|| param(query, "chain"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(CHAIN_ID);
    let to_chain: u64 = param(query, "toChain")
        .and_then(|s| s.parse().ok())
        .unwrap_or(from_chain);
    let from_token = param(query, "fromToken").unwrap_or_else(|| NATIVE.to_string());
    let to_token = param(query, "toToken").unwrap_or_else(|| USDG.to_string());
    let amount_raw = param(query, "amount").ok_or("amount")?;
    let from_dec: u32 = param(query, "fromDecimals")
        .and_then(|s| s.parse().ok())
        .unwrap_or(if is_native(&from_token) { 18 } else { 6 });
    let to_dec: u32 = param(query, "toDecimals")
        .and_then(|s| s.parse().ok())
        .unwrap_or(if is_native(&to_token) { 18 } else { 6 });
    let from_sym = param(query, "fromSymbol").unwrap_or_else(|| {
        if is_native(&from_token) {
            "ETH".into()
        } else {
            "TOKEN".into()
        }
    });
    let to_sym = param(query, "toSymbol").unwrap_or_else(|| "USDG".into());
    let from_amount = parse_amount(&amount_raw, from_dec).ok_or("bad amount")?;
    if from_amount == 0 {
        return Err("amount too small".into());
    }
    let from_address = param(query, "fromAddress").unwrap_or_else(|| QUOTE_ADDR.to_string());
    Ok(QuoteReq {
        kind: if from_chain == to_chain {
            "swap"
        } else {
            "bridge"
        },
        from_chain,
        to_chain,
        from_token,
        to_token,
        from_sym,
        to_sym,
        from_dec,
        to_dec,
        from_amount,
        from_address,
    })
}

fn addr_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn is_vapurr(chain: u64, addr: &str) -> bool {
    (chain == TESTNET_CHAIN_ID && addr_eq(addr, TESTNET_VAPURR))
        || (chain == CHAIN_ID && !VAPURR_TOKEN.is_empty() && addr_eq(addr, VAPURR_TOKEN))
}

fn is_pusd(chain: u64, addr: &str) -> bool {
    (chain == TESTNET_CHAIN_ID && addr_eq(addr, TESTNET_PUSD))
        || (chain == CHAIN_ID && !PUSD_TOKEN.is_empty() && addr_eq(addr, PUSD_TOKEN))
}

fn house_swapper(chain: u64) -> Option<&'static str> {
    if chain == TESTNET_CHAIN_ID && !TESTNET_SWAP.is_empty() {
        Some(TESTNET_SWAP)
    } else {
        None
    }
}

fn house_cand(req: &QuoteReq) -> Option<Cand> {
    if req.from_chain != req.to_chain {
        return None;
    }
    let sell_v = is_vapurr(req.from_chain, &req.from_token) && is_pusd(req.to_chain, &req.to_token);
    let sell_p = is_pusd(req.from_chain, &req.from_token) && is_vapurr(req.to_chain, &req.to_token);
    if !sell_v && !sell_p {
        return None;
    }
    let swapper = house_swapper(req.from_chain)?;
    let data = encode_swap_exact(sell_v, req.from_amount, 0);
    let est = req.from_amount.saturating_mul(997) / 1000;
    Some(finish_cand(
        Cand {
            id: "house".into(),
            provider: "vapurr".into(),
            tool: "house".into(),
            hops: vec![json!({
                "tool": "house",
                "name": "House v4",
                "type": "swap",
            })],
            gross_out: 0,
            net_out: 0,
            fee_out: 0,
            to_min_net: 0,
            gas_usd: 0.0,
            to_usd: 0.0,
            from_usd: 0.0,
            duration: 4,
            tx: json!({
                "to": swapper,
                "data": data,
                "value": "0x0",
                "chainId": req.from_chain,
                "from": req.from_address,
            }),
            step: None,
            sim: SimReport::default(),
        },
        est,
        0,
    ))
}

fn encode_swap_exact(sell_v: bool, amt: u128, min_out: u128) -> String {
    let mut d = Vec::with_capacity(100);
    d.extend_from_slice(&[0x67, 0xb7, 0x47, 0x9a]);
    let mut word = [0u8; 32];
    if sell_v {
        word[31] = 1;
    }
    d.extend_from_slice(&word);
    d.extend_from_slice(&u256_be(amt));
    d.extend_from_slice(&u256_be(min_out));
    format!("0x{}", hex::encode(d))
}

const MAX_WORD: &str =
    "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const HOUSE_GAS: u64 = 220_000;

#[derive(Clone, Default)]
struct HouseBag {
    allowance: u128,
    balance: u128,
}

fn keccak(bytes: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    Keccak256::digest(bytes).into()
}

fn abi_addr_word(addr: &str) -> [u8; 32] {
    let mut w = [0u8; 32];
    let h = addr.trim().trim_start_matches("0x").trim_start_matches("0X");
    if let Ok(b) = hex::decode(h) {
        if b.len() <= 20 {
            w[32 - b.len()..].copy_from_slice(&b);
        }
    }
    w
}

fn map_slot(addr: &str, slot: u64) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(&abi_addr_word(addr));
    buf[32..].copy_from_slice(&u256_be(slot as u128));
    keccak(&buf)
}

fn nest_slot(addr: &str, inner: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(&abi_addr_word(addr));
    buf[32..].copy_from_slice(inner);
    keccak(&buf)
}

fn word_hex(w: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(w))
}

fn encode_balance_of(owner: &str) -> String {
    let mut d = Vec::with_capacity(36);
    d.extend_from_slice(&[0x70, 0xa0, 0x82, 0x31]);
    d.extend_from_slice(&abi_addr_word(owner));
    format!("0x{}", hex::encode(d))
}

fn encode_allowance(owner: &str, spender: &str) -> String {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&[0xdd, 0x62, 0xed, 0x3e]);
    d.extend_from_slice(&abi_addr_word(owner));
    d.extend_from_slice(&abi_addr_word(spender));
    format!("0x{}", hex::encode(d))
}

fn encode_approve(spender: &str) -> String {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);
    d.extend_from_slice(&abi_addr_word(spender));
    d.extend_from_slice(&[0xff; 32]);
    format!("0x{}", hex::encode(d))
}

fn token_u128(rpc: &crate::rpc::Rpc, token: &str, data: &str) -> u128 {
    rpc.eth_call(QUOTE_ADDR, Some(token), data)
        .ok()
        .and_then(|r| parse_ret_u128(&r))
        .unwrap_or(0)
}

fn house_bag(req: &QuoteReq) -> HouseBag {
    let Some(rpc_url) = rpc_for(req.from_chain) else {
        return HouseBag::default();
    };
    let Some(swapper) = house_swapper(req.from_chain) else {
        return HouseBag::default();
    };
    let rpc = crate::rpc::Rpc::at_timeout(rpc_url, 6);
    HouseBag {
        balance: token_u128(&rpc, &req.from_token, &encode_balance_of(&req.from_address)),
        allowance: token_u128(
            &rpc,
            &req.from_token,
            &encode_allowance(&req.from_address, swapper),
        ),
    }
}

fn house_override(req: &QuoteReq) -> Option<Value> {
    let swapper = house_swapper(req.from_chain)?;
    let sell_v = is_vapurr(req.from_chain, &req.from_token);
    let (bal_slot, allow_slot) = if sell_v { (1u64, 2u64) } else { (2, 3) };
    let bal = map_slot(&req.from_address, bal_slot);
    let allow = nest_slot(swapper, &map_slot(&req.from_address, allow_slot));
    let token = req.from_token.to_ascii_lowercase();
    Some(json!({
        token: {
            "stateDiff": {
                word_hex(&bal): MAX_WORD,
                word_hex(&allow): MAX_WORD,
            }
        }
    }))
}

fn set_tx_data(c: &mut Cand, data: String) {
    if let Some(obj) = c.tx.as_object_mut() {
        obj.insert("data".into(), json!(data));
    }
}

fn simulate_house(c: &mut Cand, req: &QuoteReq) {
    let sell_v = is_vapurr(req.from_chain, &req.from_token);
    set_tx_data(c, encode_swap_exact(sell_v, req.from_amount, 0));
    let ov = house_override(req);
    c.sim = rpc_sim_state(c, &req.from_address, ov.as_ref());
    if c.sim.ok {
        if c.sim.gas == 0 {
            c.sim.gas = HOUSE_GAS;
        }
        if let Some(out) = parse_ret_u128(&c.sim.ret) {
            if out > 0 {
                c.gross_out = out;
                c.net_out = out;
                c.to_min_net = out.saturating_mul(99) / 100;
            }
        }
        set_tx_data(
            c,
            encode_swap_exact(sell_v, req.from_amount, c.to_min_net),
        );
    } else if !c.sim.ran {
        c.sim.ran = true;
        c.sim.ok = false;
        if c.sim.revert.is_empty() {
            c.sim.revert = "house sim did not run".into();
        }
    }
}

fn real_wallet(addr: &str) -> bool {
    let t = addr.trim();
    !t.is_empty() && !addr_eq(t, QUOTE_ADDR) && t.len() >= 42
}

fn house_pay_flags(v: &mut Value, req: &QuoteReq, bag: &HouseBag) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    let sim_ok = obj
        .get("sim")
        .and_then(|s| s.get("ok"))
        .and_then(|x| x.as_bool())
        == Some(true);
    let have_wallet = real_wallet(&req.from_address);
    let funded = bag.balance >= req.from_amount;
    let needs_approve = bag.allowance < req.from_amount;
    let payable = sim_ok && have_wallet && funded && !needs_approve;
    obj.insert("payable".into(), json!(payable));
    obj.insert("funded".into(), json!(funded));
    obj.insert(
        "needs_approve".into(),
        json!(sim_ok && have_wallet && funded && needs_approve),
    );
    if sim_ok && have_wallet && needs_approve {
        if let Some(swapper) = house_swapper(req.from_chain) {
            obj.insert(
                "approve".into(),
                json!({
                    "to": req.from_token,
                    "spender": swapper,
                    "data": encode_approve(swapper),
                    "chainId": req.from_chain,
                    "value": "0x0",
                }),
            );
        }
    }
    let sym = req.from_sym.trim_start_matches('$');
    let note = if payable {
        "House $VAPURR / $PUSD. 0.30% fee + 0.03% $VAPURR refund. This device signs.".to_string()
    } else if !sim_ok {
        obj.get("sim")
            .and_then(|s| s.get("revert"))
            .and_then(|x| x.as_str())
            .map(|r| format!("Simulation reverted: {r}"))
            .unwrap_or_else(|| "Simulation reverted.".into())
    } else if !have_wallet {
        "Simulated on the house book. Unlock this device to sign.".into()
    } else if !funded {
        format!("Simulated. Not enough ${sym} on this device.")
    } else if needs_approve {
        format!("Simulated. Approve ${sym} then swap.")
    } else {
        obj.get("note")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    obj.insert("note".into(), json!(note));
}

fn u256_be(n: u128) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[16..].copy_from_slice(&n.to_be_bytes());
    w
}

fn parse_ret_u128(ret: &str) -> Option<u128> {
    let s = ret.trim().trim_start_matches("0x");
    if s.is_empty() {
        return None;
    }
    let take = if s.len() > 32 { &s[s.len() - 32..] } else { s };
    u128::from_str_radix(take, 16).ok()
}

fn score_of(c: &Cand, req: &QuoteReq) -> i128 {
    user_score(
        c.net_out,
        refund_out_units(c.from_usd, c.to_usd, c.gross_out),
        gas_units_of(c, req),
    )
}

fn gas_units_of(c: &Cand, req: &QuoteReq) -> u128 {
    let eth_out = is_native(&req.to_token) || req.to_token.eq_ignore_ascii_case(WETH);
    if eth_out && c.sim.ok && c.sim.gas > 0 && c.sim.gas_price > 0 {
        return (c.sim.gas as u128).saturating_mul(c.sim.gas_price);
    }
    gas_in_out_units(c.gas_usd, c.to_usd, c.gross_out)
}

fn cmp_best(a: &Cand, b: &Cand, req: &QuoteReq) -> std::cmp::Ordering {
    match (a.sim.ok, b.sim.ok) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => score_of(b, req).cmp(&score_of(a, req)),
    }
}

fn pick_best<'a>(cands: &'a [Cand], req: &QuoteReq) -> Option<&'a Cand> {
    if cands.is_empty() {
        return None;
    }
    let mut best = 0;
    for i in 1..cands.len() {
        if cmp_best(&cands[i], &cands[best], req) == std::cmp::Ordering::Less {
            best = i;
        }
    }
    Some(&cands[best])
}

fn same_cand(a: &Cand, b: &Cand) -> bool {
    a.tool == b.tool && a.gross_out == b.gross_out
}

fn finish_cand(
    mut c: Cand,
    gross: u128,
    to_min: u128,
) -> Cand {
    let fee = scoop(gross, ROUTE_FEE_BPS);
    c.gross_out = gross;
    c.net_out = gross;
    c.fee_out = fee;
    c.to_min_net = to_min;
    c
}

fn cand_from_lifi_quote(raw: &Value, _req: &QuoteReq) -> Cand {
    let est = raw.get("estimate").cloned().unwrap_or(Value::Null);
    let gross = parse_u128(
        est.get("toAmount")
            .and_then(|x| x.as_str())
            .unwrap_or("0"),
    );
    let to_min = parse_u128(
        est.get("toAmountMin")
            .and_then(|x| x.as_str())
            .unwrap_or("0"),
    );
    let tool = raw
        .get("tool")
        .and_then(|x| x.as_str())
        .unwrap_or("lifi")
        .to_string();
    let tx = raw
        .get("transactionRequest")
        .cloned()
        .unwrap_or(Value::Null);
    finish_cand(
        Cand {
            id: raw
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("quote")
                .into(),
            provider: "LI.FI".into(),
            tool: tool.clone(),
            hops: hops_of(raw),
            gross_out: 0,
            net_out: 0,
            fee_out: 0,
            to_min_net: 0,
            gas_usd: usd_of(&est, "gasCosts"),
            to_usd: num_of(&est, "toAmountUSD"),
            from_usd: num_of(&est, "fromAmountUSD"),
            duration: est
                .get("executionDuration")
                .and_then(|x| x.as_u64())
                .or_else(|| est.get("executionDuration").and_then(|x| x.as_f64()).map(|n| n as u64))
                .unwrap_or(0),
            tx,
            step: None,
            sim: SimReport::default(),
        },
        gross,
        if to_min == 0 { gross } else { to_min },
    )
}

fn cand_from_lifi_route(raw: &Value, _req: &QuoteReq) -> Cand {
    let gross = parse_u128(raw.get("toAmount").and_then(|x| x.as_str()).unwrap_or("0"));
    let to_min = parse_u128(raw.get("toAmountMin").and_then(|x| x.as_str()).unwrap_or("0"));
    let steps = raw.get("steps").and_then(|x| x.as_array());
    let first = steps.and_then(|s| s.first()).cloned();
    let tool = first
        .as_ref()
        .and_then(|s| s.get("tool").and_then(|x| x.as_str()))
        .unwrap_or("lifi")
        .to_string();
    let hops = hops_from_steps(steps);
    let gas_usd = raw
        .get("gasCostUSD")
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    finish_cand(
        Cand {
            id: raw.get("id").and_then(|x| x.as_str()).unwrap_or("route").into(),
            provider: "LI.FI".into(),
            tool,
            hops,
            gross_out: 0,
            net_out: 0,
            fee_out: 0,
            to_min_net: 0,
            gas_usd,
            to_usd: raw
                .get("toAmountUSD")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            from_usd: raw
                .get("fromAmountUSD")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            duration: steps
                .map(|ss| {
                    ss.iter()
                        .filter_map(|s| {
                            s.get("estimate")
                                .and_then(|e| e.get("executionDuration"))
                                .and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|n| n as u64)))
                        })
                        .sum()
                })
                .unwrap_or(0),
            tx: Value::Null,
            step: first,
            sim: SimReport::default(),
        },
        gross,
        if to_min == 0 { gross } else { to_min },
    )
}

fn simulate_top(cands: &mut Vec<Cand>, from: &str, n: usize) {
    let n = n.min(cands.len());
    if n == 0 {
        return;
    }
    let from = from.to_string();
    let chunk: Vec<Cand> = cands.iter().take(n).cloned().collect();
    let done: Vec<Cand> = std::thread::scope(|s| {
        let handles: Vec<_> = chunk
            .into_iter()
            .map(|mut c| {
                let from = from.as_str();
                s.spawn(move || {
                    simulate_cand(&mut c, from);
                    c
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .collect()
    });
    for (i, c) in done.into_iter().enumerate() {
        if i < cands.len() {
            cands[i] = c;
        }
    }
}

fn simulate_cand(c: &mut Cand, from: &str) {
    if !c.tx.is_object() {
        if let Some(step) = c.step.clone() {
            match lifi_step_tx(&step) {
                Ok(filled) => {
                    if let Some(tx) = filled.get("transactionRequest").cloned() {
                        if tx.is_object() {
                            c.tx = tx;
                        }
                    }
                    if !c.tx.is_object() {
                        c.sim.ran = false;
                        c.sim.revert = "step had no tx".into();
                    }
                }
                Err(e) => {
                    c.sim.ran = false;
                    c.sim.revert = e;
                }
            }
        }
    }
    if !c.tx.is_object() {
        return;
    }
    c.sim = rpc_sim(c, from);
}

fn rpc_sim(c: &Cand, from: &str) -> SimReport {
    rpc_sim_state(c, from, None)
}

fn rpc_sim_state(c: &Cand, from: &str, state: Option<&Value>) -> SimReport {
    let mut s = SimReport {
        from: from.to_string(),
        source: "rpc".into(),
        ..SimReport::default()
    };
    let chain = c
        .tx
        .get("chainId")
        .and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| parse_chain(s))));
    let Some(chain) = chain else {
        s.revert = "tx has no chainId".into();
        return s;
    };
    s.chain_id = chain;
    let Some(rpc) = rpc_for(chain) else {
        s.revert = format!("no rpc for chain {chain}");
        return s;
    };
    s.rpc = rpc.into();
    let to = c.tx.get("to").and_then(|x| x.as_str());
    s.to = to.unwrap_or("").into();
    let data = c.tx.get("data").and_then(|x| x.as_str()).unwrap_or("0x");
    let value = c.tx.get("value").and_then(|x| x.as_str());
    s.ran = true;
    let wei = parse_wei(value.unwrap_or("0x0"));
    let from_s = from.to_string();
    let to_s = to.map(|x| x.to_string());
    let data_s = data.to_string();
    let val_s = value.map(|x| x.to_string());
    let state_call = state.cloned();
    let state_est = state.cloned();
    let (call_res, gas_res) = std::thread::scope(|sc| {
        let call = sc.spawn(move || {
            crate::rpc::Rpc::at_timeout(rpc, 6).eth_call_tx_state(
                &from_s,
                to_s.as_deref(),
                &data_s,
                val_s.as_deref(),
                state_call.as_ref(),
            )
        });
        let from_g = from.to_string();
        let to_g = to.map(|x| x.to_string());
        let data_g = data.to_string();
        let est = sc.spawn(move || {
            crate::rpc::Rpc::at_timeout(rpc, 6).eth_estimate_gas_value_state(
                &from_g,
                to_g.as_deref(),
                &data_g,
                wei,
                state_est.as_ref(),
            )
        });
        (call.join(), est.join())
    });
    match call_res {
        Ok(Ok(ret)) => {
            s.ret = ret;
            s.ok = true;
            if let Ok(Ok(g)) = gas_res {
                s.gas = g;
            }
            s.gas_price = gas_price_cached(chain, rpc);
        }
        Ok(Err(e)) => {
            s.ok = false;
            s.revert = decode_revert(&e.to_string());
        }
        Err(_) => {
            s.ok = false;
            s.revert = decode_revert("eth_call failed");
        }
    }
    s
}

fn gas_price_cached(chain: u64, rpc: &str) -> u128 {
    if let Ok(g) = GAS_CACHE.lock() {
        if let Some((at, cid, px)) = g.as_ref() {
            if *cid == chain && at.elapsed() < Duration::from_secs(12) {
                return *px;
            }
        }
    }
    let px = crate::rpc::Rpc::at_timeout(rpc, 4)
        .eth_gas_price()
        .unwrap_or(0);
    if px > 0 {
        if let Ok(mut g) = GAS_CACHE.lock() {
            *g = Some((Instant::now(), chain, px));
        }
    }
    px
}

pub fn decode_revert(err: &str) -> String {
    if let Some(s) = abi_error_string(err) {
        return s;
    }
    let t = err
        .replace("execution reverted: ", "")
        .replace("execution reverted", "");
    let t = t.trim().trim_matches('"').trim();
    if t.is_empty() {
        err.chars().take(160).collect()
    } else {
        t.chars().take(160).collect()
    }
}

fn abi_error_string(err: &str) -> Option<String> {
    let i = err.find("08c379a0")?;
    let hex = err[i..].chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>();
    if hex.len() < 8 + 64 + 64 {
        return None;
    }
    let body = &hex[8..];
    let len = u64::from_str_radix(body.get(64..128)?, 16).ok()? as usize;
    if len == 0 || len > 256 {
        return None;
    }
    let data = body.get(128..)?;
    let need = len.saturating_mul(2);
    if data.len() < need {
        return None;
    }
    let mut bytes = Vec::with_capacity(len);
    let mut k = 0;
    while k + 1 < need {
        bytes.push(u8::from_str_radix(&data[k..k + 2], 16).ok()?);
        k += 2;
    }
    let s = String::from_utf8_lossy(&bytes).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn clip_hex(s: &str, keep: usize) -> String {
    let t = s.trim();
    if t.len() <= keep + 2 {
        return t.to_string();
    }
    format!("{}…", &t[..keep.min(t.len())])
}

fn parse_wei(s: &str) -> u128 {
    let t = s.trim();
    if t.is_empty() || t == "0x" || t == "0x0" || t == "0" {
        return 0;
    }
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u128::from_str_radix(h, 16).unwrap_or(0)
    } else {
        t.parse().unwrap_or(0)
    }
}

fn parse_chain(s: &str) -> Option<u64> {
    let t = s.trim().trim_start_matches("0x");
    if s.trim().starts_with("0x") {
        u64::from_str_radix(t, 16).ok()
    } else {
        t.parse().ok()
    }
}

fn rpc_for(chain: u64) -> Option<&'static str> {
    crate::rpc_http(chain)
}

pub fn impact_pct(from_usd: f64, to_usd: f64) -> String {
    if from_usd <= 0.0 || to_usd <= 0.0 {
        return String::new();
    }
    let p = ((from_usd - to_usd) / from_usd) * 100.0;
    if !p.is_finite() {
        return String::new();
    }
    if p.abs() < 0.005 {
        return "~0%".into();
    }
    format!("{p:.2}%")
}

fn house_allowance_block(revert: &str) -> bool {
    let r = revert.to_ascii_uppercase();
    r.contains("PULL")
        || r.contains("ALLOWANCE")
        || r.contains("TRANSFERFROM")
        || r.contains("INSUFFICIENT")
}

fn pack_ranked(req: &QuoteReq, cands: &[Cand], baseline: Option<&Cand>) -> Value {
    let winner = &cands[0];
    let house = winner.tool == "house";
    let payable = if house {
        winner.tx.is_object() && (winner.sim.ok || house_allowance_block(&winner.sim.revert))
    } else {
        winner.sim.ok && winner.tx.is_object()
    };
    let fee = if house {
        json!({
            "bps": 30,
            "label": "0.30% house book. No LI.FI cut.",
        })
    } else {
        fee_plan(req, winner)
    };
    let refund_bps = if house { HOUSE_REFUND_BPS } else { ROUTE_REFUND_BPS };
    let refund = refund_plan_at(req, winner, refund_bps);
    let refund_disp = refund
        .get("display")
        .and_then(|x| x.as_str())
        .unwrap_or("0");
    let alts: Vec<Value> = cands
        .iter()
        .skip(1)
        .take(4)
        .map(|c| {
            json!({
                "id": c.id,
                "tool": c.tool,
                "hops": c.hops,
                "net_out": c.net_out.to_string(),
                "net_display": fmt_units(&c.net_out.to_string(), req.to_dec),
                "score": score_of(c, req).to_string(),
                "gas_usd": if c.gas_usd > 0.0 { format!("${:.2}", c.gas_usd) } else { String::new() },
                "sim_ok": c.sim.ok,
                "sim_ran": c.sim.ran,
                "revert": c.sim.revert,
                "duration": c.duration,
            })
        })
        .collect();
    json!({
        "ok": true,
        "kind": req.kind,
        "provider": winner.provider,
        "fee_bps": ROUTE_FEE_BPS,
        "fee": "0.25%",
        "refund_bps": refund_bps,
        "integrator": ROUTE_INTEGRATOR,
        "from_chain": req.from_chain,
        "to_chain": req.to_chain,
        "from_symbol": req.from_sym,
        "to_symbol": req.to_sym,
        "from_amount": req.from_amount.to_string(),
        "from_display": fmt_units(&req.from_amount.to_string(), req.from_dec),
        "to_amount": winner.net_out.to_string(),
        "to_min": winner.to_min_net.to_string(),
        "to_display": fmt_units(&winner.net_out.to_string(), req.to_dec),
        "to_min_display": fmt_units(&winner.to_min_net.to_string(), req.to_dec),
        "impact": impact_pct(winner.from_usd, winner.to_usd),
        "slippage": "0.50%",
        "gross_out": winner.gross_out.to_string(),
        "duration": winner.duration,
        "fee_usd": if winner.from_usd > 0.0 {
            format!("${:.2}", winner.from_usd * (ROUTE_FEE_BPS as f64) / 10_000.0)
        } else {
            String::new()
        },
        "gas_usd": if winner.gas_usd > 0.0 { format!("${:.2}", winner.gas_usd) } else { String::new() },
        "hops": winner.hops,
        "tx": winner.tx,
        "tool": winner.tool,
        "score": score_of(winner, req).to_string(),
        "best": beat_json(req, winner, cands, baseline),
        "payable": payable,
        "simulated": winner.sim.ok,
        "sim": sim_json(&winner.sim),
        "trace": build_trace(req, winner, refund_disp),
        "refund": refund,
        "fee_sink": fee,
        "routes": alts,
        "note": if house && payable {
            "House $VAPURR / $PUSD. 0.30% fee + 0.03% $VAPURR refund. This device signs.".to_string()
        } else if payable {
            "RPC simulated. Full route. Small $VAPURR refund. Remainder of 0.25% burns to $PUSD.".to_string()
        } else if !winner.sim.ran {
            "No RPC simulation yet. We will not let this pay.".to_string()
        } else {
            format!("Simulation reverted: {}", winner.sim.revert)
        },
    })
}

fn sim_json(s: &SimReport) -> Value {
    let gas_wei = (s.gas as u128).saturating_mul(s.gas_price);
    json!({
        "ok": s.ok,
        "ran": s.ran,
        "source": s.source,
        "rpc": s.rpc,
        "chain_id": s.chain_id,
        "from": s.from,
        "to": s.to,
        "gas": s.gas,
        "gas_price": s.gas_price.to_string(),
        "gas_eth": if gas_wei > 0 { fmt_units(&gas_wei.to_string(), 18) } else { String::new() },
        "revert": s.revert,
        "return": clip_hex(&s.ret, 18),
        "label": if !s.ran {
            "not simulated"
        } else if s.ok {
            "rpc call ok"
        } else {
            "rpc revert"
        },
    })
}

fn build_trace(req: &QuoteReq, c: &Cand, refund_disp: &str) -> Vec<Value> {
    let swap = if !c.sim.ran {
        "wait"
    } else if c.sim.ok {
        "ok"
    } else {
        "fail"
    };
    let after = if c.sim.ok { "ok" } else { "held" };
    let mut nodes = vec![json!({
        "kind": "in",
        "label": "You pay",
        "value": format!("{} {}", fmt_units(&req.from_amount.to_string(), req.from_dec), req.from_sym),
        "state": "ok",
    })];
    if c.hops.is_empty() {
        nodes.push(json!({
            "kind": "swap",
            "label": c.tool,
            "value": if c.sim.gas > 0 { format!("{} gas", c.sim.gas) } else { String::new() },
            "state": swap,
        }));
    } else {
        for (i, h) in c.hops.iter().enumerate() {
            let last = i + 1 == c.hops.len();
            nodes.push(json!({
                "kind": h.get("type").and_then(|x| x.as_str()).unwrap_or("swap"),
                "label": h.get("name").and_then(|x| x.as_str()).unwrap_or(&c.tool),
                "value": if last && c.sim.gas > 0 { format!("{} gas", c.sim.gas) } else { String::new() },
                "state": swap,
            }));
        }
    }
    nodes.push(json!({
        "kind": "out",
        "label": "You get",
        "value": format!("{} {}", fmt_units(&c.net_out.to_string(), req.to_dec), req.to_sym),
        "state": swap,
    }));
    nodes.push(json!({
        "kind": "refund",
        "label": "$VAPURR refund",
        "value": format!("+{refund_disp}"),
        "state": after,
    }));
    nodes.push(json!({
        "kind": "burn",
        "label": "Rest → $PUSD",
        "value": "burn",
        "state": after,
    }));
    nodes
}

fn beat_json(req: &QuoteReq, winner: &Cand, cands: &[Cand], baseline: Option<&Cand>) -> Value {
    let n = cands.len();
    let simmed = cands.iter().filter(|c| c.sim.ran).count();
    let sim_ok = cands.iter().filter(|c| c.sim.ok).count();
    let extra = baseline
        .filter(|b| !(b.tool == winner.tool && b.gross_out == winner.gross_out))
        .map(|b| winner.net_out.saturating_sub(b.net_out))
        .unwrap_or(0);
    json!({
        "of": n,
        "simulated": simmed,
        "sim_ok": sim_ok,
        "score": score_of(winner, req).to_string(),
        "vs_tool": baseline.map(|b| b.tool.clone()).unwrap_or_default(),
        "extra_out": extra.to_string(),
        "extra_display": fmt_units(&extra.to_string(), req.to_dec),
        "refund_display": fmt_units(
            &vapurr_refund_wei_bps(
                if winner.tool == "house" { HOUSE_REFUND_BPS } else { ROUTE_REFUND_BPS },
                winner.from_usd,
                req.from_amount,
                req.from_dec,
            )
            .to_string(),
            18
        ),
        "why": if winner.sim.ok {
            "Best user net among RPC-simulated routes. Full out + $VAPURR refund − gas."
        } else {
            "No RPC-passing route yet. Ranking is quote-only."
        },
    })
}

fn refund_plan(req: &QuoteReq, c: &Cand) -> Value {
    refund_plan_at(req, c, ROUTE_REFUND_BPS)
}

fn refund_plan_at(req: &QuoteReq, c: &Cand, bps: u32) -> Value {
    let wei = vapurr_refund_wei_bps(bps, c.from_usd, req.from_amount, req.from_dec);
    let pct = format!("{:.2}%", bps as f64 / 100.0);
    json!({
        "bps": bps,
        "asset": "VAPURR",
        "decimals": 18,
        "amount": wei.to_string(),
        "display": fmt_units(&wei.to_string(), 18),
        "label": format!("{pct} $VAPURR refund — the route is not cut"),
    })
}

fn fee_plan(req: &QuoteReq, c: &Cand) -> Value {
    let _ = req;
    let fee_usd = if c.from_usd > 0.0 {
        c.from_usd * (ROUTE_FEE_BPS as f64) / 10_000.0
    } else {
        0.0
    };
    let refund_usd = if c.from_usd > 0.0 {
        c.from_usd * (ROUTE_REFUND_BPS as f64) / 10_000.0
    } else {
        0.0
    };
    let burn_usd = (fee_usd - refund_usd).max(0.0);
    let keep = 1.0 - (ROUTE_FEE_MINT_SPREAD_BPS as f64) / 10_000.0;
    let pusd = burn_usd * keep;
    json!({
        "bps": ROUTE_FEE_BPS,
        "action": "buy_vapurr_refund_and_burn_mint_pusd",
        "fee_usd": if fee_usd > 0.0 { format!("{fee_usd:.4}") } else { String::new() },
        "refund_usd": if refund_usd > 0.0 { format!("{refund_usd:.4}") } else { String::new() },
        "burn_usd": if burn_usd > 0.0 { format!("{burn_usd:.4}") } else { String::new() },
        "pusd_mint": if pusd > 0.0 { format!("{pusd:.4}") } else { String::new() },
        "spread_bps": ROUTE_FEE_MINT_SPREAD_BPS,
        "label": "0.25% buys $VAPURR. Small refund to you. Rest burns to mint $PUSD.",
    })
}

fn lifi_quote(
    from_chain: u64,
    to_chain: u64,
    from_token: &str,
    to_token: &str,
    from_amount: &str,
    from_address: &str,
) -> Result<Value, String> {
    let http = client().ok_or("http")?;
    let resp = http
        .get(format!("{LIFI}/quote"))
        .query(&[
            ("fromChain", from_chain.to_string()),
            ("toChain", to_chain.to_string()),
            ("fromToken", from_token.to_string()),
            ("toToken", to_token.to_string()),
            ("fromAmount", from_amount.to_string()),
            ("fromAddress", from_address.to_string()),
            ("integrator", ROUTE_INTEGRATOR.to_string()),
            ("slippage", "0.005".into()),
            ("skipSimulation", "true".into()),
        ])
        .send()
        .map_err(|_| "lifi transport".to_string())?;
    let status = resp.status();
    let v: Value = resp.json().map_err(|_| "lifi decode".to_string())?;
    if !status.is_success() {
        let msg = v
            .get("message")
            .or_else(|| v.get("error"))
            .and_then(|x| x.as_str())
            .unwrap_or("no route");
        return Err(msg.into());
    }
    if v.get("estimate").is_none() && v.get("action").is_none() {
        return Err("no route".into());
    }
    Ok(v)
}

fn lifi_routes(
    from_chain: u64,
    to_chain: u64,
    from_token: &str,
    to_token: &str,
    from_amount: &str,
    from_address: &str,
    order: &str,
) -> Result<Vec<Value>, String> {
    let http = client().ok_or("http")?;
    let body = json!({
        "fromChainId": from_chain,
        "toChainId": to_chain,
        "fromTokenAddress": from_token,
        "toTokenAddress": to_token,
        "fromAmount": from_amount,
        "fromAddress": from_address,
        "options": {
            "integrator": ROUTE_INTEGRATOR,
            "slippage": 0.005,
            "order": order,
            "allowSwitchChain": from_chain != to_chain,
            "maxPriceImpact": 0.15,
            "timing": {
                "routeTimingStrategies": [{
                    "strategy": "minWaitTime",
                    "minWaitTimeMs": 350,
                    "startingExpectedResults": 3,
                    "reduceEveryMs": 150
                }]
            }
        }
    });
    let resp = http
        .post(format!("{LIFI}/advanced/routes"))
        .json(&body)
        .send()
        .map_err(|_| "lifi routes transport".to_string())?;
    let status = resp.status();
    let v: Value = resp.json().map_err(|_| "lifi routes decode".to_string())?;
    if !status.is_success() {
        let msg = v
            .get("message")
            .or_else(|| v.get("error"))
            .and_then(|x| x.as_str())
            .unwrap_or("no routes");
        return Err(msg.into());
    }
    Ok(v.get("routes")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default())
}

fn lifi_step_tx(step: &Value) -> Result<Value, String> {
    let http = client().ok_or("http")?;
    let resp = http
        .post(format!("{LIFI}/advanced/stepTransaction"))
        .json(&json!({ "step": step }))
        .send()
        .map_err(|_| "lifi step transport".to_string())?;
    let status = resp.status();
    let v: Value = resp.json().map_err(|_| "lifi step decode".to_string())?;
    if !status.is_success() {
        let msg = v
            .get("message")
            .or_else(|| v.get("error"))
            .and_then(|x| x.as_str())
            .unwrap_or("step failed");
        return Err(msg.into());
    }
    Ok(v)
}

fn fallback_quote(req: &QuoteReq, why: &str) -> Value {
    let out = if req.from_dec == req.to_dec {
        req.from_amount
    } else if req.from_dec > req.to_dec {
        req.from_amount / 10u128.pow(req.from_dec - req.to_dec)
    } else {
        req.from_amount.saturating_mul(10u128.pow(req.to_dec - req.from_dec))
    };
    let dummy = Cand {
        id: "est".into(),
        provider: "vapurr".into(),
        tool: "vapurr".into(),
        hops: vec![],
        gross_out: out,
        net_out: out,
        fee_out: scoop(out, ROUTE_FEE_BPS),
        to_min_net: out,
        gas_usd: 0.0,
        to_usd: 0.0,
        from_usd: 0.0,
        duration: 0,
        tx: Value::Null,
        step: None,
        sim: SimReport {
            ran: true,
            ok: false,
            revert: why.into(),
            source: "none".into(),
            ..SimReport::default()
        },
    };
    json!({
        "ok": true,
        "kind": req.kind,
        "provider": "vapurr",
        "estimate": true,
        "payable": false,
        "simulated": false,
        "sim": sim_json(&dummy.sim),
        "error": why,
        "fee_bps": ROUTE_FEE_BPS,
        "fee": "0.25%",
        "refund_bps": ROUTE_REFUND_BPS,
        "integrator": ROUTE_INTEGRATOR,
        "from_chain": req.from_chain,
        "to_chain": req.to_chain,
        "from_symbol": req.from_sym,
        "to_symbol": req.to_sym,
        "from_amount": req.from_amount.to_string(),
        "from_display": fmt_units(&req.from_amount.to_string(), req.from_dec),
        "to_amount": out.to_string(),
        "to_min": out.to_string(),
        "to_display": fmt_units(&out.to_string(), req.to_dec),
        "to_min_display": fmt_units(&out.to_string(), req.to_dec),
        "duration": 0,
        "fee_usd": "",
        "gas_usd": "",
        "hops": [],
        "tx": Value::Null,
        "tool": "vapurr",
        "refund": refund_plan(req, &dummy),
        "fee_sink": fee_plan(req, &dummy),
        "routes": [],
        "note": format!("No simulated route ({why}). Route is not cut. Small $VAPURR refund when a path is live."),
    })
}

fn hops_of(raw: &Value) -> Vec<Value> {
    let mut hops = Vec::new();
    if let Some(steps) = raw.get("includedSteps").and_then(|x| x.as_array()) {
        hops.extend(hops_from_steps(Some(steps)));
    }
    if hops.is_empty() {
        hops.extend(hops_from_steps(raw.get("steps").and_then(|x| x.as_array())));
    }
    if hops.is_empty() {
        let tool = raw.get("tool").and_then(|x| x.as_str()).unwrap_or("lifi");
        hops.push(json!({ "tool": tool, "name": tool, "type": "swap" }));
    }
    hops
}

fn hops_from_steps(steps: Option<&Vec<Value>>) -> Vec<Value> {
    let Some(steps) = steps else {
        return Vec::new();
    };
    let mut hops = Vec::new();
    for s in steps {
        let tool = s.get("tool").and_then(|x| x.as_str()).unwrap_or("step");
        let name = s
            .get("toolDetails")
            .and_then(|x| x.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or(tool);
        let kind = s.get("type").and_then(|x| x.as_str()).unwrap_or("swap");
        hops.push(json!({ "tool": tool, "name": name, "type": kind }));
        if let Some(inner) = s.get("includedSteps").and_then(|x| x.as_array()) {
            for t in inner {
                let tool = t.get("tool").and_then(|x| x.as_str()).unwrap_or("step");
                let name = t
                    .get("toolDetails")
                    .and_then(|x| x.get("name"))
                    .and_then(|x| x.as_str())
                    .unwrap_or(tool);
                let kind = t.get("type").and_then(|x| x.as_str()).unwrap_or("swap");
                hops.push(json!({ "tool": tool, "name": name, "type": kind }));
            }
        }
    }
    hops
}

fn usd_of(est: &Value, key: &str) -> f64 {
    let mut n = 0.0;
    if let Some(arr) = est.get(key).and_then(|x| x.as_array()) {
        for f in arr {
            if let Some(s) = f.get("amountUSD").and_then(|x| x.as_str()) {
                n += s.parse::<f64>().unwrap_or(0.0);
            }
        }
    }
    n
}

fn num_of(v: &Value, key: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| v.get(key).and_then(|x| x.as_f64()))
        .unwrap_or(0.0)
}

fn parse_u128(s: &str) -> u128 {
    s.trim().parse().unwrap_or(0)
}

pub fn parse_amount(raw: &str, decimals: u32) -> Option<u128> {
    let s = raw.trim().replace(',', "");
    if s.is_empty() {
        return None;
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s.as_str(), ""),
    };
    let w: u128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut f: String = frac.chars().filter(|c| c.is_ascii_digit()).take(decimals as usize).collect();
    while f.len() < decimals as usize {
        f.push('0');
    }
    let frac_n: u128 = if f.is_empty() { 0 } else { f.parse().ok()? };
    let base = 10u128.checked_pow(decimals)?;
    Some(w.saturating_mul(base).saturating_add(frac_n))
}

fn fmt_units(raw: &str, decimals: u32) -> String {
    let n: u128 = raw.parse().unwrap_or(0);
    if decimals == 0 {
        return n.to_string();
    }
    let base = 10u128.pow(decimals);
    let whole = n / base;
    let frac = n % base;
    let mut f = format!("{frac:0width$}", width = decimals as usize);
    f = f.trim_end_matches('0').to_string();
    if f.is_empty() {
        format!("{whole}")
    } else {
        format!("{whole}.{f}")
    }
}

fn is_native(addr: &str) -> bool {
    addr.eq_ignore_ascii_case(NATIVE) || addr.eq_ignore_ascii_case("ETH") || addr.eq_ignore_ascii_case("AVAX")
}

fn param(query: &str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let (k, v) = part.split_once('=').unwrap_or((part, ""));
        if k == key && !v.is_empty() {
            return Some(urldecode(v));
        }
    }
    None
}

fn urldecode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let h = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("00"), 16)
                    .unwrap_or(b'?');
                out.push(h as char);
                i += 3;
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(9))
        .pool_max_idle_per_host(8)
        .tcp_nodelay(true)
        .user_agent("vapurr/0.1")
        .build()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoop_quarter_percent() {
        assert_eq!(scoop(1_000_000, 25), 2_500);
        assert_eq!(scoop(10_000_000, 25), 25_000);
    }

    #[test]
    fn parse_human_amount() {
        assert_eq!(parse_amount("1", 18).unwrap(), 10u128.pow(18));
        assert_eq!(parse_amount("1.5", 6).unwrap(), 1_500_000);
        assert_eq!(parse_amount("1", 6).unwrap(), 1_000_000);
    }

    fn test_req() -> QuoteReq {
        QuoteReq {
            kind: "swap",
            from_chain: CHAIN_ID,
            to_chain: CHAIN_ID,
            from_token: USDG.into(),
            to_token: WETH.into(),
            from_sym: "USDG".into(),
            to_sym: "WETH".into(),
            from_dec: 6,
            to_dec: 6,
            from_amount: 1_000_000,
            from_address: QUOTE_ADDR.into(),
        }
    }

    #[test]
    fn swap_list_is_house_and_stocks_not_lifi_junk() {
        let v = tokens(Some("4663"));
        let list = v["tokens"].as_array().unwrap();
        let syms: Vec<&str> = list
            .iter()
            .filter_map(|t| t.get("symbol").and_then(|x| x.as_str()))
            .collect();
        assert!(syms.contains(&"ETH"));
        assert!(syms.contains(&"USDG"));
        assert!(syms.contains(&"NVDA"));
        assert!(syms.contains(&"TSLA"));
        assert!(syms.contains(&"MSFT"));
        assert!(syms.contains(&"PLTR"));
        assert!(!syms.contains(&"WETH"));
        assert!(
            list.iter().all(|t| t.get("chain_id").and_then(|x| x.as_u64()) == Some(4663))
        );
        assert!(
            list.len() <= 20,
            "swap picker must not dump LI.FI, got {}",
            list.len()
        );
        let nvda = list.iter().find(|t| t["symbol"] == "NVDA").unwrap();
        assert_eq!(
            nvda["address"].as_str().unwrap().to_ascii_lowercase(),
            "0xd0601ce157db5bdc3162bbac2a2c8af5320d9eec"
        );
    }

    #[test]
    fn house_book_is_vapurr_pusd_only() {
        let v = house_cand(&QuoteReq {
            kind: "swap",
            from_chain: TESTNET_CHAIN_ID,
            to_chain: TESTNET_CHAIN_ID,
            from_token: TESTNET_VAPURR.into(),
            to_token: TESTNET_PUSD.into(),
            from_sym: "VAPURR".into(),
            to_sym: "PUSD".into(),
            from_dec: 18,
            to_dec: 18,
            from_amount: 10u128.pow(18),
            from_address: QUOTE_ADDR.into(),
        });
        let c = v.expect("house V→P");
        assert_eq!(c.tool, "house");
        assert!(c.tx.is_object());
        let data = c.tx.get("data").and_then(|x| x.as_str()).unwrap();
        assert!(data.starts_with("0x67b7479a"), "{data}");
        assert!(house_cand(&test_req()).is_none());
    }

    #[test]
    fn vapurr_storage_slots_match_live_layout() {
        let user = "0xc8ae558F58BaF209cF371e64b7baa84181A90060";
        let swap = TESTNET_SWAP;
        assert_eq!(
            word_hex(&map_slot(user, 1)),
            "0x2ad8ebf0121af7723680bd40677af08ff2590074d0ff41e13d9989aefaeaeddd"
        );
        assert_eq!(
            word_hex(&nest_slot(swap, &map_slot(user, 2))),
            "0x9aefae72636686b1451a6aeefdc2154ce235c2a0958f74ae73493b7bd4987638"
        );
        assert_eq!(
            word_hex(&map_slot(user, 2)),
            "0x1ea0867a89f779cb607cb1e73bdc67e52ab43cc0b006fbbe1c3db96e4e7ee584"
        );
        assert_eq!(
            word_hex(&nest_slot(swap, &map_slot(user, 3))),
            "0x48890ae5cc0225ea43a499ca60375b84e0df77a7271c165e541dc7611ee0d52d"
        );
    }

    #[test]
    fn approve_is_unlimited_spender() {
        let d = encode_approve(TESTNET_SWAP);
        assert!(d.starts_with("0x095ea7b3"), "{d}");
        assert!(d.ends_with(&"f".repeat(64)), "{d}");
    }

    #[test]
    fn house_flags_block_pay_until_approve() {
        let req = QuoteReq {
            kind: "swap",
            from_chain: TESTNET_CHAIN_ID,
            to_chain: TESTNET_CHAIN_ID,
            from_token: TESTNET_VAPURR.into(),
            to_token: TESTNET_PUSD.into(),
            from_sym: "VAPURR".into(),
            to_sym: "PUSD".into(),
            from_dec: 18,
            to_dec: 18,
            from_amount: 10u128.pow(18),
            from_address: "0xc8ae558F58BaF209cF371e64b7baa84181A90060".into(),
        };
        let mut c = house_cand(&req).unwrap();
        c.sim = SimReport {
            ran: true,
            ok: true,
            source: "rpc".into(),
            ret: format!("0x{:064x}", 10u128.pow(18) * 995 / 1000),
            gas: 202_496,
            ..SimReport::default()
        };
        c.gross_out = 10u128.pow(18) * 995 / 1000;
        c.net_out = c.gross_out;
        c.to_min_net = c.net_out * 99 / 100;
        let mut v = pack_ranked(&req, std::slice::from_ref(&c), None);
        house_pay_flags(
            &mut v,
            &req,
            &HouseBag {
                allowance: 0,
                balance: 10u128.pow(18),
            },
        );
        assert_eq!(v["payable"], false);
        assert_eq!(v["needs_approve"], true);
        assert_eq!(v["funded"], true);
        assert!(v["approve"]["data"].as_str().unwrap().starts_with("0x095ea7b3"));
        assert!(v["note"].as_str().unwrap().contains("Approve"));
    }

    #[test]
    fn unpayable_quote_is_not_cached() {
        let v = fallback_quote(&test_req(), "no route");
        cache_put("fromChain=4663&amount=1", v);
        assert!(cache_get("fromChain=4663&amount=1").is_none());
    }

    #[test]
    fn live_house_quote_optional() {
        let q = format!(
            "fromChain={TESTNET_CHAIN_ID}&toChain={TESTNET_CHAIN_ID}&fromToken={TESTNET_VAPURR}&toToken={TESTNET_PUSD}&fromSymbol=VAPURR&toSymbol=PUSD&fromDecimals=18&toDecimals=18&amount=1&fromAddress=0xc8ae558F58BaF209cF371e64b7baa84181A90060"
        );
        let s = quote_json(&q);
        let v: Value = serde_json::from_str(&s).unwrap();
        eprintln!("house quote {s}");
        if v.get("sim").and_then(|x| x.get("ran")).and_then(|x| x.as_bool()) != Some(true) {
            return;
        }
        assert_eq!(v["ok"], true);
        assert_eq!(v["tool"], "house");
        assert_eq!(v["sim"]["ok"], true, "override sim must finish ok: {s}");
        assert_eq!(v["payable"], false, "user bag is empty");
        let out: u128 = v["to_amount"].as_str().unwrap().parse().unwrap();
        assert!(out > 0, "quoted out {out}");
        assert!(v["ms"].as_u64().unwrap_or(99_000) < 8_000, "sim hung: {s}");
    }

    #[test]
    fn net_after_fee_is_25_bps() {
        let (net, fee) = net_after_fee(1_000_000);
        assert_eq!(fee, 2_500);
        assert_eq!(net, 997_500);
    }

    #[test]
    fn house_refund_is_three_bps() {
        let wei = vapurr_refund_wei_bps(HOUSE_REFUND_BPS, 1.0, 1_000_000, 6);
        assert_eq!(wei, 3 * 10u128.pow(14)); // 0.0003 VAPURR
        assert_eq!(HOUSE_REFUND_BPS, 3);
    }

    #[test]
    fn vapurr_refund_is_five_bps() {
        let wei = vapurr_refund_wei(1.0, 1_000_000, 6);
        assert_eq!(wei, 5 * 10u128.pow(14)); // 0.0005 VAPURR
        assert_eq!(vapurr_refund_wei(0.0, 1_000_000, 6), 5 * 10u128.pow(14));
    }

    #[test]
    fn score_picks_best_full_route_minus_gas() {
        let a = route_score(1_000_000, gas_in_out_units(0.04, 1.0, 1_000_000));
        let b = route_score(1_010_000, gas_in_out_units(20.0, 1.01, 1_010_000));
        assert!(a > b, "low-gas full route must win: {a} vs {b}");
    }

    fn sample_quote() -> Value {
        json!({
            "tool": "uniswap",
            "action": { "fromAmount": "1000000" },
            "estimate": {
                "toAmount": "1000000",
                "toAmountMin": "990000",
                "toAmountUSD": "1.00",
                "fromAmountUSD": "1.00",
                "executionDuration": 8,
                "gasCosts": [{ "amountUSD": "0.04" }]
            },
            "includedSteps": [{ "tool": "uniswap", "type": "swap", "toolDetails": { "name": "Uniswap" } }],
            "transactionRequest": { "to": "0xabc", "data": "0x", "value": "0x0", "chainId": 4663 }
        })
    }

    #[test]
    fn packed_quote_does_not_cut_the_route() {
        let req = test_req();
        let c = cand_from_lifi_quote(&sample_quote(), &req);
        let v = pack_ranked(&req, &[c], None);
        assert_eq!(v["ok"], true);
        assert_eq!(v["to_display"], "1");
        assert_eq!(v["payable"], false, "a tx is not a simulation");
        assert_eq!(v["simulated"], false);
        assert_eq!(v["refund"]["display"], "0.0005");
        let hops = v["hops"].as_array().unwrap();
        assert_eq!(hops[0]["name"], "Uniswap");
        let trace = v["trace"].as_array().unwrap();
        assert!(trace.iter().any(|n| n["kind"] == "refund"));
        assert!(trace.iter().any(|n| n["kind"] == "burn"));
        assert_eq!(trace[0]["kind"], "in");
    }

    #[test]
    fn payable_only_after_rpc_sim_ok() {
        let req = test_req();
        let mut c = cand_from_lifi_quote(&sample_quote(), &req);
        c.sim = SimReport {
            ran: true,
            ok: true,
            source: "rpc".into(),
            rpc: crate::RPC_HTTP.into(),
            chain_id: 4663,
            from: QUOTE_ADDR.into(),
            to: "0xabc".into(),
            gas: 184_221,
            gas_price: 1_000_000_000,
            revert: String::new(),
            ret: "0x".into(),
        };
        let v = pack_ranked(&req, &[c], None);
        assert_eq!(v["payable"], true);
        assert_eq!(v["sim"]["ok"], true);
        assert_eq!(v["sim"]["gas"], 184_221);
        assert_eq!(v["sim"]["source"], "rpc");
        let uni = v["trace"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["label"] == "Uniswap")
            .unwrap();
        assert_eq!(uni["state"], "ok");
        assert!(uni["value"].as_str().unwrap().contains("gas"));
    }

    #[test]
    fn decode_revert_reads_error_string() {
        let hex = "08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000024869000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(decode_revert(hex), "Hi");
        assert_eq!(decode_revert("execution reverted: STABLE"), "STABLE");
    }

    #[test]
    fn fallback_is_not_payable() {
        let v = fallback_quote(&test_req(), "no route");
        assert_eq!(v["ok"], true);
        assert_eq!(v["estimate"], true);
        assert_eq!(v["payable"], false);
        assert_eq!(v["refund"]["bps"], 5);
        let out: u128 = v["to_amount"].as_str().unwrap().parse().unwrap();
        assert_eq!(out, 1_000_000);
    }

    fn mock_cand(tool: &str, out: u128, gas_usd: f64, sim_ok: bool) -> Cand {
        Cand {
            id: tool.into(),
            provider: "t".into(),
            tool: tool.into(),
            hops: vec![],
            gross_out: out,
            net_out: out,
            fee_out: 0,
            to_min_net: out,
            gas_usd,
            to_usd: 1.0,
            from_usd: 1.0,
            duration: 8,
            tx: if sim_ok {
                json!({ "to": "0xabc", "data": "0x", "chainId": 4663 })
            } else {
                Value::Null
            },
            step: None,
            sim: SimReport {
                ran: true,
                ok: sim_ok,
                source: "rpc".into(),
                gas: 21_000,
                gas_price: 0,
                ..SimReport::default()
            },
        }
    }

    #[test]
    fn pick_best_never_takes_a_worse_simulated_net() {
        let req = test_req();
        let fat_fail = mock_cand("fat", 1_200_000, 0.01, false);
        let mid_ok = mock_cand("mid", 1_000_000, 0.04, true);
        let thin_ok = mock_cand("thin", 1_010_000, 0.20, true);
        let cands = vec![fat_fail, mid_ok, thin_ok];
        let w = pick_best(&cands, &req).unwrap();
        assert_eq!(w.tool, "mid", "fat didn't sim; thin loses on gas; mid wins user-net");
        assert!(score_of(w, &req) >= score_of(&cands[2], &req));
    }

    #[test]
    fn impact_is_percent() {
        assert_eq!(impact_pct(100.0, 99.5), "0.50%");
        assert_eq!(impact_pct(1.0, 1.0), "~0%");
        assert_eq!(impact_pct(0.0, 1.0), "");
    }

    #[test]
    fn score_i128_serializes_as_string() {
        // token-unit scores exceed i64; json!(i128) panics "number out of range"
        let huge: i128 = (u128::MAX / 2) as i128;
        let v = serde_json::json!({ "score": huge.to_string() });
        assert_eq!(v["score"].as_str().unwrap(), huge.to_string());
    }

    #[test]
    fn user_score_includes_refund_and_still_picks_more_out() {
        let a = user_score(1_000_000, refund_out_units(1.0, 1.0, 1_000_000), 40_000);
        let b = user_score(1_100_000, refund_out_units(1.0, 1.0, 1_100_000), 40_000);
        assert!(b > a);
        assert_eq!(refund_out_units(1.0, 1.0, 1_000_000), 500);
    }
}
