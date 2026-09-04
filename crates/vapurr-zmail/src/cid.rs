//! CIDv1 raw (0x55) sha2-256, multibase base32. Same bytes Kubo emits for
//! `ipfs add --cid-version=1 --raw-leaves`.

use sha2::{Digest, Sha256};

const B32: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Empty raw block. Used as a fixture so we do not drift off Kubo.
pub const EMPTY_RAW: &str = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku";

pub fn cid_raw_sha256(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut buf = Vec::with_capacity(36);
    buf.extend_from_slice(&[0x01, 0x55, 0x12, 0x20]);
    buf.extend_from_slice(&hash);
    let mut out = String::with_capacity(1 + 58);
    out.push('b');
    out.push_str(&b32_nopad(&buf));
    out
}

fn b32_nopad(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() * 8).div_ceil(5));
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &b in data {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

pub fn looks_like_cid(s: &str) -> bool {
    let s = s.trim();
    s.starts_with('b')
        && s.len() >= 50
        && s.len() <= 80
        && s.bytes().all(|c| matches!(c, b'a'..=b'z' | b'2'..=b'7'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_matches_kubo() {
        assert_eq!(cid_raw_sha256(b""), EMPTY_RAW);
    }

    #[test]
    fn stable_for_same_bytes() {
        let a = cid_raw_sha256(b"purr");
        let b = cid_raw_sha256(b"purr");
        assert_eq!(a, b);
        assert!(a.starts_with("bafkrei"));
        assert!(looks_like_cid(&a));
        assert_ne!(a, cid_raw_sha256(b"purr!"));
    }
}
