//! vapurr-relay — the gasless meta-tx relayer.
//!
//! "Never goes down" here means: graceful shutdown that finishes an
//! in-flight batch before exiting, a `/health` endpoint an orchestrator
//! (systemd, a container platform, a load balancer) can poll, and a
//! process that's designed to be trivially restartable with no local
//! state that matters (the pending queue is in memory and re-fills from
//! client retries; the only durable state is on-chain: nonces and the
//! authorized-relayer set). None of that is kernel-level anything — that
//! would trade a supervised, restartable failure mode for one that can
//! take the whole machine down with it. See RELAY.md for the actual
//! deployment story (systemd unit, restart policy, what multi-instance
//! HA would additionally need).

mod abi;
mod api;
mod config;
mod eip712;
mod error;
mod fee;
mod queue;
mod signer;
mod simulate;
mod submit;

use std::sync::Arc;
use std::time::Duration;

use tokio::signal;

use config::Config;
use queue::{Queue, RequestStatus};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("config: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        relayer = %cfg.relayer_address.to_hex(),
        forwarder = %cfg.forwarder.to_hex(),
        chain_id = cfg.chain_id,
        "vapurr-relay starting"
    );
    tracing::warn!(
        "this relayer's hot key MUST be added to the forwarder's authorizedRelayers \
         on-chain (VapurrForwarder.setRelayer) before it can submit anything — \
         it does not do that itself"
    );

    let queue = Queue::new();
    let bind_addr = cfg.bind_addr.clone();
    let batch_max_size = cfg.batch_max_size;
    let batch_max_wait = cfg.batch_max_wait;

    let state = Arc::new(api::AppState { config: cfg, queue: queue.clone() });

    let shutdown = Arc::new(tokio::sync::Notify::new());

    let batcher = tokio::spawn(run_batcher(state.clone(), queue.clone(), batch_max_size, batch_max_wait, shutdown.clone()));

    let app = api::router(state);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("bind {bind_addr}: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!("listening on http://{bind_addr}");

    let server = axum::serve(listener, app).with_graceful_shutdown(wait_for_shutdown_signal());
    if let Err(e) = server.await {
        tracing::error!("server error: {e}");
    }

    tracing::info!("shutting down — waiting for the batcher to finish its current cycle");
    shutdown.notify_waiters();
    let _ = batcher.await;
    tracing::info!("shutdown complete");
}

async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        let mut sig = signal::unix::signal(signal::unix::SignalKind::terminate()).expect("install SIGTERM handler");
        sig.recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

/// The whole "batching" mechanism: wait until either the queue is full or
/// the oldest pending request has waited long enough, then flush.
async fn run_batcher(
    state: Arc<api::AppState>,
    queue: Arc<Queue>,
    max_size: usize,
    max_wait: Duration,
    shutdown: Arc<tokio::sync::Notify>,
) {
    loop {
        let wait_for = queue
            .oldest_wait()
            .await
            .map(|elapsed| max_wait.saturating_sub(elapsed))
            .unwrap_or(max_wait);

        tokio::select! {
            _ = queue.notify.notified() => {}
            _ = tokio::time::sleep(wait_for) => {}
            _ = shutdown.notified() => {
                flush(&state, &queue).await;
                return;
            }
        }

        let should_flush = queue.len().await >= max_size
            || queue.oldest_wait().await.map(|w| w >= max_wait).unwrap_or(false);

        if should_flush {
            flush(&state, &queue).await;
        }
    }
}

async fn flush(state: &Arc<api::AppState>, queue: &Arc<Queue>) {
    let batch = queue.drain(state.config.batch_max_size).await;
    if batch.is_empty() {
        return;
    }
    let n = batch.len();
    tracing::info!(batch_size = n, "flushing batch");

    match submit::submit_batch(&state.config, &batch).await {
        Ok(outcome) => {
            tracing::info!(
                tx_hash = %outcome.tx_hash,
                accepted = outcome.accepted_ids.len(),
                rejected = outcome.rejected.len(),
                "batch submitted"
            );
            for id in &outcome.accepted_ids {
                queue.mark(id, RequestStatus::Submitted { tx_hash: outcome.tx_hash.clone() }).await;
            }
            for (id, reason) in &outcome.rejected {
                queue.mark(id, RequestStatus::Rejected { reason: reason.clone() }).await;
            }
        }
        Err(e) => {
            tracing::error!(error = %e, batch_size = n, "batch submission failed — requeuing for retry");
            // A submission-level failure (RPC down, nonce race, signing
            // error) is the relayer's problem, not any individual
            // request's — put the whole batch back rather than dropping
            // it. Per-item rejections (bad sig, stale nonce) already came
            // back inside `Ok(outcome.rejected)` above, not here.
            queue.requeue(batch).await;
        }
    }
}
