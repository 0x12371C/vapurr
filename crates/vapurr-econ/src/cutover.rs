//! Atomic canonical-Lithe deployment and local book replacement.
//!
//! The factory verifies the old V supply, funds direct V conversion inventory,
//! creates old-Lithe -> V inventory convert -> canonical-Lithe PUSD mint, and deploys Oliver.
//! Does not deploy wgV/House/PairConfig; remittance must be set post-deploy.
//! This module only prepares and sends a user-approved device-wallet transaction.

use vapurr_wallet::tx::{abi_addr, abi_u256, decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn, hex0x};
use vapurr_wallet::{addr_from_hex, Address};

use crate::{econ_rpc, Client, EconError, DEC, GEN, MIN_GAS_WEI};

const FACTORY_HEX: &str = include_str!("canonical_lithe_factory.hex");
/// Initial canonical V held by canonical Lithe for immediate PUSD redemption.
/// Direct-V conversion inventory is separately equal to the observed legacy V supply.
pub(crate) const BOOTSTRAP_V: u128 = 100_000 * DEC;
/// EIP-3860 maximum initcode length, in bytes.
const MAX_INITCODE_BYTES: usize = 49_152;

impl Client {
    /// Deploy the complete successor book in one transaction and atomically replace
    /// the local address book only after the receipt and every child address verify.
    pub(crate) fn cutover_deploy(&mut self) -> Result<String, EconError> {
        if self.cfg.gen >= GEN && !self.cfg.cutover_factory.is_empty() {
            if self.live_ca(&self.cfg.cutover_factory).is_some() {
                return Ok(self.cfg.cutover_factory.clone());
            }
        }

        let legacy_market = self.live_market().ok_or(EconError::NotLive)?;
        let legacy_v = addr_from_hex(&self.cfg.vapurr).ok_or_else(|| EconError::Rpc("legacy V".into()))?;
        let legacy_pusd = addr_from_hex(&self.cfg.pusd).ok_or_else(|| EconError::Rpc("legacy PUSD".into()))?;
        let legacy_supply = self.token_total_supply(legacy_v)?;
        if legacy_supply == 0 {
            return Err(EconError::Rpc("legacy V supply is zero".into()));
        }

        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(econ_rpc)?;
        if eth < MIN_GAS_WEI {
            return Err(EconError::NeedGas);
        }

        let create = factory_create_data(legacy_market, legacy_v, legacy_supply, BOOTSTRAP_V, DEC)?;
        let hash = self.send(None, &create)?;
        let receipt = self.wait(&hash)?;
        if receipt.get("status").and_then(|v| v.as_str()).unwrap_or("0x0") != "0x1" {
            return Err(EconError::Rpc("canonical Lithe deploy reverted".into()));
        }
        let factory_hex = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no factory address".into()))?;
        let factory = addr_from_hex(factory_hex).ok_or_else(|| EconError::Rpc("bad factory address".into()))?;

        let canonical_v = self.contract_address(factory, "canonicalV()")?;
        let market = self.contract_address(factory, "market()")?;
        let loop_vault = self.contract_address(factory, "loop()")?;
        let converter = self.contract_address(factory, "converter()")?;
        let migrator = self.contract_address(factory, "migrator()")?;
        let policy = self.contract_address(factory, "policy()")?;
        let gv = self.contract_address(factory, "gV()")?;
        let pusd = self.contract_address(market, "pusd()")?;

        // Preserve the old identities for migration/audit, then make every live
        // economic surface point at the successor book. Old PUSD-app contracts are
        // cleared rather than silently receiving a mixed-token configuration.
        self.cfg.gen = GEN;
        self.cfg.legacy_market = legacy_market.to_checksum();
        self.cfg.legacy_vapurr = legacy_v.to_checksum();
        self.cfg.legacy_pusd = legacy_pusd.to_checksum();
        self.cfg.cutover_factory = factory.to_checksum();
        self.cfg.market = market.to_checksum();
        self.cfg.vapurr = canonical_v.to_checksum();
        self.cfg.pusd = pusd.to_checksum();
        self.cfg.loop_vault = loop_vault.to_checksum();
        self.cfg.v_converter = converter.to_checksum();
        self.cfg.pusd_migrator = migrator.to_checksum();
        self.cfg.rebase_policy = policy.to_checksum();
        self.cfg.gv = gv.to_checksum();
        self.cfg.house.clear();
        self.cfg.swap.clear();
        self.cfg.pair_config.clear();
        self.cfg.outbid.clear();
        self.cfg.ketlist.clear();
        self.cfg.save();
        Ok(hash)
    }

    fn token_total_supply(&self, token: Address) -> Result<u128, EconError> {
        let raw = self
            .rpc
            .eth_call(&self.key.address.to_hex(), Some(&token.to_hex()), &hex0x(&encode_fn("totalSupply()")))
            .map_err(econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("legacy V supply".into()))?;
        decode_word_u128(&bytes, 0).ok_or_else(|| EconError::Rpc("legacy V supply".into()))
    }

    fn contract_address(&self, contract: Address, sig: &str) -> Result<Address, EconError> {
        let raw = self
            .rpc
            .eth_call(&self.key.address.to_hex(), Some(&contract.to_hex()), &hex0x(&encode_fn(sig)))
            .map_err(econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("cutover address".into()))?;
        decode_word_addr(&bytes, 0).ok_or_else(|| EconError::Rpc("cutover address".into()))
    }
}

fn factory_create_data(
    legacy_market: Address,
    legacy_v: Address,
    legacy_supply: u128,
    bootstrap_v: u128,
    rate: u128,
) -> Result<Vec<u8>, EconError> {
    let mut out = hex::decode(FACTORY_HEX.trim().trim_start_matches("0x"))
        .map_err(|_| EconError::Rpc("canonical Lithe bytecode".into()))?;
    if out.len() >= MAX_INITCODE_BYTES {
        return Err(EconError::Rpc("canonical Lithe factory exceeds initcode limit".into()));
    }
    out.extend_from_slice(&abi_addr(legacy_market));
    out.extend_from_slice(&abi_addr(legacy_v));
    out.extend_from_slice(&abi_u256(legacy_supply));
    out.extend_from_slice(&abi_u256(bootstrap_v));
    out.extend_from_slice(&abi_u256(rate));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_bytecode_fits_eip_3860_initcode_limit() {
        let data = factory_create_data(Address([1; 20]), Address([2; 20]), DEC, BOOTSTRAP_V, DEC).unwrap();
        assert!(data.len() - 5 * 32 < MAX_INITCODE_BYTES);
        assert!(data.len() > 10_000);
    }

    #[test]
    fn factory_constructor_is_five_abi_words() {
        let market = Address([3; 20]);
        let vapurr = Address([4; 20]);
        let data = factory_create_data(market, vapurr, 7, 8, 9).unwrap();
        let args = &data[data.len() - 5 * 32..];
        assert_eq!(&args[0..32], &abi_addr(market));
        assert_eq!(&args[32..64], &abi_addr(vapurr));
        assert_eq!(&args[64..96], &abi_u256(7));
        assert_eq!(&args[96..128], &abi_u256(8));
        assert_eq!(&args[128..160], &abi_u256(9));
    }
}
