use super::*;

/// The device deployment book, captured once per request. Never rewrite it here.
#[derive(Clone, Debug)]
pub(super) struct HouseBook {
    pub equity: String,
    pub cash: String,
    pub swapper: String,
    pub vapurr: String,
}

impl HouseBook {
    pub fn assets(chain: u64) -> Vec<Value> {
        let Some(base) = std::env::var_os("LOCALAPPDATA") else { return Vec::new(); };
        let path = std::path::PathBuf::from(base).join("vapurr").join("market.json");
        let Ok(bytes) = std::fs::read(path) else { return Vec::new(); };
        let bytes = bytes.strip_prefix(&[0xef,0xbb,0xbf]).unwrap_or(&bytes);
        let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return Vec::new(); };
        let expected = if chain == TESTNET_CHAIN_ID { "testnet" } else if chain == CHAIN_ID { "mainnet" } else { return Vec::new(); };
        if v["net"] != expected || v["gen"].as_u64().unwrap_or(0) < 5 { return Vec::new(); }
        let mut out = Vec::new();
        for (field, symbol, name, decimals) in [
            ("gv","gV","Staked VAPURR",18), ("spusd","sPUSD","Savings PUSD",18),
            ("eeth","eETH","Test ETH pool asset",18), ("envda","eNVDA","Test NVIDIA pool asset",18),
            ("eamd","eAMD","Test AMD pool asset",18), ("eamzn","eAMZN","Test Amazon pool asset",18),
            ("etsla","eTSLA","Test Tesla pool asset",18), ("enflx","eNFLX","Test Netflix pool asset",18),
            ("epltr","ePLTR","Test Palantir pool asset",18),
        ] {
            if let Some(address) = v[field].as_str().filter(|a| valid_address(a)) {
                push_tok(&mut out,chain,address,symbol,name,decimals);
            }
        }
        out
    }
    fn from_json(v: &Value, chain: u64) -> Option<Self> {
        let book_chain = match v.get("net")?.as_str()? {
            "testnet" => TESTNET_CHAIN_ID,
            "mainnet" => CHAIN_ID,
            _ => return None,
        };
        if chain != book_chain || v.get("gen")?.as_u64()? < 5 { return None; }
        let address = |name| v.get(name)?.as_str().filter(|a| valid_address(a)).map(str::to_owned);
        let book = Self {
            equity: address("wgv")?, cash: address("pusd")?,
            swapper: address("swap")?, vapurr: address("vapurr")?,
        };
        if addr_eq(&book.equity, &book.cash) || addr_eq(&book.equity, &book.vapurr) { return None; }
        Some(book)
    }

    pub fn load(chain: u64) -> Option<Self> {
        let path = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
            .join("vapurr").join("market.json");
        let bytes = std::fs::read(path).ok()?;
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        let v = serde_json::from_slice(bytes).ok()?;
        Self::from_json(&v, chain)
    }

    pub fn is_pair(&self, req: &QuoteReq) -> bool {
        (addr_eq(&req.from_token, &self.equity) && addr_eq(&req.to_token, &self.cash))
            || (addr_eq(&req.from_token, &self.cash) && addr_eq(&req.to_token, &self.equity))
    }

    /// The address book is a pointer, not evidence of the contract's token pair.
    pub fn verify(&self, chain: u64) -> Result<u128, String> {
        let rpc = crate::rpc::Rpc::at_timeout(rpc_for(chain).ok_or("unsupported House chain")?, 6);
        for (signature, expected) in [("wgV()", &self.equity), ("pusd()", &self.cash)] {
            let data = format!("0x{}", hex::encode(&keccak(signature.as_bytes())[..4]));
            let ret = rpc.eth_call(QUOTE_ADDR, Some(&self.swapper), &data).map_err(|e| e.to_string())?;
            let word = ret.strip_prefix("0x").ok_or("invalid House token response")?;
            if word.len() != 64 || !word[..24].bytes().all(|b| b == b'0')
                || !word[24..].eq_ignore_ascii_case(expected.trim_start_matches("0x")) {
                return Err(format!("House {signature} differs from the deployment book"));
            }
        }
        let data = format!("0x{}", hex::encode(&keccak(b"fee()")[..4]));
        let ret = rpc.eth_call(QUOTE_ADDR, Some(&self.swapper), &data).map_err(|e| e.to_string())?;
        let fee = parse_ret_u128(&ret).ok_or("invalid House fee")?;
        if fee == 0 || fee > 1_000_000 { return Err("invalid House fee".into()); }
        Ok(fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "read-only RPC check against the current device deployment"]
    fn deployed_house_pair_matches_book() {
        let book = HouseBook::load(TESTNET_CHAIN_ID).expect("deployed testnet House book");
        let fee = book.verify(TESTNET_CHAIN_ID).expect("on-chain House pair verification");
        eprintln!("House {}: wgV={}, PUSD={}, fee={} ppm",book.swapper,book.equity,book.cash,fee);
    }
    #[test]
    fn deployment_book_requires_wrapped_equity_and_matching_network() {
        let mut v = json!({"gen":5,"net":"testnet","wgv":"0x1111111111111111111111111111111111111111",
            "pusd":"0x2222222222222222222222222222222222222222","swap":"0x3333333333333333333333333333333333333333",
            "vapurr":"0x4444444444444444444444444444444444444444"});
        assert!(HouseBook::from_json(&v, TESTNET_CHAIN_ID).is_some());
        assert!(HouseBook::from_json(&v, CHAIN_ID).is_none());
        v["wgv"] = v["vapurr"].clone();
        assert!(HouseBook::from_json(&v, TESTNET_CHAIN_ID).is_none());
    }
}
