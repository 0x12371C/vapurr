//! Solidity ABI encoding for `VapurrForwarder.executeBatch(...)`. Flat
//! parallel arrays rather than an array of structs — every parameter here
//! is independently dynamic (arrays always are), which keeps this to two
//! encoder shapes (array-of-fixed-width-word, array-of-bytes) instead of
//! the nested struct-array case, and those two are what get unit-tested
//! against a golden vector below.
//!
//! If you touch this file, the contract's function signature in
//! `SELECTOR_PREIMAGE` and the argument order both have to keep matching
//! `executeBatch`'s actual parameter list in VapurrForwarder.sol, or the
//! selector or the calldata layout silently stops matching what's on-chain.

use vapurr_wallet::tx::{abi_addr, abi_u256};
use vapurr_wallet::{keccak4, Address};

const SELECTOR_PREIMAGE: &str =
    "executeBatch(address[],address[],uint256[],uint256[],uint256[],bytes[],uint256[],bytes[])";

fn encode_address_array(items: &[Address]) -> Vec<u8> {
    let mut out = abi_u256(items.len() as u128).to_vec();
    for a in items {
        out.extend_from_slice(&abi_addr(*a));
    }
    out
}

fn encode_uint_array(items: &[u128]) -> Vec<u8> {
    let mut out = abi_u256(items.len() as u128).to_vec();
    for &n in items {
        out.extend_from_slice(&abi_u256(n));
    }
    out
}

/// One `bytes` value: length word, then the bytes right-padded to a
/// multiple of 32.
fn encode_bytes(b: &[u8]) -> Vec<u8> {
    let mut out = abi_u256(b.len() as u128).to_vec();
    out.extend_from_slice(b);
    let pad = (32 - (b.len() % 32)) % 32;
    out.extend(std::iter::repeat(0u8).take(pad));
    out
}

/// `bytes[]`: length word, then one offset word per element (relative to
/// right after this array's own length word), then each element's own
/// `encode_bytes` in order.
fn encode_bytes_array(items: &[Vec<u8>]) -> Vec<u8> {
    let n = items.len();
    let mut head = abi_u256(n as u128).to_vec();
    let mut tail = Vec::new();
    let mut running_offset = 32 * n;
    for item in items {
        head.extend_from_slice(&abi_u256(running_offset as u128));
        let enc = encode_bytes(item);
        running_offset += enc.len();
        tail.extend_from_slice(&enc);
    }
    head.extend_from_slice(&tail);
    head
}

#[derive(Clone, Debug)]
pub struct BatchItem {
    pub from: Address,
    pub to: Address,
    pub value: u128,
    pub gas: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub valid_until: u64,
    /// 65-byte r||s||v.
    pub sig: Vec<u8>,
}

/// Build the full calldata (selector + args) for `executeBatch`. All eight
/// parameters are dynamic, so the head is eight offset words and every
/// parameter's encoding follows in the tail, in order.
pub fn encode_execute_batch(items: &[BatchItem]) -> Vec<u8> {
    let froms: Vec<Address> = items.iter().map(|i| i.from).collect();
    let tos: Vec<Address> = items.iter().map(|i| i.to).collect();
    let values: Vec<u128> = items.iter().map(|i| i.value).collect();
    let gases: Vec<u128> = items.iter().map(|i| i.gas as u128).collect();
    let nonces: Vec<u128> = items.iter().map(|i| i.nonce as u128).collect();
    let datas: Vec<Vec<u8>> = items.iter().map(|i| i.data.clone()).collect();
    let valid_untils: Vec<u128> = items.iter().map(|i| i.valid_until as u128).collect();
    let sigs: Vec<Vec<u8>> = items.iter().map(|i| i.sig.clone()).collect();

    let params: [Vec<u8>; 8] = [
        encode_address_array(&froms),
        encode_address_array(&tos),
        encode_uint_array(&values),
        encode_uint_array(&gases),
        encode_uint_array(&nonces),
        encode_bytes_array(&datas),
        encode_uint_array(&valid_untils),
        encode_bytes_array(&sigs),
    ];

    let n = params.len();
    let mut head = Vec::with_capacity(32 * n);
    let mut tail = Vec::new();
    let mut running_offset = 32 * n;
    for p in &params {
        head.extend_from_slice(&abi_u256(running_offset as u128));
        running_offset += p.len();
    }
    for p in &params {
        tail.extend_from_slice(p);
    }

    let mut out = keccak4(SELECTOR_PREIMAGE).to_vec();
    out.extend_from_slice(&head);
    out.extend_from_slice(&tail);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> Address {
        let bytes = hex::decode(s.trim_start_matches("0x")).unwrap();
        let mut a = [0u8; 20];
        a.copy_from_slice(&bytes);
        Address(a)
    }

    /// Golden vector generated with ethers.js's Interface.encodeFunctionData
    /// against this exact function signature and these exact inputs — see
    /// the PR description for the script. A mismatch here means the
    /// encoder, not the vector, is wrong.
    #[test]
    fn matches_ethers_js_golden_vector() {
        let items = vec![
            BatchItem {
                from: addr("0x1111111111111111111111111111111111111111"),
                to: addr("0x3333333333333333333333333333333333333333"),
                value: 0,
                gas: 100_000,
                nonce: 0,
                data: hex::decode("deadbeef").unwrap(),
                valid_until: 1_893_456_000,
                sig: vec![0x11; 65],
            },
            BatchItem {
                from: addr("0x2222222222222222222222222222222222222222"),
                to: addr("0x4444444444444444444444444444444444444444"),
                value: 7,
                gas: 50_000,
                nonce: 5,
                data: vec![],
                valid_until: 1_893_456_000,
                sig: vec![0x22; 65],
            },
        ];

        let got = encode_execute_batch(&items);
        let got_hex = format!("0x{}", hex::encode(&got));

        let expected = "0xeef984c50000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001c00000000000000000000000000000000000000000000000000000000000000220000000000000000000000000000000000000000000000000000000000000028000000000000000000000000000000000000000000000000000000000000002e000000000000000000000000000000000000000000000000000000000000003a00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000200000000000000000000000011111111111111111111111111111111111111110000000000000000000000002222222222222222222222222222222222222222000000000000000000000000000000000000000000000000000000000000000200000000000000000000000033333333333333333333333333333333333333330000000000000000000000004444444444444444444444444444444444444444000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000186a0000000000000000000000000000000000000000000000000000000000000c3500000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000004deadbeef00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000070dbd8800000000000000000000000000000000000000000000000000000000070dbd8800000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000411111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000041222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222200000000000000000000000000000000000000000000000000000000000000";

        assert_eq!(got_hex, expected);
    }

    #[test]
    fn empty_batch_encodes_zero_lengths() {
        let out = encode_execute_batch(&[]);
        // selector (4) + 8 offset words + 8 length-zero words (one per array)
        assert_eq!(out.len(), 4 + 32 * 8 + 32 * 8);
    }
}
