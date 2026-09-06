//! Swap and bridge router.
//!
//! Routes rank quoted output minus gas. No rebate is advertised without a payout.
//! A source-chain simulation does not prove destination bridge settlement.

mod execution;
mod validation;
pub use execution::{take_execution, Execution};
mod book;
use book::HouseBook;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::{
    CHAIN_ID, NATIVE, PUSD_TOKEN, ROUTE_FEE_BPS, ROUTE_INTEGRATOR,
    ROUTE_REFUND_BPS, STOCKS, TESTNET_CHAIN_ID, TESTNET_PUSD, TESTNET_STOCKS,
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
static GAS_CACHE: Mutex<Option<(Instant, u64, u128)>> = Mutex::new(None);

pub fn scoop(amount: u128, bps: u32) -> u128 {
    amount.saturating_mul(bps as u128) / 10_000
}

pub fn tokens_json(query: &str) -> String {
    let chain = param(query, "chain");
    serde_json::to_string(&tokens(chain.as_deref())).unwrap_or_else(|_| "{}".into())
}

pub fn quote_json(query: &str) -> String {
    match quote(query) {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "ok": false, "error": e, "fee_bps": 0 }).to_string(),
    }
}




pub fn tokens(chain: Option<&str>) -> Value {
    let mut out = rail_tokens();
    if let Some(c) = chain.and_then(|s| s.parse::<u64>().ok()) {
        out.retain(|t| t.get("chain_id").and_then(|x| x.as_u64()) == Some(c));
    }
    json!({
        "ok": true,
        "fee_bps": 0,
        "fee": "Included in provider quote",
        "refund_bps": 0,
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
    for chain in [CHAIN_ID, TESTNET_CHAIN_ID] {
        if let Some(book) = HouseBook::load(chain) {
            out.retain(|t| t["chain_id"] != chain || !matches!(t["symbol"].as_str(), Some("VAPURR" | "PUSD")));
            push_tok(&mut out, chain, &book.vapurr, "VAPURR", "VAPURR", 18);
            push_tok(&mut out, chain, &book.equity, "wgV", "Wrapped gV", 18);
            push_tok(&mut out, chain, &book.cash, "PUSD", "PUSD", 18);
        }
    }
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
    house: Option<HouseBook>,
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



/// Quoted output less estimated gas, without hypothetical rebates.
pub fn route_score(net_out: u128, gas_out_units: u128) -> i128 {
    (net_out.min(i128::MAX as u128) as i128).saturating_sub(gas_out_units.min(i128::MAX as u128) as i128)
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
        let fee = req.house.as_ref().ok_or("House deployment missing")?.verify(req.from_chain)?;
        let bag = house_bag(&req);
        simulate_house(&mut house, &req);
        let mut v = pack_ranked(&req, std::slice::from_ref(&house), None);
        house_pay_flags(&mut v, &req, &bag);
        v["fee_sink"] = json!({"label":format!("{:.2}% House pool fee", fee as f64 / 10_000.0)});
        v["fee_bps"] = json!(fee / 100);
        if let Some(obj) = v.as_object_mut() {
            obj.insert("ms".into(), json!(t0.elapsed().as_millis() as u64));
        }
        execution::authorize(&req, &mut v);
        return Ok(v);
    }
    if req.from_chain == TESTNET_CHAIN_ID && req.to_chain == TESTNET_CHAIN_ID {
        return Ok(fallback_quote(
            &req,
            "House trades wgV / PUSD. Use Lithe to mint or redeem VAPURR / PUSD.",
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
        Ok(raw) => match validation::action(&raw, &req).and_then(|_| validation::transaction(&raw["transactionRequest"], &req)) {
            Ok(()) => cands.push(cand_from_lifi_quote(&raw, &req)),
            Err(e) => why = e,
        },
        Err(e) => why = e,
    }
    for (res, tag) in [(cheap_res, "cheap"), (fast_res, "fast")] {
        let _ = tag;
        match res {
            Ok(routes) => {
                for raw in routes {
                    if let Err(e) = validation::single_step(&raw, &req) { why = e; continue; }
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
    simulate_top(&mut cands, &req, 3);
    cands.sort_by(|a, b| cmp_best(a, b, &req));
    let mut v = pack_ranked(&req, &cands, baseline.as_ref());
    if let Some(obj) = v.as_object_mut() {
        obj.insert("ms".into(), json!(t0.elapsed().as_millis() as u64));
    }
    provider_approval(&req, &cands[0], &mut v);
    execution::authorize(&req, &mut v);
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
    let catalog = rail_tokens();
    let metadata = |chain, address: &str, decimals| -> Result<String, String> {
        let token = catalog.iter().find(|t| t["chain_id"].as_u64() == Some(chain)
            && t["address"].as_str().is_some_and(|a| addr_eq(a, address))).ok_or("Token is not in the current route catalog")?;
        if token["decimals"].as_u64() != Some(decimals as u64) { return Err("Token precision differs from the route catalog".into()); }
        Ok(token["symbol"].as_str().unwrap_or("TOKEN").to_string())
    };
    let _ = (from_sym, to_sym);
    let from_sym = metadata(from_chain, &from_token, from_dec)?;
    let to_sym = metadata(to_chain, &to_token, to_dec)?;
    if from_dec > 38 || to_dec > 38 { return Err("unsupported token precision".into()); }
    if !valid_address(&from_token) && !is_native(&from_token) { return Err("invalid source token".into()); }
    if !valid_address(&to_token) && !is_native(&to_token) { return Err("invalid destination token".into()); }
    if rpc_for(from_chain).is_none() || rpc_for(to_chain).is_none() { return Err("unsupported chain".into()); }
    if from_chain == to_chain && addr_eq(&from_token, &to_token) { return Err("choose different assets".into()); }
    let from_amount = parse_amount(&amount_raw, from_dec).ok_or("bad amount")?;
    if from_amount == 0 {
        return Err("amount too small".into());
    }
    let from_address = param(query, "fromAddress").unwrap_or_else(|| QUOTE_ADDR.to_string());
    if !valid_address(&from_address) { return Err("invalid sender".into()); }
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
        house: HouseBook::load(from_chain),
    })
}

fn addr_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}




fn house_cand(req: &QuoteReq) -> Option<Cand> {
    if req.from_chain != req.to_chain {
        return None;
    }
    let book = req.house.as_ref()?;
    if !book.is_pair(req) { return None; }
    let sell_v = addr_eq(&req.from_token, &book.equity);
    let swapper = &book.swapper;
    let data = encode_swap_exact(sell_v, req.from_amount, 0);
    let est = 0;
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

fn encode_approve(spender: &str, amount: u128) -> String {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);
    d.extend_from_slice(&abi_addr_word(spender));
    d.extend_from_slice(&u256_be(amount));
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
    let Some(swapper) = req.house.as_ref().map(|b| b.swapper.as_str()) else {
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


fn set_tx_data(c: &mut Cand, data: String) {
    if let Some(obj) = c.tx.as_object_mut() {
        obj.insert("data".into(), json!(data));
    }
}

fn simulate_house(c: &mut Cand, req: &QuoteReq) {
    let sell_v = req.house.as_ref().is_some_and(|b| addr_eq(&req.from_token, &b.equity));
    set_tx_data(c, encode_swap_exact(sell_v, req.from_amount, 0));
    c.sim = rpc_sim(c, &req.from_address);
    if !c.sim.ok { return; }
    let Some(out) = parse_ret_u128(&c.sim.ret).filter(|n| *n > 1) else {
        c.sim.ok = false; c.sim.revert = "House returned no valid output".into(); return;
    };
    c.gross_out = out; c.net_out = out;
    c.to_min_net = out.saturating_sub(scoop(out, 50)).max(1);
    set_tx_data(c, encode_swap_exact(sell_v, req.from_amount, c.to_min_net));
    // Simulate the exact calldata that will be signed, including minimum output.
    c.sim = rpc_sim(c, &req.from_address);
}

fn valid_address(addr: &str) -> bool {
    addr.len() == 42 && addr.starts_with("0x") && addr[2..].bytes().all(|b| b.is_ascii_hexdigit())
        && addr[2..].bytes().any(|b| b != b'0')
}

fn real_wallet(addr: &str) -> bool {
    valid_address(addr) && !addr_eq(addr, QUOTE_ADDR)
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
        json!(have_wallet && funded && needs_approve),
    );
    if have_wallet && funded && needs_approve {
        if let Some(swapper) = req.house.as_ref().map(|b| b.swapper.as_str()) {
            obj.insert(
                "approve".into(),
                json!({
                    "to": req.from_token,
                    "spender": swapper,
                    "data": encode_approve(swapper, req.from_amount),
                    "chainId": req.from_chain,
                    "value": "0x0",
                }),
            );
        }
    }
    let sym = req.from_sym.trim_start_matches('$');
    let note = if payable {
        "House wgV / PUSD. Review output and pool fee before signing.".to_string()
    } else if have_wallet && funded && needs_approve {
        format!("Approve ${sym} for this amount, then refresh the quote.")
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

fn provider_approval(req: &QuoteReq, winner: &Cand, quote: &mut Value) {
    if is_native(&req.from_token) || !real_wallet(&req.from_address)
        || validation::transaction(&winner.tx, req).is_err() { return; }
    let Some(step) = winner.step.as_ref() else { return; };
    let Some(spender) = step["estimate"]["approvalAddress"].as_str() else { return; };
    // This executor supports direct router approvals, not arbitrary third-party spenders.
    if !valid_address(spender) || !winner.tx["to"].as_str().is_some_and(|to| addr_eq(spender,to)) { return; }
    let Some(url) = rpc_for(req.from_chain) else { return; };
    let rpc = crate::rpc::Rpc::at_timeout(url,6);
    let balance = token_u128(&rpc,&req.from_token,&encode_balance_of(&req.from_address));
    let allowance = token_u128(&rpc,&req.from_token,&encode_allowance(&req.from_address,spender));
    if balance >= req.from_amount && allowance < req.from_amount {
        quote["payable"] = json!(false);
        quote["needs_approve"] = json!(true);
        quote["approve"] = json!({"to":req.from_token,"spender":spender,"chainId":req.from_chain,
            "value":"0x0","data":encode_approve(spender,req.from_amount)});
        quote["note"] = json!("Approve only this amount, then refresh and simulate the route.");
    }
}

fn u256_be(n: u128) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[16..].copy_from_slice(&n.to_be_bytes());
    w
}

fn parse_ret_u128(ret: &str) -> Option<u128> {
    let s = ret.trim().strip_prefix("0x")?;
    if s.len() != 64 || !s[..32].bytes().all(|b| b == b'0') { return None; }
    u128::from_str_radix(&s[32..], 16).ok()
}

fn score_of(c: &Cand, req: &QuoteReq) -> i128 {
    route_score(c.net_out, gas_units_of(c, req))
}

fn gas_units_of(c: &Cand, req: &QuoteReq) -> u128 {
    let eth_out = is_native(&req.to_token) || req.to_token.eq_ignore_ascii_case(WETH);
    if req.from_chain == req.to_chain && eth_out && c.sim.ok && c.sim.gas > 0 && c.sim.gas_price > 0 {
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

#[cfg(test)]
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
    let fee = 0;
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
            step: Some(raw.clone()),
            sim: SimReport::default(),
        },
        gross,
        to_min,
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
        to_min,
    )
}

fn simulate_top(cands: &mut Vec<Cand>, req: &QuoteReq, n: usize) {
    let n = n.min(cands.len());
    if n == 0 {
        return;
    }
    let chunk: Vec<Cand> = cands.iter().take(n).cloned().collect();
    let done: Vec<Cand> = std::thread::scope(|s| {
        let handles: Vec<_> = chunk
            .into_iter()
            .map(|mut c| {
                s.spawn(move || {
                    simulate_cand(&mut c, req);
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

fn simulate_cand(c: &mut Cand, req: &QuoteReq) {
    if !c.tx.is_object() {
        if let Some(step) = c.step.clone() {
            match lifi_step_tx(&step) {
                Ok(filled) => {
                    if let Err(e) = validation::action(&filled, req) {
                        c.sim.revert = e;
                        return;
                    }
                    if filled["estimate"]["toAmountMin"].as_str().and_then(|s| s.parse::<u128>().ok()) != Some(c.to_min_net)
                        || filled["estimate"]["toAmount"].as_str().and_then(|s| s.parse::<u128>().ok()) != Some(c.net_out) {
                        c.sim.revert = "Step output changed. Refresh the route.".into();
                        return;
                    }
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
    if let Err(e) = validation::transaction(&c.tx, req) { c.sim.revert = e; return; }
    c.sim = rpc_sim(c, &req.from_address);
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
            match gas_res {
                Ok(Ok(g)) if g > 0 => { s.ok = true; s.gas = g; }
                _ => { s.ok = false; s.revert = "gas estimation failed".into(); }
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



#[allow(dead_code)]
/// `serde_json::json!` panics on u128/i128 outside u64/i64 ("number out of range").
/// Prefer strings for token-unit ints; keep values that fit in u64 as numbers.
fn json_u128(n: u128) -> Value {
    if n <= u64::MAX as u128 {
        Value::from(n as u64)
    } else {
        Value::String(n.to_string())
    }
}

#[allow(dead_code)]
fn json_i128(n: i128) -> Value {
    if n >= i64::MIN as i128 && n <= i64::MAX as i128 {
        Value::from(n as i64)
    } else {
        Value::String(n.to_string())
    }
}

/// Recursively stringify Numbers that do not fit in u64/i64 (e.g. LiFi / arbitrary-precision).
/// Numbers fine as u64/i64 stay; floats stay.
fn stringify_oversized_numbers(v: &mut Value) {
    match v {
        Value::Number(n) => {
            if n.as_u64().is_some() || n.as_i64().is_some() {
                return;
            }
            // Integer-like that only exists as string/bignum under arbitrary_precision,
            // or a pure integer written as f64 beyond exact range — emit decimal string.
            let s = n.to_string();
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                *v = Value::String(s);
            }
        }
        Value::Array(a) => {
            for x in a.iter_mut() {
                stringify_oversized_numbers(x);
            }
        }
        Value::Object(o) => {
            for x in o.values_mut() {
                stringify_oversized_numbers(x);
            }
        }
        _ => {}
    }
}

fn safe_tx(tx: &Value) -> Value {
    let mut v = tx.clone();
    stringify_oversized_numbers(&mut v);
    v
}

fn pack_ranked(req: &QuoteReq, cands: &[Cand], baseline: Option<&Cand>) -> Value {
    let winner = &cands[0];
    let house = winner.tool == "house";
    let payable = if house {
        winner.tx.is_object() && winner.sim.ok && winner.to_min_net > 0
    } else {
        winner.sim.ok && winner.tx.is_object() && winner.to_min_net > 0
    };
    let fee = if house {
        json!({
            "bps": 30,
            "label": "0.30% house book. No LI.FI cut.",
        })
    } else {
        fee_plan(req, winner)
    };
    let refund_bps = 0;
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
        "fee_bps": 0,
        "fee": "Included in provider quote",
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
        "fee_usd": "",
        "gas_usd": if winner.gas_usd > 0.0 { format!("${:.2}", winner.gas_usd) } else { String::new() },
        "hops": winner.hops,
        "tx": safe_tx(&winner.tx),
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
            "House wgV / PUSD. Review output and pool fee before signing.".to_string()
        } else if payable {
            "Source transaction simulated. Provider fees are included; gas is separate.".to_string()
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

fn build_trace(req: &QuoteReq, c: &Cand, _refund_disp: &str) -> Vec<Value> {
    let swap = if !c.sim.ran {
        "wait"
    } else if c.sim.ok {
        "ok"
    } else {
        "fail"
    };

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
            "state": if req.kind == "bridge" { "held" } else { swap },
        }));
    } else {
        for (i, h) in c.hops.iter().enumerate() {
            let last = i + 1 == c.hops.len();
            nodes.push(json!({
                "kind": h.get("type").and_then(|x| x.as_str()).unwrap_or("swap"),
                "label": h.get("name").and_then(|x| x.as_str()).unwrap_or(&c.tool),
                "value": if last && c.sim.gas > 0 { format!("{} gas", c.sim.gas) } else { String::new() },
                "state": if req.kind == "bridge" { "held" } else { swap },
            }));
        }
    }
    nodes.push(json!({
        "kind": "out",
        "label": if req.kind == "bridge" { "Estimated destination receipt" } else { "Estimated receipt" },
        "value": format!("{} {}", fmt_units(&c.net_out.to_string(), req.to_dec), req.to_sym),
        "state": if req.kind == "bridge" { "held" } else { swap },
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
        "refund_display": "0",
        "why": if winner.sim.ok {
            "Best quoted output minus gas among source-simulated routes."
        } else {
            "No RPC-passing route yet. Ranking is quote-only."
        },
    })
}

fn refund_plan(req: &QuoteReq, c: &Cand) -> Value {
    refund_plan_at(req, c, ROUTE_REFUND_BPS)
}

fn refund_plan_at(_req: &QuoteReq, _c: &Cand, _bps: u32) -> Value {
    json!({"bps":0,"asset":"VAPURR","decimals":18,"amount":"0","display":"0",
        "status":"not_in_route","label":"No VAPURR rebate in this transaction"})
}

fn fee_plan(_req: &QuoteReq, _c: &Cand) -> Value {
    json!({"bps":0,"action":"provider_quote","label":"Provider fees included in quote; gas separate"})
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
            ("toAddress", from_address.to_string()),
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
        "toAddress": from_address,
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
    let out = 0;
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
        "fee_bps": 0,
        "fee": "Included in provider quote",
        "refund_bps": 0,
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
        "note": format!("No executable route: {why}"),
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
    if decimals > 38 { return None; }
    let s = raw.trim();
    if s.is_empty() || s.len() > 80 { return None; }
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if (whole.is_empty() && frac.is_empty()) || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit()) || frac.len() > decimals as usize { return None; }
    let w: u128 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
    let f: u128 = if frac.is_empty() { 0 } else { frac.parse().ok()? };
    w.checked_mul(10u128.checked_pow(decimals)?)?
        .checked_add(f.checked_mul(10u128.checked_pow(decimals - frac.len() as u32)?)?)
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
    use crate::TESTNET_SWAP;

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
            house: Some(HouseBook { equity: TESTNET_VAPURR.into(), cash: TESTNET_PUSD.into(), swapper: TESTNET_SWAP.into(), vapurr: "0x4444444444444444444444444444444444444444".into() }),
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
            house: Some(HouseBook { equity: TESTNET_VAPURR.into(), cash: TESTNET_PUSD.into(), swapper: TESTNET_SWAP.into(), vapurr: "0x4444444444444444444444444444444444444444".into() }),
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
    fn approve_is_exact_amount() {
        let d = encode_approve(TESTNET_SWAP, 123);
        assert!(d.starts_with("0x095ea7b3"), "{d}");
        assert!(d.ends_with(&format!("{:064x}", 123)), "{d}");
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
            house: Some(HouseBook { equity: TESTNET_VAPURR.into(), cash: TESTNET_PUSD.into(), swapper: TESTNET_SWAP.into(), vapurr: "0x4444444444444444444444444444444444444444".into() }),
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
    fn net_after_fee_is_25_bps() {
        let (net, fee) = net_after_fee(1_000_000);
        assert_eq!(fee, 2_500);
        assert_eq!(net, 997_500);
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
        assert_eq!(v["refund"]["display"], "0");
        let hops = v["hops"].as_array().unwrap();
        assert_eq!(hops[0]["name"], "Uniswap");
        let trace = v["trace"].as_array().unwrap();
        assert!(!trace.iter().any(|n| n["kind"] == "refund"));
        assert!(!trace.iter().any(|n| n["kind"] == "burn"));
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
        assert_eq!(v["refund"]["bps"], 0);
        let out: u128 = v["to_amount"].as_str().unwrap().parse().unwrap();
        assert_eq!(out, 0);
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
    fn pack_ranked_survives_huge_net_out_and_gas_price() {
        let req = test_req();
        let huge = u128::MAX / 3;
        let mut c = mock_cand("fat", huge, 0.01, true);
        c.net_out = huge;
        c.gross_out = huge;
        c.to_min_net = huge / 2;
        c.sim.gas_price = u128::MAX / 5;
        c.sim.gas = u64::MAX / 7;
        // LiFi-style tx may carry oversized numeric fields after re-encoding.
        c.tx = json!({
            "to": "0xabc",
            "data": "0x",
            "chainId": 4663,
            "gasPrice": u64::MAX,
            "value": "0x0",
            "nested": { "amount": u64::MAX },
        });
        let v = pack_ranked(&req, &[c], None);
        assert_eq!(v["ok"], true);
        assert_eq!(v["to_amount"].as_str().unwrap(), huge.to_string());
        assert!(v["score"].as_str().is_some(), "score must be string, got {:?}", v["score"]);
        assert_eq!(v["sim"]["gas_price"].as_str().unwrap(), (u128::MAX / 5).to_string());
        // Round-trip serialize must not panic.
        let s = serde_json::to_string(&v).expect("serialize");
        assert!(s.contains(&huge.to_string()));
    }

}
