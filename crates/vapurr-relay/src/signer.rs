//! The relayer's own signing identity — separate from any user's wallet.
//! This key only ever signs the relayer's OWN transactions that call
//! `VapurrForwarder.executeBatch`; it never touches a user's funds or
//! session keys. It must be an address in `authorizedRelayers` on the
//! forwarder contract, added there by the treasury owner/multisig — see
//! contracts/VapurrForwarder.sol.

use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};
use vapurr_wallet::Address;

pub fn address_of(key: &SigningKey) -> Address {
    let pk = key.verifying_key().to_encoded_point(false);
    let hash = Keccak256::digest(&pk.as_bytes()[1..]);
    let mut out = [0u8; 20];
    out.copy_from_slice(&hash[12..]);
    Address(out)
}

pub fn key_from_hex(hex_str: &str) -> Result<SigningKey, String> {
    let bytes = hex::decode(hex_str.trim().trim_start_matches("0x")).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("relayer private key must be 32 bytes".into());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    SigningKey::from_bytes((&arr).into()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_of_is_deterministic() {
        let key = SigningKey::from_bytes(&[9u8; 32].into()).unwrap();
        let a1 = address_of(&key);
        let a2 = address_of(&key);
        assert_eq!(a1, a2);
    }
}
