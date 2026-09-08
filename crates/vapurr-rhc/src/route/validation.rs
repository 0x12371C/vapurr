use super::*;

fn token_matches(token: &Value, address: &str, chain: u64, decimals: u32) -> bool {
    token["address"].as_str().is_some_and(|a| addr_eq(a, address))
        && token["chainId"].as_u64() == Some(chain)
        && token["decimals"].as_u64() == Some(decimals as u64)
}

pub(super) fn action(raw: &Value, req: &QuoteReq) -> Result<(), String> {
    let a = &raw["action"];
    if a["fromChainId"].as_u64() != Some(req.from_chain) || a["toChainId"].as_u64() != Some(req.to_chain)
        || !token_matches(&a["fromToken"], &req.from_token, req.from_chain, req.from_dec)
        || !token_matches(&a["toToken"], &req.to_token, req.to_chain, req.to_dec)
        || a["fromAmount"].as_str().and_then(|s| s.parse::<u128>().ok()) != Some(req.from_amount)
        || !a["fromAddress"].as_str().is_some_and(|s| addr_eq(s, &req.from_address))
        || !a["toAddress"].as_str().is_some_and(|s| addr_eq(s, &req.from_address)) {
        return Err("Provider route differs from requested wallet, asset, amount or chain".into());
    }
    let min = raw["estimate"]["toAmountMin"].as_str().and_then(|s| s.parse::<u128>().ok()).unwrap_or(0);
    let out = raw["estimate"]["toAmount"].as_str().and_then(|s| s.parse::<u128>().ok()).unwrap_or(0);
    if min == 0 || out < min { return Err("Provider route has no valid minimum receipt".into()); }
    Ok(())
}

pub(super) fn transaction(tx: &Value, req: &QuoteReq) -> Result<(), String> {
    let chain = tx["chainId"].as_u64().or_else(|| tx["chainId"].as_str().and_then(parse_chain));
    if chain != Some(req.from_chain) { return Err("Transaction is on the wrong source chain".into()); }
    if !tx["to"].as_str().is_some_and(valid_address) { return Err("Invalid route contract".into()); }
    if let Some(from) = tx.get("from") {
        if !from.as_str().is_some_and(|s| addr_eq(s, &req.from_address)) { return Err("Transaction sender changed".into()); }
    }
    let data = tx["data"].as_str().and_then(|s| s.strip_prefix("0x")).ok_or("Invalid calldata")?;
    if data.len() < 8 || data.len() % 2 != 0 || data.len() > 256 * 1024
        || !data.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("Invalid calldata".into()); }
    let value = tx["value"].as_str().ok_or("Missing native transaction value")?;
    let wei = if let Some(hex) = value.strip_prefix("0x") { u128::from_str_radix(hex,16).ok() } else { value.parse().ok() }
        .ok_or("Invalid native transaction value")?;
    if is_native(&req.from_token) && wei != req.from_amount { return Err("Native value differs from input amount".into()); }
    Ok(())
}

pub(super) fn single_step(raw: &Value, req: &QuoteReq) -> Result<(), String> {
    let steps = raw["steps"].as_array().ok_or("Provider route has no steps")?;
    if steps.len() != 1 { return Err("This route needs multiple signed steps; select a single-transaction route".into()); }
    action(&steps[0], req)?;
    if raw["toAmount"] != steps[0]["estimate"]["toAmount"]
        || raw["toAmountMin"] != steps[0]["estimate"]["toAmountMin"] {
        return Err("Route output differs from executable step".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> QuoteReq {
        QuoteReq {kind:"swap",from_chain:1,to_chain:1,from_token:NATIVE.into(),to_token:ETH_USDC.into(),
            from_dec:18,to_dec:6,from_sym:"ETH".into(),to_sym:"USDC".into(),from_amount:10,
            from_address:"0x1111111111111111111111111111111111111111".into(),house:None}
    }
    #[test]
    fn transaction_rejects_changed_chain_native_value_and_invalid_calldata() {
        let req=request();
        let tx=json!({"chainId":1,"to":"0x2222222222222222222222222222222222222222","data":"0x12345678","value":"0xa"});
        assert!(transaction(&tx,&req).is_ok());
        for (field,value) in [("chainId",json!(8453)),("value",json!("0xb")),("to",json!(NATIVE)),("data",json!("0x123"))] {
            let mut changed=tx.clone(); changed[field]=value; assert!(transaction(&changed,&req).is_err(),"{field}");
        }
    }
    #[test]
    fn provider_response_must_match_recipient_amount_and_minimum() {
        let req=request();
        let raw=json!({"action":{"fromChainId":1,"toChainId":1,"fromToken":{"address":NATIVE,"chainId":1,"decimals":18},
            "toToken":{"address":ETH_USDC,"chainId":1,"decimals":6},"fromAmount":"10",
            "fromAddress":req.from_address,"toAddress":req.from_address},"estimate":{"toAmount":"20","toAmountMin":"19"}});
        assert!(action(&raw,&req).is_ok());
        for (field,value) in [("toAddress",json!("attacker")),("fromAmount",json!("11")),("toChainId",json!(43114))] {
            let mut changed=raw.clone(); changed["action"][field]=value; assert!(action(&changed,&req).is_err());
        }
        let mut changed=raw.clone(); changed["estimate"]["toAmountMin"]=json!("0"); assert!(action(&changed,&req).is_err());
        assert!(single_step(&json!({"steps":[raw.clone(),raw]}),&req).is_err());
    }
    #[test]
    fn amount_parser_never_silently_changes_input() {
        for amount in ["1.2x","1.234","1,000","-1","NaN","1e3","1..2","."] {
            assert!(parse_amount(amount,2).is_none(),"{amount}");
        }
        assert!(parse_amount("1",u32::MAX).is_none());
        assert!(parse_amount(&u128::MAX.to_string(),18).is_none());
        assert_eq!(parse_amount(".25",2),Some(25));
    }
}
