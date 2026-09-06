#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("malformed signature")]
    BadSignature,
    #[error("request expired")]
    Expired,
    #[error("stale nonce — expected {expected}, got {got}")]
    StaleNonce { expected: u64, got: u64 },
    #[error("signature does not recover to `from`")]
    SignerMismatch,
    #[error("rpc: {0}")]
    Rpc(String),
    #[error("config: {0}")]
    Config(String),
    #[error("queue: {0}")]
    Queue(String),
}
