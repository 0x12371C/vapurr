#![recursion_limit = "256"]
//! Robinhood Chain is vapurr's home network.
//! Verified users never see these numbers. Advanced mode does.

pub const CAIP2: &str = "eip155:4663";
pub const CHAIN_ID: u64 = 4663;
pub const CHAIN_NAME: &str = "Robinhood Chain";
pub const RPC_HTTP: &str = "https://rpc.mainnet.chain.robinhood.com";
pub const EXPLORER: &str = "https://robinhoodchain.blockscout.com";

/// Robinhood Chain testnet (46630). Econ deploys here until mainnet has gas.
pub const TESTNET_CHAIN_ID: u64 = 46630;
pub const TESTNET_RPC_HTTP: &str = "https://rpc.testnet.chain.robinhood.com";
pub const TESTNET_EXPLORER: &str = "https://explorer.testnet.chain.robinhood.com";
pub const TESTNET_FAUCET: &str = "https://faucet.testnet.chain.robinhood.com";
/// Official Paxos testnet USDG (6 dec). Not mintable — econ deploys MockUsdg instead.
pub const TESTNET_USDG: &str = "0x7E955252E15c84f5768B83c41a71F9eba181802F";
pub const TESTNET_AMZN: &str = "0x5884aD2f920c162CFBbACc88C9C51AA75eC09E02";
/// PNS registry on testnet 46630. Root = deployer, this CA owns namehash("hood").
/// Old 0x7eAc… did not own the TLD. Do not deploy a second registry.
pub const TESTNET_PNS: &str = "0x13C9fCaB70e8f7eED688A5548B0E3849B1ae0fC4";
pub const NATIVE_SYMBOL: &str = "ETH";
pub const NATIVE_DECIMALS: u8 = 18;

pub const USDG: &str = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168";
pub const USDG_DECIMALS: u8 = 6;

/// PusdMarket on 4663. Empty until the first funded device deploys it.
pub const VAPURR_MARKET: &str = "";
pub const VAPURR_TOKEN: &str = "";
pub const PUSD_TOKEN: &str = "";
pub const WETH: &str = "0x0Bd7D308f8E1639FAb988df18A8011f41EAcAD73";

/// Canonical Uniswap v3 factory on 4663. PoolCreated is the on-chain pool list.
pub const UNI_V3_FACTORY: &str = "0x1f7d7550B1b028f7571E69A784071F0205FD2EfA";
pub const UNI_V2_FACTORY: &str = "0x8bceaa40b9acdfaedf85adf4ff01f5ad6517937f";
pub const SUSHI_V3_FACTORY: &str = "0xE51960f1B45f1C9FB6D166E6a884F866fC70433B";

/// Uniswap v4 on Robinhood Chain. Initial vapurr pools: VAPURR/PUSD and PUSD/USDG.
pub const UNI_V4_POOL_MANAGER: &str = "0x8366a39CC670B4001A1121B8F6A443A643e40951";
pub const UNI_V4_POSITION_MANAGER: &str = "0x58daec3116aae6D93017bAAea7749052E8a04fA7";
pub const UNI_V4_FEE_STABLE: u32 = 500; // 0.05% PUSD/USDG
pub const UNI_V4_TICK_STABLE: i32 = 10;
pub const UNI_V4_FEE_VOL: u32 = 3000; // 0.30% VAPURR/PUSD
pub const UNI_V4_TICK_VOL: i32 = 60;
pub const USDE: &str = "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34";
pub const PERMIT2: &str = "0x000000000022D473030F116dDEE9F6B43aC78BA3";
pub const NATIVE: &str = "0x0000000000000000000000000000000000000000";

/// Swap and bridge integrator scoop. Same as a MetaMask-style router fee.
pub const ROUTE_FEE_BPS: u32 = 25;
pub const ROUTE_FEE: f64 = 0.0025;
pub const ROUTE_INTEGRATOR: &str = "vapurr";

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
    }

    #[test]
    fn route_fee_is_25_bps() {
        assert_eq!(ROUTE_FEE_BPS, 25);
        assert!((ROUTE_FEE - 0.0025).abs() < 1e-12);
        assert_eq!(route::scoop(10_000_000, ROUTE_FEE_BPS), 25_000);
    }
}
