use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, Notify};

use crate::eip712::ForwardRequest;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Submitted { tx_hash: String },
    Confirmed { tx_hash: String, success: bool },
    Rejected { reason: String },
}

pub struct PendingRequest {
    pub id: String,
    pub req: ForwardRequest,
    pub sig: Vec<u8>,
    pub received_at: Instant,
}

#[derive(Default)]
struct Inner {
    pending: Vec<PendingRequest>,
    status: HashMap<String, RequestStatus>,
}

/// The relayer's whole notion of "what's waiting to be batched." One
/// process, one queue — see RELAY.md for why multi-instance HA needs more
/// than duplicating this (nonce coordination), which this pass doesn't
/// attempt.
pub struct Queue {
    inner: Mutex<Inner>,
    pub notify: Notify,
}

impl Queue {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { inner: Mutex::new(Inner::default()), notify: Notify::new() })
    }

    pub async fn enqueue(&self, id: String, req: ForwardRequest, sig: Vec<u8>) {
        let mut g = self.inner.lock().await;
        g.status.insert(id.clone(), RequestStatus::Pending);
        g.pending.push(PendingRequest { id, req, sig, received_at: Instant::now() });
        drop(g);
        self.notify.notify_one();
    }

    pub async fn status(&self, id: &str) -> Option<RequestStatus> {
        self.inner.lock().await.status.get(id).cloned()
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.pending.len()
    }

    /// How long the oldest still-pending request has been waiting — the
    /// batch-max-wait clock is driven off this, not off when the queue
    /// last emptied.
    pub async fn oldest_wait(&self) -> Option<Duration> {
        self.inner.lock().await.pending.first().map(|p| p.received_at.elapsed())
    }

    pub async fn drain(&self, max: usize) -> Vec<PendingRequest> {
        let mut g = self.inner.lock().await;
        let n = g.pending.len().min(max);
        g.pending.drain(0..n).collect()
    }

    /// Put a drained batch back at the front of the queue — used when a
    /// submission attempt fails before anything was actually sent
    /// on-chain (an RPC error, a nonce race, etc.), so those requests get
    /// picked up again on the next flush instead of silently vanishing.
    /// Front, not back: they're the oldest waiting requests by
    /// construction, and `received_at` (unchanged here) is what the
    /// max-wait clock reads.
    pub async fn requeue(&self, mut items: Vec<PendingRequest>) {
        let mut g = self.inner.lock().await;
        items.append(&mut g.pending);
        g.pending = items;
    }

    pub async fn mark(&self, id: &str, status: RequestStatus) {
        self.inner.lock().await.status.insert(id.to_string(), status);
    }
}
