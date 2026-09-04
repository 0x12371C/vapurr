#![recursion_limit = "256"]
//! Robinhood Chain is vapurr's home network.
//! Verified users never see these numbers. Advanced mode does.

pub const CAIP2: &str = "eip155:4663";
/// KetPay / $PUSD spend in v1.2 is this net only. We have tokens here.
pub const TESTNET_CAIP2: &str = "eip155:46630";
pub const CHAIN_ID: u64 = 4663;
pub const CHAIN_NAME: &str = "Robinhood Chain";
pub const RPC_HTTP: &str = "https://rpc.mainnet.chain.robinhood.com";
pub const EXPLORER: &str = "https://robinhoodchain.blockscout.com";

/// Robinhood Chain testnet (46630). Econ deploys here until mainnet has gas.
pub const TESTNET_CHAIN_ID: u64 = 46630;
/// EIP-1193 `eth_chainId` for testnet. `0xb616` is 46614 — do not typo this.
pub const TESTNET_CHAIN_ID_HEX: &str = "0xb626";
pub const TESTNET_RPC_HTTP: &str = "https://rpc.testnet.chain.robinhood.com";
pub const TESTNET_EXPLORER: &str = "https://explorer.testnet.chain.robinhood.com";
pub const TESTNET_FAUCET: &str = "https://faucet.testnet.chain.robinhood.com";
/// Official Paxos testnet USDG (6 dec). Not mintable — econ deploys MockUsdg instead.
pub const TESTNET_USDG: &str = "0x7E955252E15c84f5768B83c41a71F9eba181802F";
pub const TESTNET_AMZN: &str = "0x5884aD2f920c162CFBbACc88C9C51AA75eC09E02";
/// PNS registry on testnet 46630. Root = deployer, this CA owns namehash("hood").
/// Old 0x7eAc… did not own the TLD. Do not deploy a second registry.
pub const TESTNET_PNS: &str = "0x13C9fCaB70e8f7eED688A5548B0E3849B1ae0fC4";
/// Fresh gen-4 PusdMarket on 46630. Deployed 2026-09-04 from this device.
/// Retired 0x447F… / 0x435C… / 0x59bB… / 0x159d… do not count.
pub const TESTNET_MARKET: &str = "0x47Aca5292423e2133A3eE983aB38291de3983617";
pub const TESTNET_VAPURR: &str = "0xD4b36DDe47d6294274193d1Bf546E5C32c1E7585";
pub const TESTNET_PUSD: &str = "0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e";
pub const TESTNET_OUTBID: &str = "";
/// Ketcharts $PUSD listing board (`KetList.sol`). Empty until this device deploys it.
pub const TESTNET_KETLIST: &str = "";
/// Isolated $PUSD vault (`PusdLoop.sol`). Deployed 2026-09-04 on gen-4.
pub const TESTNET_LOOP: &str = "0x89E17eefa58B99d025145970c0FBAe7768a14521";
/// House v4 exact-in swapper. Live 2026-09-04.
pub const TESTNET_SWAP: &str = "0x6304419b838Efb12D0Cdf931dd9579c5b4084dD2";
/// House Uniswap v4 CL (`HouseLp.sol`) $VAPURR / $PUSD. Seeded 2026-09-04.
pub const TESTNET_HOUSE: &str = "0x667bFcAF9D3Ee809336788Bf52511D35AE9C1bf7";
/// Official testnet stock tokens. Ops liquidity — not the house book.
pub const TESTNET_TSLA: &str = "0xC9f9c86933092BbbfFF3CCb4b105A4A94bf3Bd4E";
pub const TESTNET_AMD: &str = "0x71178BAc73cBeb415514eB542a8995b82669778d";
pub const TESTNET_NFLX: &str = "0x3b8262A63d25f0477c4DDE23F83cfe22Cb768C93";
pub const TESTNET_PLTR: &str = "0x1FBE1a0e43594b3455993B5dE5Fd0A7A266298d0";
pub const TESTNET_STOCKS: &[(&str, &str)] = &[
    ("AMZN", TESTNET_AMZN),
    ("TSLA", TESTNET_TSLA),
    ("AMD", TESTNET_AMD),
    ("NFLX", TESTNET_NFLX),
    ("PLTR", TESTNET_PLTR),
];
/// Old mintable mock. Not in the fresh book.
pub const TESTNET_MOCK_USDG: &str = "";
pub const NATIVE_SYMBOL: &str = "ETH";
pub const NATIVE_DECIMALS: u8 = 18;

pub const USDG: &str = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168";
pub const USDG_DECIMALS: u8 = 6;

/// PusdMarket on mainnet 4663. Empty until the vanity deploy tx lands.
pub const VAPURR_MARKET: &str = "";
pub const VAPURR_TOKEN: &str = "";
pub const PUSD_TOKEN: &str = "";
/// Pinned CREATE nonce-0 vanity for the first mainnet PusdMarket.
/// Deployer first outbound tx **must** be `PusdMarket` (receiving ETH does not bump nonce).
/// Do not copy these into `VAPURR_MARKET` until the receipt is on 4663.
pub const MAINNET_DEPLOYER: &str = "0x48043E2Cda4D403c10dbB1F4614c4F6ad0f9AeA5";
pub const MAINNET_MARKET_VANITY: &str = "0xC47f00D61F8379337f9fb42E6DcC695AE2d6EBD2";
pub const MAINNET_VAPURR_VANITY: &str = "0x526323EfAe2465e9e5b8Bad6277A8F10B1D6c75e";
pub const MAINNET_PUSD_VANITY: &str = "0x98c35e266f4E38F90580cA074a5f382D49400F5f";
pub const VAPURR_LOOP: &str = "";
pub const WETH: &str = "0x0Bd7D308f8E1639FAb988df18A8011f41EAcAD73";

/// Official Robinhood Stock Tokens on mainnet 4663. Address is the token, not the ticker.
/// Source: Robinhood contracts registry. Lookalikes that reuse these tickers are not these.
pub const STOCKS: &[(&str, &str, &str)] = &[
    ("AAPL", "Apple", "0xaF3D76f1834A1d425780943C99Ea8A608f8a93f9"),
    ("AMD", "AMD", "0x86923f96303D656E4aa86D9d42D1e57ad2023fdC"),
    ("AMZN", "Amazon", "0x12f190a9F9d7D37a250758b26824B97CE941bF54"),
    ("COIN", "Coinbase", "0x6330D8C3178a418788dF01a47479c0ce7CCF450b"),
    ("GOOGL", "Alphabet Class A", "0x2e0847E8910a9732eB3fb1bb4b70a580ADAD4FE3"),
    ("INTC", "Intel", "0xc72b96e0E48ecd4DC75E1e45396e26300BC39681"),
    ("META", "Meta Platforms", "0xc0D6457C16Cc70d6790Dd43521C899C87ce02f35"),
    ("MSFT", "Microsoft", "0xe93237C50D904957Cf27E7B1133b510C669c2e74"),
    ("NVDA", "NVIDIA", "0xd0601CE157Db5bdC3162BbaC2a2C8aF5320D9EEC"),
    ("ORCL", "Oracle", "0xb0992820E760d836549ba69BC7598b4af75dEE03"),
    ("PLTR", "Palantir Technologies", "0x894E1EC2D74FFE5AEF8Dc8A9e84686acCB964F2A"),
    ("TSLA", "Tesla", "0x322F0929c4625eD5bAd873c95208D54E1c003b2d"),
    ("SPY", "SPDR S&P 500 ETF", "0x117cc2133c37B721F49dE2A7a74833232B3B4C0C"),
    ("QQQ", "Invesco QQQ", "0xD5f3879160bc7c32ebb4dC785F8a4F505888de68"),
];

/// Canonical Uniswap v3 factory on 4663. PoolCreated is the on-chain pool list.
pub const UNI_V3_FACTORY: &str = "0x1f7d7550B1b028f7571E69A784071F0205FD2EfA";
pub const UNI_V2_FACTORY: &str = "0x8bceaa40b9acdfaedf85adf4ff01f5ad6517937f";
pub const SUSHI_V3_FACTORY: &str = "0xE51960f1B45f1C9FB6D166E6a884F866fC70433B";

/// Uniswap v4 on Robinhood Chain.
/// House AMM is **only** $VAPURR / $PUSD. No house pool vs USDG, WETH, USDE, or ETH.
/// Dumping V or P into Robinhood's dollar or gas is not the product. $PUSD has to be
/// the dollar; Euler-style supply/borrow on $PUSD is how that dollar gets its own depth.
pub const UNI_V4_POOL_MANAGER: &str = "0x8366a39CC670B4001A1121B8F6A443A643e40951";
pub const UNI_V4_POSITION_MANAGER: &str = "0x58daec3116aae6D93017bAAea7749052E8a04fA7";
/// Not a house pool. Do not seed $PUSD/USDG.
pub const UNI_V4_FEE_STABLE: u32 = 500;
pub const UNI_V4_TICK_STABLE: i32 = 10;
/// House book: $VAPURR / $PUSD 0.30%.
pub const UNI_V4_FEE_VOL: u32 = 3000;
pub const UNI_V4_TICK_VOL: i32 = 60;
pub const USDE: &str = "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34";
pub const PERMIT2: &str = "0x000000000022D473030F116dDEE9F6B43aC78BA3";
pub const NATIVE: &str = "0x0000000000000000000000000000000000000000";

/// Swap/bridge protocol fee. 25 bps buys $VAPURR. The user's route is not cut.
/// A small slice is refunded in $VAPURR; the rest burns to mint $PUSD.
pub const ROUTE_FEE_BPS: u32 = 25;
pub const ROUTE_FEE: f64 = 0.0025;
/// User rebate in $VAPURR. 5 bps of notional — the route itself is not stopped.
pub const ROUTE_REFUND_BPS: u32 = 5;
/// House book ($VAPURR/$PUSD) user rebate in $VAPURR. 3 bps of notional.
pub const HOUSE_REFUND_BPS: u32 = 3;
pub const ROUTE_INTEGRATOR: &str = "vapurr";
/// Mint spread assumed when estimating $PUSD created from the burned slice (market min 2%).
pub const ROUTE_FEE_MINT_SPREAD_BPS: u32 = 200;

pub const ETH_RPC: &str = "https://ethereum-rpc.publicnode.com";
pub const BASE_RPC: &str = "https://mainnet.base.org";
pub const ARB_RPC: &str = "https://arb1.arbitrum.io/rpc";

pub fn rpc_http(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        4663 => Some(RPC_HTTP),
        46630 => Some(TESTNET_RPC_HTTP),
        1 => Some(ETH_RPC),
        43114 => Some(AVAX_RPC),
        8453 => Some(BASE_RPC),
        42161 => Some(ARB_RPC),
        _ => None,
    }
}

pub const ENTRY_POINT_V07: &str = "0x0000000071727De22E5E9d8BAf0edAc6f37da032";
pub const ENTRY_POINT_V08: &str = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108";
pub const SENDER_CREATOR_V07: &str = "0xEFC2c1444eBCC4Db75e7613d20C6a62fF67A167C";

pub const AVAX_CAIP2: &str = "eip155:43114";
pub const AVAX_CHAIN_ID: u64 = 43114;
pub const AVAX_RPC: &str = "https://api.avax.network/ext/bc/C/rpc";
pub const AVAX_NATIVE_USDC: &str = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E";
pub const AVAX_USDC_DECIMALS: u8 = 6;
pub const RAIN_CONTROLLER: &str = "0x5bbb435eC20154f016ee96041C3fDFe46354603a";
pub const RAIN_LIQUIDITY: &str = "0x5d699a30bf073da1c248694eff9a94ddeff146e1";

pub fn usdg_to_minor(dollars: f64) -> u128 {
    if dollars <= 0.0 {
        return 0;
    }
    (dollars * 1_000_000.0).round() as u128
}

pub fn minor_to_usdg(minor: u128) -> f64 {
    minor as f64 / 1_000_000.0
}

pub fn format_usd(minor: u128) -> String {
    format!("${:.2}", minor_to_usdg(minor))
}

/// `$PUSD` / `$VAPURR` are 18 decimals. USDG is 6. Two-decimal dollar label.
pub fn format_usd_units(minor: u128, decimals: u8) -> String {
    if decimals == 0 {
        return format!("${minor}.00");
    }
    if decimals == USDG_DECIMALS {
        return format_usd(minor);
    }
    let scale = 10u128.saturating_pow(decimals as u32);
    if scale == 0 {
        return format!("${minor}.00");
    }
    format!("${:.2}", minor as f64 / scale as f64)
}

pub mod index;
pub mod liq;
pub mod rpc;
pub mod route;
pub mod scan;
pub use rpc::{ChainFeed, Rpc};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_is_robinhood() {
        assert_eq!(CHAIN_ID, 4663);
        assert_eq!(CAIP2, "eip155:4663");
        assert_eq!(USDG.len(), 42);
    }

    #[test]
    fn usd_roundtrip() {
        assert_eq!(usdg_to_minor(1.20), 1_200_000);
        assert_eq!(format_usd(1_200_000), "$1.20");
        assert_eq!(format_usd_units(1_200_000, 6), "$1.20");
        assert_eq!(
            format_usd_units(1_200_000_000_000_000_000, 18),
            "$1.20"
        );
    }

    #[test]
    fn canonical_testnet_book_is_set() {
        assert_eq!(TESTNET_CHAIN_ID, 46630);
        assert_eq!(TESTNET_CHAIN_ID_HEX, "0xb626");
        assert_eq!(
            format!("0x{:x}", TESTNET_CHAIN_ID),
            TESTNET_CHAIN_ID_HEX
        );
        assert_eq!(TESTNET_CAIP2, "eip155:46630");
        assert_eq!(TESTNET_PNS.len(), 42);
        assert_eq!(TESTNET_MARKET.len(), 42);
        assert_eq!(TESTNET_PUSD.len(), 42);
        assert_eq!(TESTNET_VAPURR.len(), 42);
        assert!(TESTNET_OUTBID.is_empty());
        assert!(TESTNET_MOCK_USDG.is_empty());
        assert!(VAPURR_MARKET.is_empty());
        assert!(PUSD_TOKEN.is_empty());
        assert_eq!(MAINNET_MARKET_VANITY.len(), 42);
        assert!(MAINNET_MARKET_VANITY
            .to_ascii_lowercase()
            .starts_with("0xc47f00d"));
        assert_eq!(MAINNET_DEPLOYER.len(), 42);
        assert_eq!(MAINNET_VAPURR_VANITY.len(), 42);
        assert_eq!(MAINNET_PUSD_VANITY.len(), 42);
        assert_eq!(TESTNET_LOOP.len(), 42);
        assert_eq!(TESTNET_SWAP.len(), 42);
        assert!(TESTNET_KETLIST.is_empty());
        assert_eq!(TESTNET_HOUSE.len(), 42);
        assert!(VAPURR_LOOP.is_empty());
        assert_eq!(TESTNET_STOCKS.len(), 5);
        assert_eq!(STOCKS.len(), 14);
        assert!(STOCKS.iter().any(|(s, _, a)| *s == "NVDA" && a.len() == 42));
        assert!(STOCKS.iter().any(|(s, _, _)| *s == "TSLA"));
        assert!(STOCKS.iter().any(|(s, _, _)| *s == "MSFT"));
        assert!(STOCKS.iter().any(|(s, _, _)| *s == "PLTR"));
    }

    #[test]
    fn route_fee_is_25_bps() {
        assert_eq!(ROUTE_FEE_BPS, 25);
        assert!((ROUTE_FEE - 0.0025).abs() < 1e-12);
        assert_eq!(route::scoop(10_000_000, ROUTE_FEE_BPS), 25_000);
        assert_eq!(ROUTE_REFUND_BPS, 5);
        assert!(ROUTE_REFUND_BPS < ROUTE_FEE_BPS);
        assert_eq!(ROUTE_FEE_MINT_SPREAD_BPS, 200);
        assert_eq!(rpc_http(4663), Some(RPC_HTTP));
        assert_eq!(rpc_http(46630), Some(TESTNET_RPC_HTTP));
        assert!(rpc_http(999).is_none());
    }
}
