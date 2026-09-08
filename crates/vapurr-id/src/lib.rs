//! zer0ID: prove claims, keep the documents. vapurr stores an attestation id and a handle.
//! Earn payout requires a loaded VerifiedAccount. Never auto-write Proven.
//!
//! Hardened: an attestation is only ever trusted if it recovers to a
//! configured issuer address (`VAPURR_ZEROID_ISSUER_ADDRESS`). Before this,
//! `verify_attestation` accepted any locally hand-written `id.json` with a
//! non-empty claims list — that's the sybil hole docs/ketpay/SYBIL.md warns
//! about (one human, many wallets, many payouts). It's closed now: no
//! signature from a trusted issuer, no VerifiedAccount, full stop.

mod sig;

use chrono::{DateTime, Utc};
use k256::ecdsa::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Secret Lab KYC. Chrome opens this; vapurr does not host a KYC form.
/// Apex only — www.thesecretlab.app's cert expired 2026-05-20 and the host is HSTS.
pub const KYC_URL: &str = "https://thesecretlab.app/kyc";
pub const ID_FILE: &str = "id.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Claim {
    AgeOver18,
    UniqueHuman,
    SanctionsClear,
    Jurisdiction(String),
}

impl Claim {
    /// Canonical wire form used in the signed message — must match
    /// vapurrdb's `crypto/signatures.ts` claim strings exactly.
    pub fn canonical(&self) -> String {
        match self {
            Claim::AgeOver18 => "AgeOver18".to_string(),
            Claim::UniqueHuman => "UniqueHuman".to_string(),
            Claim::SanctionsClear => "SanctionsClear".to_string(),
            Claim::Jurisdiction(code) => format!("Jurisdiction:{code}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycStatus {
    Pending,
    Proven,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycSession {
    pub id: String,
    pub status: KycStatus,
    pub started_at: DateTime<Utc>,
}

/// A zer0ID attestation. `sig` is an EIP-191 signature over
/// `signing_message()` from the wallet at `issuer` — but `issuer` is a
/// self-reported label until verification recovers the real signer, so
/// never branch on `issuer` directly; only on what `verify_attestation`
/// returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub id: String,
    pub subject_handle: String,
    pub claims: Vec<Claim>,
    /// ZER0ID's progressive trust scale: 0 unique-human, 1 age, 2
    /// jurisdiction, 3 full KYC, 4 accredited investor.
    pub trust_level: u8,
    /// Sybil-resistance nullifier — one per underlying human per issuer.
    /// A real one is a Poseidon hash of a device-bound secret from Secret
    /// Lab's circuit; never derivable from a public handle. vapurr never
    /// computes one itself, only carries and checks it.
    pub nullifier: String,
    pub issuer: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    /// `0x`-hex EIP-191 signature over `signing_message()`.
    pub sig: String,
}

impl Attestation {
    /// The exact message the issuer signs. Field order and formatting are
    /// load-bearing — vapurrdb's `attestationSigningMessage` must produce
    /// byte-identical output or every cross-checked attestation fails.
    pub fn signing_message(&self) -> String {
        let claims = self
            .claims
            .iter()
            .map(Claim::canonical)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "zer0ID Attestation\nid:{}\nhandle:{}\ntrustLevel:{}\nnullifier:{}\nclaims:{}\nissuedAt:{}\nexpiresAt:{}",
            self.id,
            self.subject_handle,
            self.trust_level,
            self.nullifier,
            claims,
            self.issued_at.timestamp(),
            self.expires_at.map(|t| t.timestamp()).unwrap_or(0),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAccount {
    pub handle: String,
    pub attestation_id: String,
    pub verified_at: DateTime<Utc>,
}

pub trait IdentityProvider: Send + Sync {
    fn start_session(&self, requested_handle: &str) -> Result<KycSession, IdError>;
    fn poll_session(&self, id: &str) -> Result<KycSession, IdError>;
    fn complete_session(&self, id: &str) -> Result<VerifiedAccount, IdError>;
    fn verify_attestation(&self, att: &Attestation) -> Result<VerifiedAccount, IdError>;
}

pub struct Zer0IdProvider {
    pub base_url: Option<String>,
    /// Lowercase `0x` addresses trusted to sign attestations this provider accepts.
    pub trusted_issuers: Vec<String>,
    /// Present only on the local simulator, which signs completed sessions
    /// with its own key so the same real verification path runs in dev/tests.
    issuer_key: Option<SigningKey>,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    sessions: Vec<(KycSession, String)>,
    next: u64,
}

impl Zer0IdProvider {
    pub fn simulator() -> Arc<Self> {
        let key = SigningKey::random(&mut OsRng);
        let address = sig::address_of(&key);
        Arc::new(Self {
            base_url: None,
            trusted_issuers: vec![address],
            issuer_key: Some(key),
            inner: Mutex::new(Inner::default()),
        })
    }

    pub fn from_env() -> Arc<Self> {
        let base = std::env::var("VAPURR_ZEROID_URL")
            .ok()
            .filter(|s| !s.is_empty());
        Arc::new(Self {
            base_url: base,
            trusted_issuers: trusted_issuers_from_env(),
            issuer_key: None,
            inner: Mutex::new(Inner::default()),
        })
    }

    /// This simulator's own issuer address — for wiring `id.json` fixtures
    /// or `VAPURR_ZEROID_ISSUER_ADDRESS` in local dev/tests.
    pub fn issuer_address(&self) -> Option<String> {
        self.issuer_key.as_ref().map(sig::address_of)
    }

    /// SIMULATOR ONLY: mint and self-sign a fresh Attestation for `handle`,
    /// for dev/test flows that need a realistic, verifiable `id.json`
    /// rather than a hand-typed one. The nullifier here is derived from
    /// the handle purely for repeatable fixtures — see the field doc on
    /// `Attestation::nullifier` for why that's never true of a real one.
    pub fn issue_attestation(
        &self,
        handle: &str,
        trust_level: u8,
        claims: Vec<Claim>,
    ) -> Result<Attestation, IdError> {
        let key = self.issuer_key.as_ref().ok_or(IdError::IssuerUnwired)?;
        let handle = normalize_handle(handle)?;
        let mut g = self.inner.lock().map_err(|_| IdError::Poison)?;
        g.next += 1;
        let id = format!("att_sim_{}", g.next);
        let nullifier = format!("0x{}", hex::encode(Keccak256::digest(handle.as_bytes())));
        let mut att = Attestation {
            id,
            subject_handle: handle,
            claims,
            trust_level,
            nullifier,
            issuer: sig::address_of(key),
            issued_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(365)),
            sig: String::new(),
        };
        att.sig = sig::sign_message(&att.signing_message(), key);
        Ok(att)
    }
}

impl IdentityProvider for Zer0IdProvider {
    fn start_session(&self, requested_handle: &str) -> Result<KycSession, IdError> {
        let handle = normalize_handle(requested_handle)?;
        let mut g = self.inner.lock().map_err(|_| IdError::Poison)?;
        g.next += 1;
        let sess = KycSession {
            id: format!("z0_{}", g.next),
            status: KycStatus::Pending,
            started_at: Utc::now(),
        };
        g.sessions.push((sess.clone(), handle));
        Ok(sess)
    }

    fn poll_session(&self, id: &str) -> Result<KycSession, IdError> {
        let g = self.inner.lock().map_err(|_| IdError::Poison)?;
        g.sessions
            .iter()
            .find(|(s, _)| s.id == id)
            .map(|(s, _)| s.clone())
            .ok_or(IdError::UnknownSession)
    }

    fn complete_session(&self, id: &str) -> Result<VerifiedAccount, IdError> {
        // Only the simulator holds a key to vouch with. A live issuer is
        // reached over HTTP (not built yet — see docs/zeroid/RHC.md); the
        // shell must not fabricate a Proven result in the meantime.
        self.issuer_key.as_ref().ok_or(IdError::IssuerUnwired)?;
        let mut g = self.inner.lock().map_err(|_| IdError::Poison)?;
        let row = g
            .sessions
            .iter_mut()
            .find(|(s, _)| s.id == id)
            .ok_or(IdError::UnknownSession)?;
        row.0.status = KycStatus::Proven;
        Ok(VerifiedAccount {
            handle: row.1.clone(),
            attestation_id: format!("att_{}", row.0.id),
            verified_at: Utc::now(),
        })
    }

    fn verify_attestation(&self, att: &Attestation) -> Result<VerifiedAccount, IdError> {
        if att.id.is_empty() || att.subject_handle.trim().is_empty() || att.nullifier.is_empty() {
            return Err(IdError::WeakAttestation);
        }
        if !att
            .claims
            .iter()
            .any(|c| matches!(c, Claim::UniqueHuman | Claim::AgeOver18))
        {
            return Err(IdError::WeakAttestation);
        }
        if let Some(expires_at) = att.expires_at {
            if expires_at < Utc::now() {
                return Err(IdError::Expired);
            }
        }
        if self.trusted_issuers.is_empty() {
            return Err(IdError::NoTrustedIssuers);
        }
        let signer = sig::recover_signer(&att.signing_message(), &att.sig)?;
        if !self
            .trusted_issuers
            .iter()
            .any(|t| t.eq_ignore_ascii_case(&signer))
        {
            return Err(IdError::UntrustedIssuer);
        }
        Ok(VerifiedAccount {
            handle: att.subject_handle.clone(),
            attestation_id: att.id.clone(),
            verified_at: att.issued_at,
        })
    }
}

pub fn payout_ready(acct: &VerifiedAccount) -> bool {
    !acct.handle.is_empty() && !acct.attestation_id.is_empty()
}

/// Trusted issuer addresses from `VAPURR_ZEROID_ISSUER_ADDRESS`
/// (comma-separated, case-insensitive).
pub fn trusted_issuers_from_env() -> Vec<String> {
    std::env::var("VAPURR_ZEROID_ISSUER_ADDRESS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(|a| a.trim().to_ascii_lowercase())
                .filter(|a| !a.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Load a verified account from the profile dir. Missing/empty/weak/
/// unsigned/expired/untrusted-signer → None. Verified against
/// `trusted_issuers` — never against the simulator's own key unless the
/// caller explicitly passes it in, so a hand-written `id.json` can no
/// longer self-attest (see the module doc for why that mattered).
pub fn load_verified(profile_dir: &Path, trusted_issuers: &[String]) -> Option<VerifiedAccount> {
    if trusted_issuers.is_empty() {
        return None; // nothing configured to trust, ever
    }
    let raw = std::fs::read_to_string(profile_dir.join(ID_FILE)).ok()?;
    let att: Attestation = serde_json::from_str(&raw).ok()?;
    let verifier = Zer0IdProvider {
        base_url: None,
        trusted_issuers: trusted_issuers.to_vec(),
        issuer_key: None,
        inner: Mutex::new(Inner::default()),
    };
    verifier.verify_attestation(&att).ok()
}

/// The zer0ID ladder, as actually implemented by the issuer. Kept here rather
/// than in the chrome so the surface can never invent a level that the backend
/// does not issue. `docs/zeroid/RHC.md` is the source of truth for what each
/// level really proves — keep the two in step.
pub const LEVELS: [(u8, &str, &str, &str); 5] = [
    (0, "Wallet", "Controls this device key", "secp256k1 challenge signature"),
    (1, "Age", "Self-attested 18+", "Unverified self-declaration"),
    (2, "Region", "Not in an embargoed jurisdiction", "IP geolocation, country-level only"),
    (3, "Phone", "Controls a phone number", "SMS one-time code — falls back to an on-screen mock code when carrier SMS is unwired, which proves nothing about the number"),
    (4, "Unique human", "One account per person", "Ocular scan; open-iris template, kept as a nullifier"),
];

/// Device zer0ID state for the `vapurr://id` surface.
///
/// Reports `verified` strictly from [`load_verified`] — a signature from a
/// trusted issuer, unexpired. `claimedLevel` is whatever the on-disk file says
/// and is reported separately and never merged into `trustLevel`, so a
/// hand-edited `id.json` shows up as a claim that does not verify instead of
/// silently reading as Proven.
pub fn status_json(profile_dir: &Path, trusted_issuers: &[String]) -> serde_json::Value {
    use serde_json::json;

    let raw = std::fs::read_to_string(profile_dir.join(ID_FILE)).ok();
    let parsed: Option<Attestation> = raw.as_deref().and_then(|r| serde_json::from_str(r).ok());
    let verified = load_verified(profile_dir, trusted_issuers);

    let claimed_level = parsed.as_ref().map(|a| a.trust_level).unwrap_or(0);
    let level = if verified.is_some() { claimed_level } else { 0 };

    let reason = if trusted_issuers.is_empty() {
        "issuer not configured on this device"
    } else if raw.is_none() {
        "no attestation on this device"
    } else if parsed.is_none() {
        "attestation file is unreadable"
    } else if verified.is_none() {
        "attestation does not verify against a trusted issuer"
    } else {
        ""
    };

    let claims: Vec<String> = parsed
        .as_ref()
        .map(|a| a.claims.iter().map(|c| c.canonical()).collect())
        .unwrap_or_default();
    let expires_at = parsed.as_ref().and_then(|a| a.expires_at).map(|t| t.timestamp());

    let levels: Vec<serde_json::Value> = LEVELS
        .iter()
        .map(|(n, name, proves, method)| {
            json!({
                "level": n,
                "name": name,
                "proves": proves,
                "method": method,
                "done": verified.is_some() && *n <= level,
                "biometric": *n == 4,
            })
        })
        .collect();

    json!({
        "verified": verified.is_some(),
        "trustLevel": level,
        "claimedLevel": claimed_level,
        "handle": verified.as_ref().map(|v| v.handle.clone()).unwrap_or_default(),
        "claims": claims,
        "expiresAt": expires_at,
        "hasNullifier": parsed.as_ref().map(|a| !a.nullifier.is_empty()).unwrap_or(false),
        "issuerWired": !trusted_issuers.is_empty(),
        "reason": reason,
        "kycUrl": KYC_URL,
        // Ladder order is phone → scan; level 4 will not issue without 3.
        "phoneUrl": format!("{KYC_URL}/phone"),
        "scanUrl": format!("{KYC_URL}/scan"),
        "levels": levels,
    })
}

fn normalize_handle(h: &str) -> Result<String, IdError> {
    let h = h.trim().trim_start_matches('@').to_ascii_lowercase();
    if h.len() < 3 || h.len() > 24 || !h.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(IdError::BadHandle);
    }
    Ok(h)
}

#[derive(Debug, thiserror::Error)]
pub enum IdError {
    #[error("unknown session")]
    UnknownSession,
    #[error("handle must be 3-24 chars, letters/digits/_")]
    BadHandle,
    #[error("attestation missing required claims")]
    WeakAttestation,
    #[error("attestation signature is malformed")]
    BadSignature,
    #[error("attestation has expired")]
    Expired,
    #[error("no trusted zer0ID issuer configured — set VAPURR_ZEROID_ISSUER_ADDRESS")]
    NoTrustedIssuers,
    #[error("attestation signer is not a trusted zer0ID issuer")]
    UntrustedIssuer,
    #[error("lock poisoned")]
    Poison,
    #[error("live issuer is not wired; no fake Proven")]
    IssuerUnwired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulator_verifies_handle() {
        let p = Zer0IdProvider::simulator();
        let s = p.start_session("@Ada_Lovelace").unwrap();
        let acct = p.complete_session(&s.id).unwrap();
        assert_eq!(acct.handle, "ada_lovelace");
    }

    #[test]
    fn no_pii_on_attestation() {
        let p = Zer0IdProvider::simulator();
        let att = p
            .issue_attestation("rel", 0, vec![Claim::AgeOver18, Claim::UniqueHuman])
            .unwrap();
        let json = serde_json::to_string(&att).unwrap();
        assert!(!json.to_lowercase().contains("passport"));
        p.verify_attestation(&att).unwrap();
    }

    #[test]
    fn live_url_refuses_simulator_complete() {
        let p = Zer0IdProvider {
            base_url: Some("https://thesecretlab.app".into()),
            trusted_issuers: vec![],
            issuer_key: None,
            inner: Mutex::new(Inner::default()),
        };
        let s = p.start_session("relic").unwrap();
        assert!(matches!(
            p.complete_session(&s.id),
            Err(IdError::IssuerUnwired)
        ));
    }

    #[test]
    fn missing_id_json_is_not_proven() {
        let dir = std::env::temp_dir().join(format!("vapurr-id-miss-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let trusted = vec!["0x0000000000000000000000000000000000dead".to_string()];
        assert!(load_verified(&dir, &trusted).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_trusted_issuers_is_never_proven_even_if_signed() {
        let p = Zer0IdProvider::simulator();
        let att = p
            .issue_attestation("rel", 0, vec![Claim::AgeOver18])
            .unwrap();
        let dir = std::env::temp_dir().join(format!("vapurr-id-notrust-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(ID_FILE), serde_json::to_string(&att).unwrap()).unwrap();
        assert!(load_verified(&dir, &[]).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bare_hand_written_account_is_not_proven() {
        // The old shape this repo used to accept before signatures existed.
        // It must not deserialize into a valid, verifiable Attestation.
        let dir = std::env::temp_dir().join(format!("vapurr-id-bare-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join(ID_FILE),
            r#"{"handle":"rel","attestation_id":"att_fake","verified_at":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();
        let trusted = vec!["0x0000000000000000000000000000000000dead".to_string()];
        assert!(load_verified(&dir, &trusted).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn weak_attestation_is_not_proven_even_when_signed() {
        let p = Zer0IdProvider::simulator();
        let att = p
            .issue_attestation("rel", 2, vec![Claim::Jurisdiction("us".into())])
            .unwrap();
        let dir = std::env::temp_dir().join(format!("vapurr-id-weak-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(ID_FILE), serde_json::to_string(&att).unwrap()).unwrap();
        let trusted = vec![p.issuer_address().unwrap()];
        assert!(load_verified(&dir, &trusted).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn untrusted_signer_is_not_proven() {
        let issuer = Zer0IdProvider::simulator();
        let attacker = Zer0IdProvider::simulator(); // a different key, self-declared trusted only to itself
        let forged = attacker
            .issue_attestation("rel", 0, vec![Claim::UniqueHuman])
            .unwrap();
        let dir = std::env::temp_dir().join(format!("vapurr-id-forged-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(ID_FILE), serde_json::to_string(&forged).unwrap()).unwrap();
        // Only `issuer`'s address is actually trusted — attacker's signature is real, but not from a trusted key.
        let trusted = vec![issuer.issuer_address().unwrap()];
        assert!(load_verified(&dir, &trusted).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expired_attestation_is_not_proven() {
        let p = Zer0IdProvider::simulator();
        // Build + re-sign by hand so expiry — not a stale signature — is what's under test.
        let mut att = p
            .issue_attestation("rel", 0, vec![Claim::UniqueHuman])
            .unwrap();
        att.expires_at = Some(Utc::now() - chrono::Duration::days(1));
        let key = p.issuer_key.as_ref().unwrap();
        att.sig = sig::sign_message(&att.signing_message(), key);

        assert!(matches!(p.verify_attestation(&att), Err(IdError::Expired)));

        let dir = std::env::temp_dir().join(format!("vapurr-id-expired-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(ID_FILE), serde_json::to_string(&att).unwrap()).unwrap();
        let trusted = vec![p.issuer_address().unwrap()];
        assert!(load_verified(&dir, &trusted).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_human_attestation_loads() {
        let p = Zer0IdProvider::simulator();
        let att = p
            .issue_attestation("rel", 0, vec![Claim::AgeOver18, Claim::UniqueHuman])
            .unwrap();
        let dir = std::env::temp_dir().join(format!("vapurr-id-ok-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(ID_FILE), serde_json::to_string(&att).unwrap()).unwrap();
        let trusted = vec![p.issuer_address().unwrap()];
        let acct = load_verified(&dir, &trusted).expect("attestation should verify");
        assert_eq!(acct.handle, "rel");
        assert_eq!(acct.attestation_id, att.id);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
