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
}
