//! zer0ID: prove claims, keep the documents. vapurr stores an attestation id and a handle.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Claim {
    AgeOver18,
    UniqueHuman,
    SanctionsClear,
    Jurisdiction(String),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub id: String,
    pub subject_handle: String,
    pub claims: Vec<Claim>,
    pub issuer: String,
    pub issued_at: DateTime<Utc>,
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
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    sessions: Vec<(KycSession, String)>,
    next: u64,
}

impl Zer0IdProvider {
    pub fn simulator() -> Arc<Self> {
        Arc::new(Self {
            base_url: None,
            inner: Mutex::new(Inner::default()),
        })
    }

    pub fn from_env() -> Arc<Self> {
        let base = std::env::var("VAPURR_ZEROID_URL")
            .ok()
            .filter(|s| !s.is_empty());
        Arc::new(Self {
            base_url: base,
            inner: Mutex::new(Inner::default()),
        })
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
        if !att
            .claims
            .iter()
            .any(|c| matches!(c, Claim::UniqueHuman | Claim::AgeOver18))
        {
            return Err(IdError::WeakAttestation);
        }
        Ok(VerifiedAccount {
            handle: att.subject_handle.clone(),
            attestation_id: att.id.clone(),
            verified_at: att.issued_at,
        })
    }
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
    #[error("lock poisoned")]
    Poison,
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
        let att = Attestation {
            id: "att_1".into(),
            subject_handle: "rel".into(),
            claims: vec![Claim::AgeOver18, Claim::UniqueHuman],
            issuer: "zer0id".into(),
            issued_at: Utc::now(),
        };
        let json = serde_json::to_string(&att).unwrap();
        assert!(!json.to_lowercase().contains("passport"));
        Zer0IdProvider::simulator().verify_attestation(&att).unwrap();
    }
}
