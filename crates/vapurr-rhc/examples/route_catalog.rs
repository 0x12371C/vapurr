//! Read-only route catalog inspection for UI integration checks.
fn main() {
    let mut catalog = vapurr_rhc::route::tokens(Some("46630"));
    if std::env::args().any(|a| a == "--verify") {
        let rpc = vapurr_rhc::rpc::Rpc::at_timeout(vapurr_rhc::TESTNET_RPC_HTTP, 6);
        let chain = rpc.call("eth_chainId", serde_json::json!([])).expect("RPC chain ID");
        assert_eq!(chain.as_str(), Some("0xb626"), "wrong RPC chain");
        for token in catalog["tokens"].as_array_mut().unwrap() {
            if token["native"] == true { token["verified"] = true.into(); continue; }
            let address = token["address"].as_str().unwrap().to_owned();
            let code = rpc.call("eth_getCode", serde_json::json!([address,"latest"]));
            let decimals = rpc.eth_call(vapurr_rhc::NATIVE, Some(&address), "0x313ce567");
            let symbol = rpc.eth_call(vapurr_rhc::NATIVE, Some(&address), "0x95d89b41");
            token["verified"] = serde_json::json!(code.as_ref().ok().and_then(|v|v.as_str()).is_some_and(|s|s.len()>2));
            assert_eq!(token["verified"], true, "missing contract: {address}");
            let actual = u64::from_str_radix(decimals.as_ref().expect("token decimals").trim_start_matches("0x"), 16).unwrap();
            assert_eq!(Some(actual), token["decimals"].as_u64(), "wrong decimals: {address}");
            token["decimals_rpc"] = serde_json::json!(decimals.ok());
            token["symbol_rpc"] = serde_json::json!(symbol.ok());
        }
    }
    println!("{}", catalog);
}
