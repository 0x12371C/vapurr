//! Swap and bridge router. LI.FI finds the path; vapurr scoops 25 bps.

use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use crate::{
    CHAIN_ID, NATIVE, ROUTE_FEE, ROUTE_FEE_BPS, ROUTE_INTEGRATOR, USDG, USDG_DECIMALS, WETH,
};

const LIFI: &str = "https://li.quest/v1";
const QUOTE_ADDR: &str = "0x552008c0f6870c2f77e5cC1d2eb9bdff03e30Ea0";
const AVAX_USDC: &str = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E";
const ETH_USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

struct Cache<T> {
    val: T,
}

static TOKENS: Mutex<Option<Cache<Value>>> = Mutex::new(None);
static TOKEN_LOOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
        Err(e) => json!({ "ok": false, "error": e, "fee_bps": ROUTE_FEE_BPS }).to_string(),
    }
}

pub fn tokens(chain: Option<&str>) -> Value {
    let rails = rail_tokens();
    let extra = lifi_tokens();
    let mut out = rails;
    if let Some(arr) = extra.as_array() {
        for t in arr {
            let addr = t.get("address").and_then(|x| x.as_str()).unwrap_or("");
            let cid = t.get("chain_id").and_then(|x| x.as_u64()).unwrap_or(0);
            let dup = out.iter().any(|x| {
                x.get("address").and_then(|a| a.as_str()) == Some(addr)
                    && x.get("chain_id").and_then(|a| a.as_u64()) == Some(cid)
            });
            if !dup && !addr.is_empty() {
                out.push(t.clone());
            }
        }
    }
    if let Some(c) = chain.and_then(|s| s.parse::<u64>().ok()) {
        out.retain(|t| t.get("chain_id").and_then(|x| x.as_u64()) == Some(c));
    }
    json!({
        "ok": true,
        "fee_bps": ROUTE_FEE_BPS,
        "fee": "0.25%",
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
    vec![
        tok(CHAIN_ID, NATIVE, "ETH", "Ether", 18),
        tok(CHAIN_ID, WETH, "WETH", "Wrapped ETH", 18),
        tok(CHAIN_ID, USDG, "USDG", "USDG", USDG_DECIMALS as u32),
        tok(1, NATIVE, "ETH", "Ether", 18),
        tok(1, ETH_USDC, "USDC", "USD Coin", 6),
        tok(43114, NATIVE, "AVAX", "Avalanche", 18),
        tok(43114, AVAX_USDC, "USDC", "USD Coin", 6),
        tok(8453, NATIVE, "ETH", "Ether", 18),
        tok(42161, NATIVE, "ETH", "Ether", 18),
    ]
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

fn lifi_tokens() -> Value {
    kick_tokens();
    if let Ok(g) = TOKENS.lock() {
        if let Some(c) = g.as_ref() {
            return c.val.clone();
        }
    }
    json!([])
}

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

fn quote(query: &str) -> Result<Value, String> {
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
    let kind = if from_chain == to_chain {
        "swap"
    } else {
        "bridge"
    };

    match lifi_quote(
        from_chain,
        to_chain,
        &from_token,
        &to_token,
        &from_amount.to_string(),
        &from_address,
    ) {
        Ok(raw) => Ok(pack_lifi(
            kind,
            &raw,
            from_chain,
            to_chain,
            &from_sym,
            &to_sym,
            from_dec,
            to_dec,
            &from_amount.to_string(),
        )),
        Err(e) => Ok(fallback_quote(
            kind,
            from_chain,
            to_chain,
            &from_token,
            &to_token,
            &from_sym,
            &to_sym,
            from_dec,
            to_dec,
            from_amount,
            &e,
        )),
    }
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
            ("fee", ROUTE_FEE.to_string()),
            ("slippage", "0.005".into()),
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

fn pack_lifi(
    kind: &str,
    raw: &Value,
    from_chain: u64,
    to_chain: u64,
    from_sym: &str,
    to_sym: &str,
    from_dec: u32,
    to_dec: u32,
    from_amount: &str,
) -> Value {
    let est = raw.get("estimate").cloned().unwrap_or(Value::Null);
    let action = raw.get("action").cloned().unwrap_or(Value::Null);
    let to_amount = est
        .get("toAmount")
        .and_then(|x| x.as_str())
        .unwrap_or("0");
    let to_min = est
        .get("toAmountMin")
        .and_then(|x| x.as_str())
        .unwrap_or(to_amount);
    let from_amt = action
        .get("fromAmount")
        .and_then(|x| x.as_str())
        .unwrap_or(from_amount);
    let hops = hops_of(raw);
    let fee_usd = fee_usd_of(&est);
    let gas_usd = gas_usd_of(&est);
    let tx = raw.get("transactionRequest").cloned();
    json!({
        "ok": true,
        "kind": kind,
        "provider": "LI.FI",
        "fee_bps": ROUTE_FEE_BPS,
        "fee": "0.25%",
        "integrator": ROUTE_INTEGRATOR,
        "from_chain": from_chain,
        "to_chain": to_chain,
        "from_symbol": from_sym,
        "to_symbol": to_sym,
        "from_amount": from_amt,
        "from_display": fmt_units(from_amt, from_dec),
        "to_amount": to_amount,
        "to_min": to_min,
        "to_display": fmt_units(to_amount, to_dec),
        "to_min_display": fmt_units(to_min, to_dec),
        "duration": est.get("executionDuration").and_then(|x| x.as_u64()).unwrap_or(0),
        "fee_usd": fee_usd,
        "gas_usd": gas_usd,
        "hops": hops,
        "tx": tx,
        "tool": raw.get("tool").and_then(|x| x.as_str()).unwrap_or("lifi"),
    })
}

fn fallback_quote(
    kind: &str,
    from_chain: u64,
    to_chain: u64,
    from_token: &str,
    to_token: &str,
    from_sym: &str,
    to_sym: &str,
    from_dec: u32,
    to_dec: u32,
    from_amount: u128,
    why: &str,
) -> Value {
    let _ = (from_token, to_token);
    let fee = scoop(from_amount, ROUTE_FEE_BPS);
    let after = from_amount.saturating_sub(fee);
    // Same-asset estimate when decimals match; otherwise keep units and mark as estimate.
    let out = if from_dec == to_dec {
        after
    } else if from_dec > to_dec {
        after / 10u128.pow(from_dec - to_dec)
    } else {
        after.saturating_mul(10u128.pow(to_dec - from_dec))
    };
    json!({
        "ok": true,
        "kind": kind,
        "provider": "vapurr",
        "estimate": true,
        "error": why,
        "fee_bps": ROUTE_FEE_BPS,
        "fee": "0.25%",
        "integrator": ROUTE_INTEGRATOR,
        "from_chain": from_chain,
        "to_chain": to_chain,
        "from_symbol": from_sym,
        "to_symbol": to_sym,
        "from_amount": from_amount.to_string(),
        "from_display": fmt_units(&from_amount.to_string(), from_dec),
        "to_amount": out.to_string(),
        "to_min": out.to_string(),
        "to_display": fmt_units(&out.to_string(), to_dec),
        "to_min_display": fmt_units(&out.to_string(), to_dec),
        "duration": 0,
        "fee_usd": "",
        "gas_usd": "",
        "hops": [{ "tool": "vapurr", "name": format!("0.25% scoop · {why}") }],
        "tx": Value::Null,
        "tool": "vapurr",
        "note": format!("LI.FI: {why}. Showing 0.25% vapurr scoop on a local estimate."),
    })
}

fn hops_of(raw: &Value) -> Vec<Value> {
    let mut hops = Vec::new();
    if let Some(steps) = raw.get("includedSteps").and_then(|x| x.as_array()) {
        for s in steps {
            let tool = s.get("tool").and_then(|x| x.as_str()).unwrap_or("step");
            let name = s
                .get("toolDetails")
                .and_then(|x| x.get("name"))
                .and_then(|x| x.as_str())
                .unwrap_or(tool);
            let kind = s.get("type").and_then(|x| x.as_str()).unwrap_or("");
            hops.push(json!({ "tool": tool, "name": name, "type": kind }));
        }
    }
    if hops.is_empty() {
        let tool = raw.get("tool").and_then(|x| x.as_str()).unwrap_or("lifi");
        hops.push(json!({ "tool": tool, "name": tool, "type": "swap" }));
    }
    hops.insert(
        0,
        json!({ "tool": "vapurr", "name": "0.25% vapurr", "type": "fee" }),
    );
    hops
}

fn fee_usd_of(est: &Value) -> String {
    let mut n = 0.0;
    if let Some(arr) = est.get("feeCosts").and_then(|x| x.as_array()) {
        for f in arr {
            if let Some(s) = f.get("amountUSD").and_then(|x| x.as_str()) {
                n += s.parse::<f64>().unwrap_or(0.0);
            }
        }
    }
    if n <= 0.0 {
        return String::new();
    }
    format!("${n:.2}")
}

fn gas_usd_of(est: &Value) -> String {
    let mut n = 0.0;
    if let Some(arr) = est.get("gasCosts").and_then(|x| x.as_array()) {
        for f in arr {
            if let Some(s) = f.get("amountUSD").and_then(|x| x.as_str()) {
                n += s.parse::<f64>().unwrap_or(0.0);
            }
        }
    }
    if n <= 0.0 {
        return String::new();
    }
    format!("${n:.2}")
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
        .timeout(Duration::from_secs(12))
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

    #[test]
    fn pack_lifi_includes_vapurr_fee_hop() {
        let raw = json!({
            "tool": "uniswap",
            "action": { "fromAmount": "1000000" },
            "estimate": {
                "toAmount": "997500",
                "toAmountMin": "990000",
                "executionDuration": 8,
                "feeCosts": [{ "amountUSD": "0.25" }],
                "gasCosts": [{ "amountUSD": "0.04" }]
            },
            "includedSteps": [{ "tool": "uniswap", "type": "swap", "toolDetails": { "name": "Uniswap" } }],
            "transactionRequest": { "to": "0xabc", "data": "0x", "value": "0x0", "chainId": 4663 }
        });
        let v = pack_lifi("swap", &raw, 4663, 4663, "USDG", "WETH", 6, 6, "1000000");
        assert_eq!(v["ok"], true);
        assert_eq!(v["fee_bps"], 25);
        assert_eq!(v["to_display"], "0.9975");
        let hops = v["hops"].as_array().unwrap();
        assert_eq!(hops[0]["tool"], "vapurr");
        assert!(v["tx"].is_object());
    }

    #[test]
    fn fallback_still_scoops() {
        let v = fallback_quote(
            "swap",
            CHAIN_ID,
            CHAIN_ID,
            USDG,
            USDG,
            "USDG",
            "USDG",
            6,
            6,
            1_000_000,
            "no route",
        );
        assert_eq!(v["ok"], true);
        assert_eq!(v["fee"], "0.25%");
        assert_eq!(v["estimate"], true);
        let out: u128 = v["to_amount"].as_str().unwrap().parse().unwrap();
        assert_eq!(out, 1_000_000 - scoop(1_000_000, 25));
    }
}
