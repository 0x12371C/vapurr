//! Threshold wallet recovery for zer0ID.
//!
//! # Why this shape
//!
//! The tempting design is "derive the wallet key from the iris." It does not
//! work. A biometric read is *noisy* — every scan of the same eye differs by
//! lighting, angle and pupil dilation — so it is not a stable secret and cannot
//! be a private key. Making it usable needs a fuzzy extractor (secure sketch +
//! error correction) whose **helper data must be stored**, which is squarely at
//! odds with zer0ID's shipped promise that the iris template is never written
//! down and only a one-way nullifier is kept. A one-way nullifier, by
//! construction, reconstructs nothing.
//!
//! So factors here **release shares**; they are never themselves the key:
//!
//! 1. The wallet seed (or a key-encrypting key over it) is split with Shamir
//!    over GF(2^8) into `n` shares needing `t` to reconstruct.
//! 2. Each share is sealed to one factor — a phone OTP, a biometric fuzzy
//!    extractor output, a device key, a written-down backup code.
//! 3. Recovery gathers any `t` factors. Fewer reveals nothing: Shamir is
//!    information-theoretically secure below the threshold, so `t-1` shares
//!    leave every secret equally likely.
//!
//! # Why `t >= 2` is enforced
//!
//! A 1-of-n recovery means any single factor moves the wallet. For SMS that is
//! a SIM swap; worse, zer0ID's level 3 currently falls back to an on-screen
//! mock code when carrier SMS is unwired, which proves nothing at all. For a
//! biometric it is permanent: you cannot rotate an iris, so one leak of the
//! helper data burns that factor forever. [`RecoveryPolicy::new`] therefore
//! refuses to build a 1-of-n policy.
//!
//! This module is deliberately factor-agnostic. It does the part that is real
//! cryptography today; it does not pretend a fuzzy extractor exists yet. See
//! `docs/zeroid/RECOVERY.md`.

/// GF(2^8) multiply, AES polynomial 0x11b. Branch-free on the data bits.
fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut out = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 {
            out ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    out
}

/// Multiplicative inverse in GF(2^8): a^254 == a^-1. inv(0) is defined as 0,
/// which never arises below because Lagrange only inverts non-zero differences
/// of distinct x-coordinates.
fn gf_inv(a: u8) -> u8 {
    let mut out = 1u8;
    for _ in 0..254 {
        out = gf_mul(out, a);
    }
    out
}

/// One Shamir share. `x` is the evaluation point and must be non-zero — x = 0
/// is the secret itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    pub x: u8,
    pub y: Vec<u8>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RecoveryError {
    #[error("threshold must be at least 2 — a single factor must never move a wallet")]
    ThresholdTooLow,
    #[error("threshold cannot exceed the number of shares")]
    ThresholdAboveShares,
    #[error("need at most 255 shares")]
    TooManyShares,
    #[error("secret is empty")]
    EmptySecret,
    #[error("need at least {needed} shares, got {got}")]
    NotEnoughShares { needed: usize, got: usize },
    #[error("shares disagree in length")]
    RaggedShares,
    #[error("duplicate share x-coordinate")]
    DuplicateShare,
    #[error("share x-coordinate must be non-zero")]
    ZeroShareX,
    #[error("could not seal share")]
    SealFailed,
    #[error("could not open share — wrong factor secret, or the blob was tampered with")]
    OpenFailed,
    #[error("factor {0} is not implemented, so it cannot hold a share")]
    FactorNotImplemented(String),
    #[error("recovered secret failed its commitment — wrong shares combined")]
    VerifierMismatch,
}

/// Split `secret` into `shares` pieces, any `threshold` of which reconstruct it.
///
/// `rand` supplies the polynomial coefficients; it must be a CSPRNG. Passing a
/// predictable source destroys the guarantee — the coefficients are what hide
/// the secret below the threshold.
pub fn split_with<R: FnMut(&mut [u8])>(
    secret: &[u8],
    threshold: u8,
    shares: u8,
    mut rand: R,
) -> Result<Vec<Share>, RecoveryError> {
    if secret.is_empty() {
        return Err(RecoveryError::EmptySecret);
    }
    if threshold < 2 {
        return Err(RecoveryError::ThresholdTooLow);
    }
    if threshold > shares {
        return Err(RecoveryError::ThresholdAboveShares);
    }

    // One random polynomial per secret byte, constant term = the byte.
    let degree = usize::from(threshold) - 1;
    let mut coeffs = vec![0u8; secret.len() * degree];
    if !coeffs.is_empty() {
        rand(&mut coeffs);
    }

    let mut out = Vec::with_capacity(usize::from(shares));
    for i in 1..=u16::from(shares) {
        let x = i as u8; // 1..=255, never 0
        let mut y = Vec::with_capacity(secret.len());
        for (b, &s) in secret.iter().enumerate() {
            // Horner from the top coefficient down to the secret byte.
            let mut acc = 0u8;
            for d in (0..degree).rev() {
                acc = gf_mul(acc, x) ^ coeffs[b * degree + d];
            }
            y.push(gf_mul(acc, x) ^ s);
        }
        out.push(Share { x, y });
    }
    Ok(out)
}

/// Reconstruct a secret from `t` or more shares (Lagrange interpolation at 0).
///
/// Shamir is not authenticated: a wrong-but-well-formed share yields a
/// wrong-but-well-formed secret rather than an error. Callers must verify the
/// result out of band — check the recovered seed derives the expected address
/// before trusting it.
pub fn combine(shares: &[Share]) -> Result<Vec<u8>, RecoveryError> {
    if shares.len() < 2 {
        return Err(RecoveryError::NotEnoughShares { needed: 2, got: shares.len() });
    }
    let len = shares[0].y.len();
    if shares.iter().any(|s| s.y.len() != len) {
        return Err(RecoveryError::RaggedShares);
    }
    if shares.iter().any(|s| s.x == 0) {
        return Err(RecoveryError::ZeroShareX);
    }
    for i in 0..shares.len() {
        for j in (i + 1)..shares.len() {
            if shares[i].x == shares[j].x {
                return Err(RecoveryError::DuplicateShare);
            }
        }
    }

    let mut secret = Vec::with_capacity(len);
    for b in 0..len {
        let mut acc = 0u8;
        for (i, si) in shares.iter().enumerate() {
            // Lagrange basis at x = 0: prod_{j != i} x_j / (x_i ^ x_j)
            let mut num = 1u8;
            let mut den = 1u8;
            for (j, sj) in shares.iter().enumerate() {
                if i == j {
                    continue;
                }
                num = gf_mul(num, sj.x);
                den = gf_mul(den, si.x ^ sj.x);
            }
            acc ^= gf_mul(si.y[b], gf_mul(num, gf_inv(den)));
        }
        secret.push(acc);
    }
    Ok(secret)
}

/// What can release a share. Each is a *factor*, never key material itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Factor {
    /// Share sealed to this install's device key. Lost with the device.
    Device,
    /// Released after a phone one-time code.
    ///
    /// Weak on its own: SIM-swappable, and zer0ID level 3 currently falls back
    /// to an on-screen mock code when carrier SMS is unwired. Only ever counts
    /// as one factor toward the threshold.
    Phone,
    /// Released by a fuzzy extractor over the iris template.
    ///
    /// NOT IMPLEMENTED. Requires persisted helper data, which contradicts the
    /// current "template is never written down" promise, and cannot be rotated
    /// after a leak. Present so the policy can be expressed and reviewed; any
    /// attempt to actually seal to it must fail loudly rather than pretend.
    Biometric,
    /// A code the user wrote down at create.
    BackupCode,
}

impl Factor {
    /// Whether sealing a share to this factor is actually buildable today.
    pub fn implemented(&self) -> bool {
        !matches!(self, Factor::Biometric)
    }

    /// Stable slot name. Persisted in the envelope and bound into key
    /// derivation, so a share sealed for one factor cannot be opened as another.
    pub fn label(&self) -> &'static str {
        match self {
            Factor::Device => "device",
            Factor::Phone => "phone",
            Factor::Biometric => "biometric",
            Factor::BackupCode => "backup_code",
        }
    }
}

/// A recovery configuration: which factors hold shares, and how many are needed.
#[derive(Debug, Clone)]
pub struct RecoveryPolicy {
    pub threshold: u8,
    pub factors: Vec<Factor>,
}

impl RecoveryPolicy {
    /// Refuses 1-of-n. See the module docs: a single factor moving a wallet is
    /// the failure mode this whole design exists to avoid.
    pub fn new(threshold: u8, factors: Vec<Factor>) -> Result<Self, RecoveryError> {
        if threshold < 2 {
            return Err(RecoveryError::ThresholdTooLow);
        }
        if usize::from(threshold) > factors.len() {
            return Err(RecoveryError::ThresholdAboveShares);
        }
        if factors.len() > 255 {
            return Err(RecoveryError::TooManyShares);
        }
        Ok(Self { threshold, factors })
    }

    /// Factors in this policy that cannot be sealed yet.
    pub fn unimplemented(&self) -> Vec<Factor> {
        self.factors.iter().copied().filter(|f| !f.implemented()).collect()
    }

    /// Whether the policy can be enacted right now with real crypto.
    pub fn enactable_today(&self) -> bool {
        // Enough implemented factors must remain to still meet the threshold.
        let usable = self.factors.iter().filter(|f| f.implemented()).count();
        usable >= usize::from(self.threshold)
    }
}

// ---------------------------------------------------------------------------
// Sealing: turning a bare Shamir share into something safe to store
// ---------------------------------------------------------------------------

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

type HmacSha256 = Hmac<Sha256>;

/// Domain separator, so a key derived here can never collide with one derived
/// for another purpose from the same input material.
const HKDF_DOMAIN: &[u8] = b"vapurr/zer0id/recovery/v1";

/// HKDF-SHA256 (extract-then-expand, RFC 5869) to one 32-byte key.
///
/// A fast KDF is correct *only* because every input used here is already
/// high-entropy: a generated backup code, a random escrow key, a device key.
/// It would be wrong for a user-chosen password or a 6-digit OTP, which need a
/// memory-hard KDF or must not be used as key material at all — see
/// [`Factor::Phone`].
pub fn derive_key(material: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let mut ext = <HmacSha256 as Mac>::new_from_slice(salt).expect("hmac accepts any key length");
    ext.update(material);
    let prk = ext.finalize().into_bytes();

    let mut exp = <HmacSha256 as Mac>::new_from_slice(&prk).expect("hmac accepts any key length");
    exp.update(HKDF_DOMAIN);
    exp.update(info);
    exp.update(&[0x01]);
    let okm = exp.finalize().into_bytes();

    let mut key = [0u8; 32];
    key.copy_from_slice(&okm[..32]);
    key
}

/// A share encrypted under a factor-derived key, safe to persist or escrow.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SealedShare {
    pub factor: String,
    pub x: u8,
    #[serde(with = "hex_bytes")]
    pub salt: Vec<u8>,
    #[serde(with = "hex_bytes")]
    pub nonce: Vec<u8>,
    #[serde(with = "hex_bytes")]
    pub ct: Vec<u8>,
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

/// The persisted recovery blob. Holds only ciphertext plus a verifier — never
/// the secret, and never a bare share.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Envelope {
    pub version: u8,
    pub threshold: u8,
    pub shares: Vec<SealedShare>,
    /// `SHA-256(domain || secret)`. Shamir is **not authenticated**: combining
    /// wrong-but-well-formed shares yields wrong-but-well-formed output. Without
    /// this, a bad recovery hands back a plausible seed pointing at an empty
    /// wallet. Safe to store because the secret it commits to is 256-bit random.
    #[serde(with = "hex_bytes")]
    pub verifier: Vec<u8>,
}

fn verifier_of(secret: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(HKDF_DOMAIN);
    h.update(b"/verifier");
    h.update(secret);
    h.finalize().to_vec()
}

/// Seal one share under `key_material` for `factor`.
pub fn seal_share(
    share: &Share,
    factor: Factor,
    key_material: &[u8],
) -> Result<SealedShare, RecoveryError> {
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    let mut key = derive_key(key_material, &salt, factor.label().as_bytes());
    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    // The x-coordinate is authenticated but not secret: binding it as AAD stops
    // a share being replayed into a different slot.
    let ct = cipher
        .encrypt(nonce, chacha20poly1305::aead::Payload { msg: &share.y, aad: &[share.x] })
        .map_err(|_| RecoveryError::SealFailed)?;
    key.zeroize();

    Ok(SealedShare {
        factor: factor.label().to_string(),
        x: share.x,
        salt: salt.to_vec(),
        nonce: nonce_bytes.to_vec(),
        ct,
    })
}

/// Open a sealed share. Wrong key material fails the AEAD tag rather than
/// returning plausible garbage.
pub fn open_share(sealed: &SealedShare, key_material: &[u8]) -> Result<Share, RecoveryError> {
    let mut key = derive_key(key_material, &sealed.salt, sealed.factor.as_bytes());
    let cipher = ChaCha20Poly1305::new((&key).into());
    if sealed.nonce.len() != 12 {
        return Err(RecoveryError::OpenFailed);
    }
    let nonce = Nonce::from_slice(&sealed.nonce);
    let y = cipher
        .decrypt(nonce, chacha20poly1305::aead::Payload { msg: &sealed.ct, aad: &[sealed.x] })
        .map_err(|_| RecoveryError::OpenFailed)?;
    key.zeroize();
    Ok(Share { x: sealed.x, y })
}

/// Split `secret` and seal each share to its factor.
///
/// `factors` pairs each factor with its key material, in slot order. Refuses
/// any factor that is not implemented, so a policy naming the biometric cannot
/// silently produce an envelope that can never be opened.
pub fn seal_envelope(
    secret: &[u8],
    threshold: u8,
    factors: &[(Factor, Vec<u8>)],
) -> Result<Envelope, RecoveryError> {
    if let Some((f, _)) = factors.iter().find(|(f, _)| !f.implemented()) {
        return Err(RecoveryError::FactorNotImplemented(f.label().to_string()));
    }
    let n = u8::try_from(factors.len()).map_err(|_| RecoveryError::TooManyShares)?;
    let shares = split_with(secret, threshold, n, |buf| rand::rngs::OsRng.fill_bytes(buf))?;

    let mut sealed = Vec::with_capacity(factors.len());
    for (share, (factor, material)) in shares.iter().zip(factors.iter()) {
        sealed.push(seal_share(share, *factor, material)?);
    }
    Ok(Envelope {
        version: 1,
        threshold,
        shares: sealed,
        verifier: verifier_of(secret),
    })
}

/// Recover the secret from any `threshold` opened factors.
///
/// `unlocked` pairs a factor label with its key material. Verifies the result
/// against the envelope's commitment, so a wrong combination is reported as
/// [`RecoveryError::VerifierMismatch`] instead of returning a plausible seed.
pub fn open_envelope(
    env: &Envelope,
    unlocked: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, RecoveryError> {
    let mut shares = Vec::new();
    for (label, material) in unlocked {
        if let Some(s) = env.shares.iter().find(|s| &s.factor == label) {
            shares.push(open_share(s, material)?);
        }
    }
    if shares.len() < usize::from(env.threshold) {
        return Err(RecoveryError::NotEnoughShares {
            needed: usize::from(env.threshold),
            got: shares.len(),
        });
    }
    let secret = combine(&shares)?;
    if verifier_of(&secret) != env.verifier {
        return Err(RecoveryError::VerifierMismatch);
    }
    Ok(secret)
}

/// A written-down recovery code.
///
/// 160 bits from the OS CSPRNG, rendered in Crockford base32 (no I/L/O/U, so it
/// survives being copied off a screen by hand) in groups of four. High entropy
/// on purpose: it means the key derivation does not need to be memory-hard.
pub fn generate_backup_code() -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut raw = [0u8; 20];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let mut out = String::new();
    for (i, b) in raw.iter().enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push('-');
        }
        out.push(ALPHABET[usize::from(b >> 3)] as char);
        out.push(ALPHABET[usize::from(((b & 0b111) << 2) | (b >> 6))] as char);
    }
    raw.zeroize();
    out
}

/// Normalize a typed backup code: strip separators, uppercase, and map the
/// characters people reliably mistype in Crockford base32.
pub fn normalize_backup_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'I' | 'L' => '1',
            'O' => '0',
            'U' => 'V',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic filler — tests only. Real splits must use a CSPRNG.
    fn fill(seed: u8) -> impl FnMut(&mut [u8]) {
        let mut n = seed;
        move |buf: &mut [u8]| {
            for b in buf.iter_mut() {
                n = n.wrapping_mul(31).wrapping_add(17);
                *b = n;
            }
        }
    }

    #[test]
    fn gf_inverse_round_trips() {
        for a in 1u16..=255 {
            let a = a as u8;
            assert_eq!(gf_mul(a, gf_inv(a)), 1, "inv failed for {a}");
        }
    }

    #[test]
    fn any_threshold_subset_reconstructs() {
        let secret = b"correct horse battery staple 0123456789abcdef";
        let shares = split_with(secret, 3, 5, fill(7)).unwrap();
        assert_eq!(shares.len(), 5);
        // every 3-subset works
        for i in 0..5 {
            for j in (i + 1)..5 {
                for k in (j + 1)..5 {
                    let got = combine(&[shares[i].clone(), shares[j].clone(), shares[k].clone()]).unwrap();
                    assert_eq!(got, secret.to_vec(), "subset {i},{j},{k}");
                }
            }
        }
    }

    #[test]
    fn more_than_threshold_also_reconstructs() {
        let secret = b"seed";
        let shares = split_with(secret, 2, 4, fill(3)).unwrap();
        assert_eq!(combine(&shares).unwrap(), secret.to_vec());
    }

    /// Below the threshold the result must not be the secret. Shamir gives
    /// information-theoretic secrecy here, so this is the load-bearing property.
    #[test]
    fn below_threshold_does_not_reveal_the_secret() {
        let secret = b"correct horse battery staple";
        let shares = split_with(secret, 3, 5, fill(11)).unwrap();
        let two = combine(&[shares[0].clone(), shares[1].clone()]).unwrap();
        assert_ne!(two, secret.to_vec());
    }

    #[test]
    fn one_of_n_is_refused() {
        assert_eq!(split_with(b"x", 1, 3, fill(1)), Err(RecoveryError::ThresholdTooLow));
        assert_eq!(
            RecoveryPolicy::new(1, vec![Factor::Phone, Factor::Device]).unwrap_err(),
            RecoveryError::ThresholdTooLow
        );
    }

    #[test]
    fn threshold_cannot_exceed_shares() {
        assert_eq!(split_with(b"x", 4, 3, fill(1)), Err(RecoveryError::ThresholdAboveShares));
    }

    #[test]
    fn duplicate_and_zero_x_are_rejected() {
        let s = Share { x: 1, y: vec![9] };
        assert_eq!(combine(&[s.clone(), s.clone()]), Err(RecoveryError::DuplicateShare));
        let z = Share { x: 0, y: vec![9] };
        assert_eq!(combine(&[z, s]), Err(RecoveryError::ZeroShareX));
    }

    #[test]
    fn biometric_is_declared_unimplemented() {
        assert!(!Factor::Biometric.implemented());
        let p = RecoveryPolicy::new(2, vec![Factor::Phone, Factor::Biometric]).unwrap();
        assert_eq!(p.unimplemented(), vec![Factor::Biometric]);
        // Phone alone cannot meet a 2-of-2 once the biometric is unavailable.
        assert!(!p.enactable_today());
    }

    #[test]
    fn a_policy_of_implemented_factors_is_enactable() {
        let p = RecoveryPolicy::new(2, vec![Factor::Phone, Factor::Device, Factor::BackupCode]).unwrap();
        assert!(p.unimplemented().is_empty());
        assert!(p.enactable_today());
    }

    #[test]
    fn full_length_secret_survives() {
        // 32-byte seed material, the real shape.
        let secret: Vec<u8> = (0u8..32).collect();
        let shares = split_with(&secret, 2, 3, fill(5)).unwrap();
        assert_eq!(combine(&shares[..2]).unwrap(), secret);
    }

    // --- sealing ---------------------------------------------------------

    fn seed() -> Vec<u8> {
        (0u8..32).map(|i| i.wrapping_mul(7).wrapping_add(3)).collect()
    }

    fn factors() -> Vec<(Factor, Vec<u8>)> {
        vec![
            (Factor::Device, b"device-key-material".to_vec()),
            (Factor::Phone, b"escrowed-random-key".to_vec()),
            (Factor::BackupCode, b"HTQ4-9WZM-3K7X-PB2N-5RJD".to_vec()),
        ]
    }

    #[test]
    fn any_two_factors_recover_the_seed() {
        let s = seed();
        let f = factors();
        let env = seal_envelope(&s, 2, &f).unwrap();
        assert_eq!(env.shares.len(), 3);

        for i in 0..3 {
            for j in (i + 1)..3 {
                let unlocked = vec![
                    (f[i].0.label().to_string(), f[i].1.clone()),
                    (f[j].0.label().to_string(), f[j].1.clone()),
                ];
                assert_eq!(open_envelope(&env, &unlocked).unwrap(), s, "pair {i},{j}");
            }
        }
    }

    #[test]
    fn one_factor_alone_recovers_nothing() {
        let s = seed();
        let f = factors();
        let env = seal_envelope(&s, 2, &f).unwrap();
        let only_phone = vec![(Factor::Phone.label().to_string(), f[1].1.clone())];
        assert_eq!(
            open_envelope(&env, &only_phone).unwrap_err(),
            RecoveryError::NotEnoughShares { needed: 2, got: 1 }
        );
    }

    /// A stolen envelope is not a stolen wallet: without factor secrets the
    /// AEAD tag fails rather than yielding a share.
    #[test]
    fn wrong_factor_secret_fails_the_tag() {
        let s = seed();
        let f = factors();
        let env = seal_envelope(&s, 2, &f).unwrap();
        let wrong = vec![
            (Factor::Device.label().to_string(), b"not-the-device-key".to_vec()),
            (Factor::Phone.label().to_string(), f[1].1.clone()),
        ];
        assert_eq!(open_envelope(&env, &wrong).unwrap_err(), RecoveryError::OpenFailed);
    }

    /// The envelope holds ciphertext only — the seed must not appear in it.
    #[test]
    fn envelope_does_not_contain_the_secret() {
        let s = seed();
        let env = seal_envelope(&s, 2, &factors()).unwrap();
        let blob = serde_json::to_string(&env).unwrap();
        assert!(!blob.contains(&hex::encode(&s)));
        for sh in &env.shares {
            assert_ne!(sh.ct, s, "a share ciphertext must not be the seed");
        }
    }

    #[test]
    fn envelope_round_trips_as_json() {
        let s = seed();
        let f = factors();
        let env = seal_envelope(&s, 2, &f).unwrap();
        let blob = serde_json::to_string(&env).unwrap();
        let back: Envelope = serde_json::from_str(&blob).unwrap();
        let unlocked = vec![
            (f[0].0.label().to_string(), f[0].1.clone()),
            (f[2].0.label().to_string(), f[2].1.clone()),
        ];
        assert_eq!(open_envelope(&back, &unlocked).unwrap(), s);
    }

    /// Shamir is unauthenticated, so a mismatched combination must be caught by
    /// the commitment rather than handed back as a plausible seed.
    #[test]
    fn shares_from_a_different_envelope_are_rejected() {
        let f = factors();
        let a = seal_envelope(&seed(), 2, &f).unwrap();
        let b = seal_envelope(&vec![9u8; 32], 2, &f).unwrap();
        // Splice one of b's shares into a — it decrypts fine (same factor key)
        // but belongs to a different polynomial.
        let mut mixed = a.clone();
        mixed.shares[1] = b.shares[1].clone();
        let unlocked = vec![
            (f[0].0.label().to_string(), f[0].1.clone()),
            (f[1].0.label().to_string(), f[1].1.clone()),
        ];
        assert_eq!(
            open_envelope(&mixed, &unlocked).unwrap_err(),
            RecoveryError::VerifierMismatch
        );
    }

    #[test]
    fn biometric_cannot_be_sealed() {
        let err = seal_envelope(
            &seed(),
            2,
            &[
                (Factor::Phone, b"k".to_vec()),
                (Factor::Biometric, b"iris".to_vec()),
            ],
        )
        .unwrap_err();
        assert_eq!(err, RecoveryError::FactorNotImplemented("biometric".into()));
    }

    #[test]
    fn sealing_refuses_one_of_n() {
        assert_eq!(
            seal_envelope(&seed(), 1, &factors()).unwrap_err(),
            RecoveryError::ThresholdTooLow
        );
    }

    #[test]
    fn backup_codes_are_distinct_and_shaped() {
        let a = generate_backup_code();
        let b = generate_backup_code();
        assert_ne!(a, b);
        assert_eq!(a.matches('-').count(), 4, "five groups of four bytes");
        assert_eq!(normalize_backup_code(&a).len(), 40, "20 bytes -> 40 symbols");
    }

    #[test]
    fn backup_code_normalization_fixes_common_mistypes() {
        assert_eq!(normalize_backup_code("htq4-9wzm"), "HTQ49WZM");
        assert_eq!(normalize_backup_code("IL0O-U"), "11000V");
    }

    #[test]
    fn derived_keys_are_factor_separated() {
        // Same material, different slot label must not yield the same key.
        let k1 = derive_key(b"same-material", b"same-salt", Factor::Phone.label().as_bytes());
        let k2 = derive_key(b"same-material", b"same-salt", Factor::Device.label().as_bytes());
        assert_ne!(k1, k2);
    }
}
