//! ECDSA (secp256k1) signing/verification for zer0ID attestations.
//!
//! No new crypto stack: this is the same EIP-191 "personal_sign" +
//! Keccak256 pair vapurr-wallet already uses to sign transactions
//! (`k256::ecdsa` + `sha3::Keccak256`), applied to attestations instead of
//! txs. A vapurrDB deployment verifies the identical signature the same
//! way via `ethers.verifyMessage` — see vapurrdb's crypto/signatures.ts.

use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use sha3::{Digest, Keccak256};

use crate::IdError;

/// EIP-191 "personal_sign" digest: keccak256("\x19Ethereum Signed Message:\n" + len + message).
fn personal_sign_hash(message: &str) -> [u8; 32] {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut bytes = Vec::with_capacity(prefix.len() + message.len());
    bytes.extend_from_slice(prefix.as_bytes());
    bytes.extend_from_slice(message.as_bytes());
    Keccak256::digest(&bytes).into()
}

fn address_from_verifying_key(vk: &VerifyingKey) -> String {
    let pk = vk.to_encoded_point(false);
    let hash = Keccak256::digest(&pk.as_bytes()[1..]);
    format!("0x{}", hex::encode(&hash[12..]))
}

/// The address a `SigningKey` recovers to — for a provider to advertise its own issuer address.
pub fn address_of(key: &SigningKey) -> String {
    address_from_verifying_key(key.verifying_key())
}

/// Sign `message` with `key`, returning a 65-byte r||s||v signature as `0x`-hex.
pub fn sign_message(message: &str, key: &SigningKey) -> String {
    let hash = personal_sign_hash(message);
    let (sig, rec) = key
        .sign_prehash_recoverable(&hash)
        .expect("signing a 32-byte digest cannot fail");
    let b = sig.to_bytes();
    let mut out = [0u8; 65];
    out[..32].copy_from_slice(&b[..32]);
    out[32..64].copy_from_slice(&b[32..]);
    out[64] = rec.to_byte();
    format!("0x{}", hex::encode(out))
}

/// Recover the signer address from a `0x`-hex 65-byte (r||s||v) signature over `message`.
pub fn recover_signer(message: &str, sig_hex: &str) -> Result<String, IdError> {
    let hex_str = sig_hex.trim().trim_start_matches("0x");
    let bytes = hex::decode(hex_str).map_err(|_| IdError::BadSignature)?;
    if bytes.len() != 65 {
        return Err(IdError::BadSignature);
    }
    let sig = Signature::from_slice(&bytes[..64]).map_err(|_| IdError::BadSignature)?;
    let rec = RecoveryId::from_byte(bytes[64]).ok_or(IdError::BadSignature)?;
    let hash = personal_sign_hash(message);
    let vk = VerifyingKey::recover_from_prehash(&hash, &sig, rec)
        .map_err(|_| IdError::BadSignature)?;
    Ok(address_from_verifying_key(&vk))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn sign_then_recover_matches() {
        let key = SigningKey::random(&mut OsRng);
        let addr = address_of(&key);
        let sig = sign_message("hello zer0ID", &key);
        let recovered = recover_signer("hello zer0ID", &sig).unwrap();
        assert_eq!(recovered, addr);
    }

    #[test]
    fn tampered_message_fails_to_match() {
        let key = SigningKey::random(&mut OsRng);
        let addr = address_of(&key);
        let sig = sign_message("hello zer0ID", &key);
        let recovered = recover_signer("goodbye zer0ID", &sig).unwrap();
        assert_ne!(recovered, addr);
    }

    #[test]
    fn malformed_signature_is_rejected() {
        assert!(matches!(
            recover_signer("hello", "0xnothex"),
            Err(IdError::BadSignature)
        ));
        assert!(matches!(
            recover_signer("hello", "0xdead"),
            Err(IdError::BadSignature)
        ));
    }
}
