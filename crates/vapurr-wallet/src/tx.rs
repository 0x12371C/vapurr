//! EIP-1559 typed txs (0x02) for Robinhood Chain.

use k256::ecdsa::{RecoveryId, Signature, SigningKey};
use sha3::{Digest, Keccak256};

use crate::{Address, DeviceKey, WalletError};

#[derive(Clone, Debug)]
pub struct Tx {
    pub chain_id: u64,
    pub nonce: u64,
    pub max_priority_fee: u128,
    pub max_fee: u128,
    pub gas: u64,
    pub to: Option<Address>,
    pub value: u128,
    pub data: Vec<u8>,
}

impl DeviceKey {
    pub fn sign_tx(&self, tx: &Tx) -> Result<Vec<u8>, WalletError> {
        sign_tx(&self.signing, tx)
    }
}

pub fn sign_tx(key: &SigningKey, tx: &Tx) -> Result<Vec<u8>, WalletError> {
    let unsigned = rlp_list(&[
        rlp_u64(tx.chain_id),
        rlp_u64(tx.nonce),
        rlp_u128(tx.max_priority_fee),
        rlp_u128(tx.max_fee),
        rlp_u64(tx.gas),
        rlp_address(tx.to),
        rlp_u128(tx.value),
        rlp_bytes(&tx.data),
        rlp_list(&[]),
    ]);
    let mut pre = Vec::with_capacity(1 + unsigned.len());
    pre.push(0x02);
    pre.extend_from_slice(&unsigned);
    let hash = Keccak256::digest(&pre);
    let (sig, rec) = key
        .sign_prehash_recoverable(hash.as_slice())
        .map_err(|_| WalletError::Sign)?;
    let (r, s, y) = split_sig(&sig, rec);
    let signed = rlp_list(&[
        rlp_u64(tx.chain_id),
        rlp_u64(tx.nonce),
        rlp_u128(tx.max_priority_fee),
        rlp_u128(tx.max_fee),
        rlp_u64(tx.gas),
        rlp_address(tx.to),
        rlp_u128(tx.value),
        rlp_bytes(&tx.data),
        rlp_list(&[]),
        rlp_u64(y as u64),
        rlp_bytes(trim_left(&r)),
        rlp_bytes(trim_left(&s)),
    ]);
    let mut out = Vec::with_capacity(1 + signed.len());
    out.push(0x02);
    out.extend_from_slice(&signed);
    Ok(out)
}

fn split_sig(sig: &Signature, rec: RecoveryId) -> ([u8; 32], [u8; 32], u8) {
    let b = sig.to_bytes();
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&b[..32]);
    s.copy_from_slice(&b[32..]);
    (r, s, rec.to_byte())
}

fn trim_left(b: &[u8; 32]) -> &[u8] {
    let i = b.iter().position(|&x| x != 0).unwrap_or(b.len());
    if i == b.len() {
        &[]
    } else {
        &b[i..]
    }
}

pub fn rlp_bytes(b: &[u8]) -> Vec<u8> {
    if b.len() == 1 && b[0] < 0x80 {
        return vec![b[0]];
    }
    rlp_header(0x80, b.len(), b)
}

pub fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut body = Vec::new();
    for it in items {
        body.extend_from_slice(it);
    }
    rlp_header(0xc0, body.len(), &body)
}

fn rlp_header(offset: usize, len: usize, payload: &[u8]) -> Vec<u8> {
    if len <= 55 {
        let mut o = Vec::with_capacity(1 + payload.len());
        o.push((offset + len) as u8);
        o.extend_from_slice(payload);
        o
    } else {
        let lb = len_bytes(len);
        let mut o = Vec::with_capacity(1 + lb.len() + payload.len());
        o.push((offset + 55 + lb.len()) as u8);
        o.extend_from_slice(&lb);
        o.extend_from_slice(payload);
        o
    }
}

fn len_bytes(mut n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    while n > 0 {
        v.push((n & 0xff) as u8);
        n >>= 8;
    }
    v.reverse();
    v
}

pub fn rlp_u64(n: u64) -> Vec<u8> {
    if n == 0 {
        return vec![0x80];
    }
    let b = n.to_be_bytes();
    let i = b.iter().position(|&x| x != 0).unwrap();
    rlp_bytes(&b[i..])
}

pub fn rlp_u128(n: u128) -> Vec<u8> {
    if n == 0 {
        return vec![0x80];
    }
    let b = n.to_be_bytes();
    let i = b.iter().position(|&x| x != 0).unwrap();
    rlp_bytes(&b[i..])
}

fn rlp_address(to: Option<Address>) -> Vec<u8> {
    match to {
        None => vec![0x80],
        Some(a) => rlp_bytes(&a.0),
    }
}

pub fn abi_u256(n: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[16..].copy_from_slice(&n.to_be_bytes());
    out
}

pub fn abi_addr(a: Address) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&a.0);
    out
}

pub fn encode_fn_u256(sig: &str, n: u128) -> Vec<u8> {
    let mut d = Vec::with_capacity(36);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_u256(n));
    d
}

pub fn encode_fn_bytes32(sig: &str, key: &[u8; 32]) -> Vec<u8> {
    let mut d = Vec::with_capacity(36);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(key);
    d
}

pub fn encode_fn_bytes32_addr(sig: &str, node: &[u8; 32], a: Address) -> Vec<u8> {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(node);
    d.extend_from_slice(&abi_addr(a));
    d
}

pub fn encode_fn_addr(sig: &str, a: Address) -> Vec<u8> {
    let mut d = Vec::with_capacity(36);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_addr(a));
    d
}

pub fn encode_fn(sig: &str) -> Vec<u8> {
    crate::keccak4(sig).to_vec()
}

pub fn encode_fn_addr_u256(sig: &str, a: Address, n: u128) -> Vec<u8> {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_addr(a));
    d.extend_from_slice(&abi_u256(n));
    d
}

pub fn encode_fn_two_u256(sig: &str, a: u128, b: u128) -> Vec<u8> {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_u256(a));
    d.extend_from_slice(&abi_u256(b));
    d
}

pub fn encode_fn_four_u256(sig: &str, a: u128, b: u128, c: u128, d: u128) -> Vec<u8> {
    let mut out = Vec::with_capacity(132);
    out.extend_from_slice(&crate::keccak4(sig));
    out.extend_from_slice(&abi_u256(a));
    out.extend_from_slice(&abi_u256(b));
    out.extend_from_slice(&abi_u256(c));
    out.extend_from_slice(&abi_u256(d));
    out
}

pub fn encode_fn_addr_addr(sig: &str, a: Address, b: Address) -> Vec<u8> {
    let mut d = Vec::with_capacity(68);
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_addr(a));
    d.extend_from_slice(&abi_addr(b));
    d
}

fn abi_string(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(32 + ((b.len() + 31) / 32) * 32);
    out.extend_from_slice(&abi_u256(b.len() as u128));
    out.extend_from_slice(b);
    while out.len() % 32 != 0 {
        out.push(0);
    }
    out
}

/// `fn(string)` — head is offset, then the string.
pub fn encode_fn_str(sig: &str, s: &str) -> Vec<u8> {
    let tail = abi_string(s);
    let mut d = Vec::with_capacity(4 + 32 + tail.len());
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_u256(32));
    d.extend_from_slice(&tail);
    d
}

/// `fn(string,bytes32)` — head is offset + word, then the string.
pub fn encode_fn_str_bytes32(sig: &str, s: &str, word: [u8; 32]) -> Vec<u8> {
    let tail = abi_string(s);
    let mut d = Vec::with_capacity(4 + 64 + tail.len());
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_u256(64));
    d.extend_from_slice(&word);
    d.extend_from_slice(&tail);
    d
}

/// ABI `string` at the head of an eth_call result (offset pointer, then bytes).
pub fn decode_abi_string(data: &[u8]) -> Option<String> {
    if data.len() < 64 {
        return None;
    }
    let off = decode_word_u128(data, 0)? as usize;
    if off.saturating_add(32) > data.len() {
        return None;
    }
    let n = {
        let w = &data[off..off + 32];
        if w[..16].iter().any(|&x| x != 0) {
            return None;
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(&w[16..]);
        u128::from_be_bytes(b) as usize
    };
    let start = off.saturating_add(32);
    if data.len() < start.saturating_add(n) {
        return None;
    }
    Some(String::from_utf8_lossy(&data[start..start + n]).into_owned())
}

/// `fn(string,string,uint256)` — head is offset/offset/value, then the two strings.
pub fn encode_fn_two_str_u256(sig: &str, a: &str, b: &str, n: u128) -> Vec<u8> {
    let a_enc = abi_string(a);
    let b_enc = abi_string(b);
    let mut d = Vec::with_capacity(4 + 96 + a_enc.len() + b_enc.len());
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_u256(96));
    d.extend_from_slice(&abi_u256(96 + a_enc.len() as u128));
    d.extend_from_slice(&abi_u256(n));
    d.extend_from_slice(&a_enc);
    d.extend_from_slice(&b_enc);
    d
}

/// `fn(address,address,string,string,uint256)` — head is addr/addr/offset/offset/value.
pub fn encode_fn_two_addr_two_str_u256(
    sig: &str,
    a: Address,
    b: Address,
    s: &str,
    t: &str,
    n: u128,
) -> Vec<u8> {
    let s_enc = abi_string(s);
    let t_enc = abi_string(t);
    let mut d = Vec::with_capacity(4 + 160 + s_enc.len() + t_enc.len());
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_addr(a));
    d.extend_from_slice(&abi_addr(b));
    d.extend_from_slice(&abi_u256(160));
    d.extend_from_slice(&abi_u256(160 + s_enc.len() as u128));
    d.extend_from_slice(&abi_u256(n));
    d.extend_from_slice(&s_enc);
    d.extend_from_slice(&t_enc);
    d
}

/// `fn(address,address,string,string,string,uint256)` — head is addr/addr/off/off/off/value.
pub fn encode_fn_two_addr_three_str_u256(
    sig: &str,
    a: Address,
    b: Address,
    s: &str,
    t: &str,
    u: &str,
    n: u128,
) -> Vec<u8> {
    let s_enc = abi_string(s);
    let t_enc = abi_string(t);
    let u_enc = abi_string(u);
    let mut d = Vec::with_capacity(4 + 192 + s_enc.len() + t_enc.len() + u_enc.len());
    d.extend_from_slice(&crate::keccak4(sig));
    d.extend_from_slice(&abi_addr(a));
    d.extend_from_slice(&abi_addr(b));
    d.extend_from_slice(&abi_u256(192));
    d.extend_from_slice(&abi_u256(192 + s_enc.len() as u128));
    d.extend_from_slice(&abi_u256(192 + s_enc.len() as u128 + t_enc.len() as u128));
    d.extend_from_slice(&abi_u256(n));
    d.extend_from_slice(&s_enc);
    d.extend_from_slice(&t_enc);
    d.extend_from_slice(&u_enc);
    d
}

pub fn decode_dyn_string(data: &[u8], offset: usize) -> Option<String> {
    if data.len() < offset.saturating_add(32) {
        return None;
    }
    let n = decode_word_u128(data, offset / 32)? as usize;
    let start = offset.saturating_add(32);
    if data.len() < start.saturating_add(n) {
        return None;
    }
    Some(String::from_utf8_lossy(&data[start..start + n]).into_owned())
}

pub fn hex0x(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

pub fn decode_word_u128(data: &[u8], i: usize) -> Option<u128> {
    let s = i.checked_mul(32)?;
    if data.len() < s + 32 {
        return None;
    }
    let w = &data[s..s + 32];
    if w[..16].iter().any(|&x| x != 0) {
        return Some(u128::MAX);
    }
    let mut b = [0u8; 16];
    b.copy_from_slice(&w[16..]);
    Some(u128::from_be_bytes(b))
}

pub fn decode_word_addr(data: &[u8], i: usize) -> Option<Address> {
    let s = i.checked_mul(32)?;
    if data.len() < s + 32 {
        return None;
    }
    let mut a = [0u8; 20];
    a.copy_from_slice(&data[s + 12..s + 32]);
    Some(Address(a))
}

pub fn decode_hex_bytes(s: &str) -> Result<Vec<u8>, WalletError> {
    let s = s.trim().trim_start_matches("0x");
    if s.is_empty() {
        return Ok(Vec::new());
    }
    hex::decode(s).map_err(|_| WalletError::Rpc)
}

/// Solidity `Error(string)` payload from a revert.
pub fn revert_reason(data: &str) -> Option<String> {
    let bytes = decode_hex_bytes(data).ok()?;
    if bytes.len() < 4 + 64 {
        return None;
    }
    if bytes[0..4] != [0x08, 0xc3, 0x79, 0xa0] {
        return None;
    }
    let mut lenb = [0u8; 8];
    lenb.copy_from_slice(&bytes[4 + 24..4 + 32]);
    // offset is usually 32; length is the next word
    if bytes.len() < 4 + 32 + 32 {
        return None;
    }
    let mut nbuf = [0u8; 8];
    nbuf.copy_from_slice(&bytes[4 + 32 + 24..4 + 32 + 32]);
    let n = u64::from_be_bytes(nbuf) as usize;
    let start = 4 + 32 + 32;
    if bytes.len() < start + n {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes[start..start + n]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::VerifyingKey;

    #[test]
    fn two_str_u256_head() {
        let d = encode_fn_two_str_u256("bid(string,string,uint256)", "@vapurr", "cat", 10);
        assert_eq!(&d[..4], &crate::keccak4("bid(string,string,uint256)"));
        assert_eq!(&d[4..36], &abi_u256(96));
        let a_len = 32 + 32; // " @vapurr" padded: len word + 7 bytes → 32
                             // "@vapurr" is 7 bytes → length word + 32 pad = 64
        assert_eq!(&d[36..68], &abi_u256(96 + 64));
        assert_eq!(&d[68..100], &abi_u256(10));
        assert_eq!(decode_dyn_string(&d[4..], 96).as_deref(), Some("@vapurr"));
        assert_eq!(decode_dyn_string(&d[4..], 96 + 64).as_deref(), Some("cat"));
        let _ = a_len;
    }

    #[test]
    fn two_addr_two_str_u256_head() {
        let token = Address([0x11u8; 20]);
        let pool = Address([0x22u8; 20]);
        let d = encode_fn_two_addr_two_str_u256(
            "list(address,address,string,string,uint256)",
            token,
            pool,
            "FOO",
            "Foo Token",
            50,
        );
        assert_eq!(
            &d[..4],
            &crate::keccak4("list(address,address,string,string,uint256)")
        );
        assert_eq!(&d[4..36], &abi_addr(token));
        assert_eq!(&d[36..68], &abi_addr(pool));
        assert_eq!(&d[68..100], &abi_u256(160));
        let s_len = 32 + 32; // "FOO" → length word + 32 pad
        assert_eq!(&d[100..132], &abi_u256(160 + s_len as u128));
        assert_eq!(&d[132..164], &abi_u256(50));
        assert_eq!(decode_dyn_string(&d[4..], 160).as_deref(), Some("FOO"));
        assert_eq!(
            decode_dyn_string(&d[4..], 160 + s_len).as_deref(),
            Some("Foo Token")
        );
    }

    #[test]
    fn two_addr_three_str_u256_head() {
        let token = Address([0x11u8; 20]);
        let pool = Address([0x22u8; 20]);
        let d = encode_fn_two_addr_three_str_u256(
            "list(address,address,string,string,string,uint256)",
            token,
            pool,
            "FOO",
            "Foo Token",
            "{\"w\":\"https://foo.hood\"}",
            50,
        );
        assert_eq!(
            &d[..4],
            &crate::keccak4("list(address,address,string,string,string,uint256)")
        );
        assert_eq!(&d[4..36], &abi_addr(token));
        assert_eq!(&d[68..100], &abi_u256(192));
        assert_eq!(&d[164..196], &abi_u256(50));
        assert_eq!(decode_dyn_string(&d[4..], 192).as_deref(), Some("FOO"));
    }

    #[test]
    fn rlp_scalars() {
        assert_eq!(rlp_u64(0), vec![0x80]);
        assert_eq!(rlp_u64(1), vec![0x01]);
        assert_eq!(rlp_u64(127), vec![0x7f]);
        assert_eq!(rlp_u64(128), vec![0x81, 0x80]);
        assert_eq!(rlp_list(&[]), vec![0xc0]);
    }

    #[test]
    fn signed_tx_recovers_sender() {
        let key = DeviceKey::generate();
        let tx = Tx {
            chain_id: 4663,
            nonce: 0,
            max_priority_fee: 1_000_000,
            max_fee: 1_000_000_000,
            gas: 21_000,
            to: Some(Address([0x11; 20])),
            value: 0,
            data: vec![],
        };
        let raw = key.sign_tx(&tx).unwrap();
        assert_eq!(raw[0], 0x02);
        let unsigned = rlp_list(&[
            rlp_u64(tx.chain_id),
            rlp_u64(tx.nonce),
            rlp_u128(tx.max_priority_fee),
            rlp_u128(tx.max_fee),
            rlp_u64(tx.gas),
            rlp_address(tx.to),
            rlp_u128(tx.value),
            rlp_bytes(&tx.data),
            rlp_list(&[]),
        ]);
        let mut pre = vec![0x02];
        pre.extend_from_slice(&unsigned);
        let hash = Keccak256::digest(&pre);
        // last 3 rlp items of the signed list are y, r, s — recover via sign roundtrip
        let (sig, rec) = key
            .signing
            .sign_prehash_recoverable(hash.as_slice())
            .unwrap();
        let vk = VerifyingKey::recover_from_prehash(hash.as_slice(), &sig, rec).unwrap();
        let pk = vk.to_encoded_point(false);
        let h = Keccak256::digest(&pk.as_bytes()[1..]);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&h[12..]);
        assert_eq!(addr, key.address.0);
    }

    #[test]
    fn mint_selector() {
        assert_eq!(crate::keccak4("mintPusd(uint256)").len(), 4);
        assert_eq!(crate::keccak4("snapshot(address)").len(), 4);
        assert_ne!(
            crate::keccak4("mintPusd(uint256)"),
            crate::keccak4("snapshot(address)")
        );
    }
}
