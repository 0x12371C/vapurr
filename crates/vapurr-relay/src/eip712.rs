//! EIP-712 signing for `VapurrForwarder.ForwardRequest` — must match
//! contracts/VapurrForwarder.sol's `DOMAIN_TYPEHASH`/`FORWARD_REQUEST_TYPEHASH`
//! and digest construction byte-for-byte, or a validly-signed request from
//! a user's wallet would recover to the wrong address here (or vice versa:
//! something this module accepts would fail on-chain). Change one, change
//! both, in the same breath — same discipline as vapurr-id's
//! Attestation::signing_message() / vapurrdb's attestationSigningMessage().

use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use sha3::{Digest, Keccak256};
use vapurr_wallet::tx::{abi_addr, abi_u256};
use vapurr_wallet::Address;

use crate::error::RelayError;

fn keccak(bytes: &[u8]) -> [u8; 32] {
    Keccak256::digest(bytes).into()
}

#[derive(Clone, Debug)]
pub struct Domain {
    pub chain_id: u64,
    pub verifying_contract: Address,
}

/// One signed, gas-sponsored intent. Mirrors the Solidity struct exactly:
/// `ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data,uint256 validUntil)`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ForwardRequest {
    #[serde(with = "addr_hex")]
    pub from: Address,
    #[serde(with = "addr_hex")]
    pub to: Address,
    #[serde(with = "str_u128")]
    pub value: u128,
    pub gas: u64,
    pub nonce: u64,
    #[serde(with = "hex_bytes")]
    pub data: Vec<u8>,
    pub valid_until: u64,
}

/// keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
const DOMAIN_TYPEHASH_PREIMAGE: &[u8] =
    b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";

/// keccak256("ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data,uint256 validUntil)")
const FORWARD_REQUEST_TYPEHASH_PREIMAGE: &[u8] =
    b"ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data,uint256 validUntil)";

pub fn domain_separator(domain: &Domain) -> [u8; 32] {
    let domain_typehash = keccak(DOMAIN_TYPEHASH_PREIMAGE);
    let name_hash = keccak(b"VapurrForwarder");
    let version_hash = keccak(b"1");

    let mut buf = Vec::with_capacity(32 * 5);
    buf.extend_from_slice(&domain_typehash);
    buf.extend_from_slice(&name_hash);
    buf.extend_from_slice(&version_hash);
    buf.extend_from_slice(&abi_u256(domain.chain_id as u128));
    buf.extend_from_slice(&abi_addr(domain.verifying_contract));
    keccak(&buf)
}

pub fn struct_hash(req: &ForwardRequest) -> [u8; 32] {
    let type_hash = keccak(FORWARD_REQUEST_TYPEHASH_PREIMAGE);
    let data_hash = keccak(&req.data);

    let mut buf = Vec::with_capacity(32 * 7);
    buf.extend_from_slice(&type_hash);
    buf.extend_from_slice(&abi_addr(req.from));
    buf.extend_from_slice(&abi_addr(req.to));
    buf.extend_from_slice(&abi_u256(req.value));
    buf.extend_from_slice(&abi_u256(req.gas as u128));
    buf.extend_from_slice(&abi_u256(req.nonce as u128));
    buf.extend_from_slice(&data_hash);
    buf.extend_from_slice(&abi_u256(req.valid_until as u128));
    keccak(&buf)
}

/// The exact digest a wallet signs (raw EIP-712 — "\x19\x01" prefix, NOT
/// the "\x19Ethereum Signed Message:\n" personal-sign prefix vapurr-id uses
/// for attestations). Different scheme on purpose: this is a typed-data
/// signature a wallet UI can render field-by-field, matching what
/// contracts/VapurrForwarder.sol's `digest()` computes on-chain.
pub fn digest(domain: &Domain, req: &ForwardRequest) -> [u8; 32] {
    let mut buf = Vec::with_capacity(2 + 32 + 32);
    buf.extend_from_slice(&[0x19, 0x01]);
    buf.extend_from_slice(&domain_separator(domain));
    buf.extend_from_slice(&struct_hash(req));
    keccak(&buf)
}

/// Recover the address that signed `req` for `domain`, from a 65-byte
/// (r || s || v) signature. Returns `RelayError::BadSignature` on a
/// malformed signature, and `RelayError::SignerMismatch` if it recovers to
/// something other than `req.from` — the relayer must never batch a
/// request whose signer isn't the address it claims to be acting for.
pub fn recover_and_verify(domain: &Domain, req: &ForwardRequest, sig: &[u8]) -> Result<(), RelayError> {
    if sig.len() != 65 {
        return Err(RelayError::BadSignature);
    }
    let signature = Signature::from_slice(&sig[..64]).map_err(|_| RelayError::BadSignature)?;
    let mut v = sig[64];
    if v >= 27 {
        v -= 27;
    }
    let rec = RecoveryId::from_byte(v).ok_or(RelayError::BadSignature)?;

    let h = digest(domain, req);
    let vk = VerifyingKey::recover_from_prehash(&h, &signature, rec).map_err(|_| RelayError::BadSignature)?;
    let pk = vk.to_encoded_point(false);
    let hash = keccak(&pk.as_bytes()[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);

    if Address(addr) != req.from {
        return Err(RelayError::SignerMismatch);
    }
    Ok(())
}

/// Convert a standard 65-byte (r, s, v) signature to EIP-2098 compact
/// form (64 bytes: r, then s with its top bit repurposed to hold the
/// recovery parity — safe because canonical low-s guarantees that bit is
/// always 0 otherwise). Matches `VapurrForwarder._recoverCompact` exactly.
///
/// Called only AFTER `recover_and_verify` already checked the original
/// 65-byte signature — this never changes what was verified, only how
/// it's encoded for the cheaper on-chain path. The user's wallet never
/// sees or produces this format; it signs an ordinary EIP-712 signature
/// and the relayer compacts it here before batching.
pub fn to_compact(sig: &[u8]) -> Result<[u8; 64], RelayError> {
    if sig.len() != 65 {
        return Err(RelayError::BadSignature);
    }
    let mut v = sig[64];
    if v >= 27 {
        v -= 27;
    }
    if v > 1 {
        return Err(RelayError::BadSignature);
    }
    // Canonical low-s (already enforced by the recovery path that ran
    // before this) means byte 32 (s's most significant byte) never has
    // its top bit set — that's the bit this reclaims for parity.
    if sig[32] & 0x80 != 0 {
        return Err(RelayError::BadSignature);
    }
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&sig[..32]); // r
    out[32..].copy_from_slice(&sig[32..64]); // s
    if v == 1 {
        out[32] |= 0x80;
    }
    Ok(out)
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("0x{}", hex::encode(bytes)))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        let s = s.trim_start_matches("0x");
        hex::decode(s).map_err(serde::de::Error::custom)
    }
}

/// `0x`-hex over the wire — a JSON byte array is technically what Address's
/// own derive would produce, but nobody's client should have to build that
/// by hand.
mod addr_hex {
    use serde::{Deserialize, Deserializer, Serializer};
    use vapurr_wallet::Address;

    pub fn serialize<S: Serializer>(a: &Address, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("0x{}", hex::encode(a.0)))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Address, D::Error> {
        let s = String::deserialize(d)?;
        let bytes = hex::decode(s.trim_start_matches("0x")).map_err(serde::de::Error::custom)?;
        if bytes.len() != 20 {
            return Err(serde::de::Error::custom("address must be 20 bytes"));
        }
        let mut a = [0u8; 20];
        a.copy_from_slice(&bytes);
        Ok(Address(a))
    }
}

/// Decimal string over the wire — a JSON number silently loses precision
/// above 2^53, and a token value in base units routinely exceeds that.
mod str_u128 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(n: &u128, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&n.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u128, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<u128>().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;

    fn addr_from_hex(s: &str) -> Address {
        let s = s.trim_start_matches("0x");
        let bytes = hex::decode(s).unwrap();
        let mut a = [0u8; 20];
        a.copy_from_slice(&bytes);
        Address(a)
    }

    fn bytes_from_hex(s: &str) -> [u8; 32] {
        let bytes = hex::decode(s.trim_start_matches("0x")).unwrap();
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }

    /// Golden vectors generated with ethers.js's TypedDataEncoder against
    /// this exact domain/type — see the PR description for the script.
    /// If this test ever fails after touching this file, the fix is almost
    /// certainly here, not in the vector.
    #[test]
    fn matches_ethers_js_golden_vector() {
        let domain = Domain {
            chain_id: 4663,
            verifying_contract: addr_from_hex("0x5555555555555555555555555555555555555555"),
        };
        let req = ForwardRequest {
            from: addr_from_hex("0x550A84e80AC1E7557AAEa3Ad78F0F335b99B66aB"),
            to: addr_from_hex("0x6666666666666666666666666666666666666666"),
            value: 0,
            gas: 100_000,
            nonce: 3,
            data: hex::decode("deadbeef").unwrap(),
            valid_until: 1_893_456_000,
        };

        let expected_domain_sep = bytes_from_hex("0xc9d67468640f26bea99ccf52b400ef49f8429e6d5e0878448ee89ff84732b2d0");
        let expected_struct_hash = bytes_from_hex("0x242578c53e5083d8956f402bb0db57936a21a394b5e1154cf093540a4742d090");
        let expected_digest = bytes_from_hex("0x314125d151e8c63c74e00e53e9678b077c80a3a0c0f0e322a15199a4b37ec484");

        assert_eq!(domain_separator(&domain), expected_domain_sep, "domain separator mismatch");
        assert_eq!(struct_hash(&req), expected_struct_hash, "struct hash mismatch");
        assert_eq!(digest(&domain, &req), expected_digest, "digest mismatch");

        let sig = hex::decode(
            "17f8bb7893395a948c32db97df0bd680a565dc9ea07764ff73791442e53924614649f84d9eb75e521ebef777212d4fc4b751d2c75a897f9ed41d504e3acc18e01c"
        ).unwrap();
        recover_and_verify(&domain, &req, &sig).expect("golden signature must recover to req.from");
    }

    /// Golden vectors from ethers.js's `Signature.compactSerialized`, one
    /// per parity bit — this is the conversion `VapurrForwarder._recoverCompact`
    /// has to agree with byte-for-byte, so both v=27 and v=28 get checked,
    /// not just whichever one a random test wallet happens to produce.
    #[test]
    fn to_compact_matches_ethers_js_golden_vectors() {
        let cases = [
            (
                "5dc008c003d8b71d0451251b422b25d0a02cf9208661bbe380a703b4e9ef77534c9ca8786bda36294e876b7a417983ae76d67a29bb054842630a33cf7e38e1a71b",
                "5dc008c003d8b71d0451251b422b25d0a02cf9208661bbe380a703b4e9ef77534c9ca8786bda36294e876b7a417983ae76d67a29bb054842630a33cf7e38e1a7",
            ),
            (
                "e5c26562bc983d9cdcb14ba771d6776a6e010d58daa5cf2cef2d3eaeda782f386b0a5829947be7c28f972ff7673054efeea029dd4e099abb1b955727cd658c641c",
                "e5c26562bc983d9cdcb14ba771d6776a6e010d58daa5cf2cef2d3eaeda782f38eb0a5829947be7c28f972ff7673054efeea029dd4e099abb1b955727cd658c64",
            ),
        ];
        for (full_hex, compact_hex) in cases {
            let full = hex::decode(full_hex).unwrap();
            let expected = hex::decode(compact_hex).unwrap();
            let got = to_compact(&full).expect("valid 65-byte sig must compact");
            assert_eq!(got.to_vec(), expected);
        }
    }

    #[test]
    fn sign_then_recover_round_trips() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32].into()).unwrap();
        let pk = signing_key.verifying_key().to_encoded_point(false);
        let hash = keccak(&pk.as_bytes()[1..]);
        let mut from_bytes = [0u8; 20];
        from_bytes.copy_from_slice(&hash[12..]);
        let from = Address(from_bytes);

        let domain = Domain { chain_id: 46630, verifying_contract: Address([0x11; 20]) };
        let req = ForwardRequest {
            from,
            to: Address([0x22; 20]),
            value: 0,
            gas: 21000,
            nonce: 0,
            data: vec![],
            valid_until: u64::MAX,
        };

        let h = digest(&domain, &req);
        let (sig, rec) = signing_key.sign_prehash_recoverable(&h).unwrap();
        let mut raw = [0u8; 65];
        raw[..64].copy_from_slice(&sig.to_bytes());
        raw[64] = rec.to_byte();

        recover_and_verify(&domain, &req, &raw).expect("self-signed request must verify");

        let mut tampered = req.clone();
        tampered.value = 1;
        assert!(matches!(
            recover_and_verify(&domain, &tampered, &raw),
            Err(RelayError::SignerMismatch)
        ));
    }
}
