use std::time::Duration;

use k256::ecdsa::SigningKey;
use vapurr_wallet::Address;

use crate::error::RelayError;
use crate::signer;

pub struct Config {
    pub rpc_url: String,
    pub chain_id: u64,
    pub forwarder: Address,
    pub relayer_key: SigningKey,
    pub relayer_address: Address,
    pub bind_addr: String,
    /// Flush a batch once it hits this many pending requests...
    pub batch_max_size: usize,
    /// ...or once the oldest pending request has waited this long,
    /// whichever comes first. A small batch that ages out still gets
    /// submitted — nobody's gasless transaction should wait forever for
    /// strangers to show up and subsidize it.
    pub batch_max_wait: Duration,
    /// Basis points of a user's own solo cost they're charged for
    /// sponsorship (5_000 = pay half of what self-paying would cost).
    /// Billing collection itself isn't wired up yet — see RELAY.md.
    pub user_fee_bps: u64,
    /// Priority fee added on top of the current base fee for the
    /// relayer's own submission tx. Tune from real RHC fee history.
    pub priority_fee_wei: u128,
}

fn env(name: &str) -> Result<String, RelayError> {
    std::env::var(name).map_err(|_| RelayError::Config(format!("{name} is not set")))
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn addr_from_hex(s: &str) -> Result<Address, RelayError> {
    let bytes = hex::decode(s.trim().trim_start_matches("0x"))
        .map_err(|_| RelayError::Config(format!("not valid hex: {s}")))?;
    if bytes.len() != 20 {
        return Err(RelayError::Config(format!("not a 20-byte address: {s}")));
    }
    let mut a = [0u8; 20];
    a.copy_from_slice(&bytes);
    Ok(Address(a))
}

impl Config {
    pub fn from_env() -> Result<Self, RelayError> {
        let local = std::env::var("VAPURR_RELAY_LOCAL").as_deref() == Ok("1");

        let rpc_url = if local {
            env_or("VAPURR_RELAY_RPC_URL", vapurr_rhc::TESTNET_RPC_HTTP)
        } else {
            env("VAPURR_RELAY_RPC_URL")?
        };
        let chain_id: u64 = env_or("VAPURR_RELAY_CHAIN_ID", &vapurr_rhc::TESTNET_CHAIN_ID.to_string())
            .parse()
            .map_err(|_| RelayError::Config("VAPURR_RELAY_CHAIN_ID must be a number".into()))?;

        let forwarder = addr_from_hex(&env("VAPURR_RELAY_FORWARDER_ADDRESS")?)?;

        let relayer_key = signer::key_from_hex(&env("VAPURR_RELAY_PRIVATE_KEY")?)
            .map_err(RelayError::Config)?;
        let relayer_address = signer::address_of(&relayer_key);

        let bind_addr = env_or("VAPURR_RELAY_BIND_ADDR", "127.0.0.1:8792");

        let batch_max_size: usize = env_or("VAPURR_RELAY_BATCH_MAX_SIZE", "32")
            .parse()
            .map_err(|_| RelayError::Config("VAPURR_RELAY_BATCH_MAX_SIZE must be a number".into()))?;
        let batch_max_wait_ms: u64 = env_or("VAPURR_RELAY_BATCH_MAX_WAIT_MS", "2000")
            .parse()
            .map_err(|_| RelayError::Config("VAPURR_RELAY_BATCH_MAX_WAIT_MS must be a number".into()))?;
        let user_fee_bps: u64 = env_or("VAPURR_RELAY_USER_FEE_BPS", "5000")
            .parse()
            .map_err(|_| RelayError::Config("VAPURR_RELAY_USER_FEE_BPS must be a number".into()))?;
        let priority_fee_wei: u128 = env_or("VAPURR_RELAY_PRIORITY_FEE_WEI", "1000000000")
            .parse()
            .map_err(|_| RelayError::Config("VAPURR_RELAY_PRIORITY_FEE_WEI must be a number".into()))?;

        Ok(Self {
            rpc_url,
            chain_id,
            forwarder,
            relayer_key,
            relayer_address,
            bind_addr,
            batch_max_size,
            batch_max_wait: Duration::from_millis(batch_max_wait_ms),
            user_fee_bps,
            priority_fee_wei,
        })
    }
}
