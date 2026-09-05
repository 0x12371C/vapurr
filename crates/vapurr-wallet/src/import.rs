//! Load a secp256k1 key from hex or a BIP-39 seed. ETH path m/44'/60'/0'/0/0.

use hmac::{Hmac, Mac};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::elliptic_curve::PrimeField;
use k256::{Scalar, SecretKey};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha512;
use zeroize::Zeroize;

use crate::{DeviceKey, WalletError};

type HmacSha512 = Hmac<Sha512>;

pub fn import_text(raw: &str) -> Result<DeviceKey, WalletError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(WalletError::Fail("paste a key or seed".into()));
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    let sk = if words.len() == 12 || words.len() == 24 {
        seed_to_eth_sk(&mnemonic_seed(&words.join(" "))?)?
    } else {
        hex_sk(s)?
    };
    let key = DeviceKey::from_secret(&sk).ok_or(WalletError::Fail("bad key".into()))?;
    Ok(key)
}

/// New 12-word wallet. ETH path m/44'/60'/0'/0/0. Does not write disk.
pub(crate) fn generate_phrase() -> Result<(DeviceKey, String), WalletError> {
    let mut entropy = [0u8; 16];
    OsRng.fill_bytes(&mut entropy);
    let m = bip39::Mnemonic::from_entropy(&entropy)
        .map_err(|_| WalletError::Fail("could not create seed".into()))?;
    let phrase = m.to_string();
    entropy.zeroize();
    let seed = mnemonic_seed(&phrase)?;
    let sk = seed_to_eth_sk(&seed)?;
    let key = DeviceKey::from_secret(&sk).ok_or(WalletError::Fail("bad key".into()))?;
    Ok((key, phrase))
}

fn hex_sk(s: &str) -> Result<[u8; 32], WalletError> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    let bytes = hex::decode(s).map_err(|_| WalletError::Fail("bad key".into()))?;
    if bytes.len() != 32 {
        return Err(WalletError::Fail("key must be 32 bytes".into()));
    }
    let mut sk = [0u8; 32];
    sk.copy_from_slice(&bytes);
    Ok(sk)
}

fn mnemonic_seed(phrase: &str) -> Result<[u8; 64], WalletError> {
    let m = bip39::Mnemonic::parse_normalized(phrase)
        .map_err(|_| WalletError::Fail("bad seed phrase".into()))?;
    Ok(m.to_seed(""))
}

fn seed_to_eth_sk(seed: &[u8; 64]) -> Result<[u8; 32], WalletError> {
    let (mut k, mut c) = master(seed)?;
    // m/44'/60'/0'/0/0
    for (i, hard) in [(44u32, true), (60, true), (0, true), (0, false), (0, false)] {
        let next = derive(&k, &c, i, hard)?;
        k.zeroize();
        c.zeroize();
        k = next.0;
        c = next.1;
    }
    c.zeroize();
    Ok(k)
}

fn master(seed: &[u8]) -> Result<([u8; 32], [u8; 32]), WalletError> {
    split(&hmac512(b"Bitcoin seed", seed))
}

fn derive(
    k: &[u8; 32],
    chain: &[u8; 32],
    i: u32,
    hard: bool,
) -> Result<([u8; 32], [u8; 32]), WalletError> {
    let idx = if hard { i | 0x8000_0000 } else { i };
    let mut data = Vec::with_capacity(37);
    if hard {
        data.push(0);
        data.extend_from_slice(k);
    } else {
        data.extend_from_slice(&compressed(k)?);
    }
    data.extend_from_slice(&idx.to_be_bytes());
    let h = hmac512(chain, &data);
    data.zeroize();
    let (il, ir) = split(&h)?;
    let kid = add_il(k, &il).ok_or(WalletError::Fail("bad seed".into()))?;
    Ok((kid, ir))
}

fn compressed(k: &[u8; 32]) -> Result<[u8; 33], WalletError> {
    let sk = SecretKey::from_slice(k).map_err(|_| WalletError::Fail("bad key".into()))?;
    let pt = sk.public_key().to_encoded_point(true);
    let b = pt.as_bytes();
    if b.len() != 33 {
        return Err(WalletError::Fail("bad key".into()));
    }
    let mut out = [0u8; 33];
    out.copy_from_slice(b);
    Ok(out)
}

fn add_il(k: &[u8; 32], il: &[u8; 32]) -> Option<[u8; 32]> {
    let a = Scalar::from_repr((*k).into()).into_option()?;
    let b = Scalar::from_repr((*il).into()).into_option()?;
    let s = a + b;
    if bool::from(s.is_zero()) {
        return None;
    }
    Some(s.to_bytes().into())
}

fn hmac512(key: &[u8], data: &[u8]) -> [u8; 64] {
    let mut mac = HmacSha512::new_from_slice(key).expect("hmac");
    mac.update(data);
    let out = mac.finalize().into_bytes();
    let mut a = [0u8; 64];
    a.copy_from_slice(&out);
    a
}

fn split(h: &[u8; 64]) -> Result<([u8; 32], [u8; 32]), WalletError> {
    let mut l = [0u8; 32];
    let mut r = [0u8; 32];
    l.copy_from_slice(&h[..32]);
    r.copy_from_slice(&h[32..]);
    Ok((l, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip_address() {
        let k = DeviceKey::generate();
        let hex = hex::encode(k.secret_bytes().unwrap());
        let got = hex_sk(&hex).unwrap();
        let again = DeviceKey::from_secret(&got).unwrap();
        assert_eq!(again.address.0, k.address.0);
    }

    #[test]
    fn abandon_about_first_account() {
        // BIP39 test mnemonic. First ETH account m/44'/60'/0'/0/0.
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = mnemonic_seed(phrase).unwrap();
        let sk = seed_to_eth_sk(&seed).unwrap();
        let key = DeviceKey::from_secret(&sk).unwrap();
        assert_eq!(
            key.address.to_hex(),
            "0x9858effd232b4033e47d90003d41ec34ecaeda94"
        );
    }

    #[test]
    fn generate_phrase_is_twelve_and_roundtrips() {
        let (k, phrase) = generate_phrase().unwrap();
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), 12);
        let seed = mnemonic_seed(&phrase).unwrap();
        let sk = seed_to_eth_sk(&seed).unwrap();
        let again = DeviceKey::from_secret(&sk).unwrap();
        assert_eq!(again.address.0, k.address.0);
    }
}
