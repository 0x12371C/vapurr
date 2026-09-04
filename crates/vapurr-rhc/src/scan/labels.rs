use super::*;


pub(crate) fn addr_label(a: &str) -> String {
    let a = a.trim();
    if a.is_empty() {
        return String::new();
    }
    if a.eq_ignore_ascii_case(USDG) {
        return "USDG".into();
    }
    if a.eq_ignore_ascii_case(WETH) {
        return "WETH".into();
    }
    if a.eq_ignore_ascii_case(USDE) {
        return "USDe".into();
    }
    if a.eq_ignore_ascii_case(UNI_V3_FACTORY) {
        return "Uniswap V3 Factory".into();
    }
    if a.eq_ignore_ascii_case(UNI_V2_FACTORY) {
        return "Uniswap V2 Factory".into();
    }
    if a.eq_ignore_ascii_case(SUSHI_V3_FACTORY) {
        return "Sushi V3 Factory".into();
    }
    if a.eq_ignore_ascii_case(UNI_V4_POOL_MANAGER) {
        return "Uniswap v4 PoolManager".into();
    }
    if a.eq_ignore_ascii_case(UNI_V4_POSITION_MANAGER) {
        return "Uniswap v4 PositionManager".into();
    }
    if a.eq_ignore_ascii_case(PERMIT2) {
        return "Permit2".into();
    }
    if a.eq_ignore_ascii_case(ENTRY_POINT_V07) {
        return "EntryPoint v0.7".into();
    }
    if a.eq_ignore_ascii_case(ENTRY_POINT_V08) {
        return "EntryPoint v0.8".into();
    }
    if a.eq_ignore_ascii_case(NATIVE) {
        return "native".into();
    }
    if let Some(hit) = crate::liq::token_hit(a) {
        if let Some(sym) = hit.get("symbol").and_then(|v| v.as_str()) {
            if !sym.is_empty() {
                return sym.to_string();
            }
        }
    }
    String::new()
}


pub(crate) fn label_tx_addrs(v: &mut Value) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    if let Some(a) = obj.get("from").and_then(|x| x.as_str()).map(|s| s.to_string()) {
        let l = addr_label(&a);
        if !l.is_empty() {
            obj.insert("from_label".into(), json!(l));
        }
    }
    if let Some(a) = obj.get("to").and_then(|x| x.as_str()).map(|s| s.to_string()) {
        let l = addr_label(&a);
        if !l.is_empty() {
            obj.insert("to_label".into(), json!(l));
        }
    }
}


pub(crate) fn method_name(input: &str) -> String {
    method_name_val(input, 0)
}


pub(crate) fn method_name_val(input: &str, value: u128) -> String {
    let sel = input.get(..10).unwrap_or("").to_ascii_lowercase();
    match sel.as_str() {
        "" | "0x" | "0x0" => {
            if value > 0 {
                "transfer".into()
            } else {
                "call".into()
            }
        }
        "0xa9059cbb" => "transfer".into(),
        "0x23b872dd" => "transferFrom".into(),
        "0x095ea7b3" => "approve".into(),
        "0x70a08231" => "balanceOf".into(),
        "0x18160ddd" => "totalSupply".into(),
        "0x313ce567" => "decimals".into(),
        "0x06fdde03" => "name".into(),
        "0x95d89b41" => "symbol".into(),
        "0x2e1a7d4d" => "withdraw".into(),
        "0xd0e30db0" => "deposit".into(),
        "0x3593564c" | "0xb61d27f6" | "0x24856bc3" => "execute".into(),
        "0x40c10f19" | "0x6a627842" | "0xa0712d68" | "0x1249c58b" => "mint".into(),
        "0x42966c68" => "burn".into(),
        "0xa22cb465" => "setApprovalForAll".into(),
        "0x42842e0e" | "0xb88d4fde" => "safeTransferFrom".into(),
        "0xf242432a" => "safeTransferFrom".into(),
        "0x2eb2c2d6" => "safeBatchTransferFrom".into(),
        "0x38ed1739" => "swapExactTokensForTokens".into(),
        "0x7ff36ab5" => "swapExactETHForTokens".into(),
        "0x18cbafe5" => "swapExactTokensForETH".into(),
        "0x5c11d795" => "swapExactTokensForTokensSupportingFeeOnTransferTokens".into(),
        "0xfb3bdb41" => "swapETHForExactTokens".into(),
        "0x4a25d94a" => "swapTokensForExactETH".into(),
        "0x8803dbee" => "swapTokensForExactTokens".into(),
        "0x022c0d9f" | "0x128acb08" => "swap".into(),
        "0x414bf389" | "0x04e45aaf" => "exactInputSingle".into(),
        "0xc04b8d59" => "exactInput".into(),
        "0xdb3e2198" => "exactOutputSingle".into(),
        "0xf28c0498" => "exactOutput".into(),
        "0xac9650d8" | "0x5ae401dc" => "multicall".into(),
        "0x12aa3caf" | "0x7c025200" | "0x5f575529" => "swap".into(),
        "0xd9627aa4" => "sellToUniswap".into(),
        "0x83bd37f9" => "unxswapTo".into(),
        "0x0502b1c5" => "unoswap".into(),
        "0x4e71d92d" | "0x379607f5" => "claim".into(),
        "0x3d18b912" => "getReward".into(),
        other if other.len() == 10 && other.starts_with("0x") => other.to_string(),
        _ => "call".into(),
    }
}

