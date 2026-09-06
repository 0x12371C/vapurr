use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::eip712::ForwardRequest;
use crate::fee;
use crate::queue::Queue;
use crate::simulate;

pub struct AppState {
    pub config: Config,
    pub queue: Arc<Queue>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/relay/submit", post(submit))
        .route("/relay/status/:id", get(status))
        .route("/relay/quote", get(quote))
        .with_state(state)
        // Permissive on purpose: this binds to 127.0.0.1 for a chrome page
        // (http://vapurr.localhost) to call directly, and nothing here is
        // cookie/session-authenticated — the real authorization boundary
        // is the EIP-712 signature itself, which an origin header can't
        // forge either way. Restrict this if the bind address ever
        // changes to something reachable off the local machine.
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "product": "vapurr-relay",
        "relayer": state.config.relayer_address.to_hex(),
        "forwarder": state.config.forwarder.to_hex(),
        "chainId": state.config.chain_id,
        "pending": state.queue.len().await,
    }))
}

#[derive(Deserialize)]
struct SubmitBody {
    #[serde(flatten)]
    req: ForwardRequest,
    sig: String,
}

/// Accepts one signed ForwardRequest. This only enqueues it — the queue's
/// background flush loop is what actually re-verifies and submits it
/// on-chain, batched with whatever else is waiting. Returning quickly here
/// is the point: nobody's wallet should sit on a spinner waiting for a
/// batch window to close.
async fn submit(State(state): State<Arc<AppState>>, Json(body): Json<SubmitBody>) -> impl IntoResponse {
    let sig = match hex::decode(body.sig.trim_start_matches("0x")) {
        Ok(b) if b.len() == 65 => b,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "sig must be a 65-byte 0x-hex string"}))).into_response(),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if body.req.valid_until <= now {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "validUntil is already in the past"}))).into_response();
    }

    // A real eth_estimateGas for this exact call, right now — replaces
    // fee.rs's flat avgCallGas guess with what this specific request
    // actually costs. Best-effort: a simulation failure (RPC hiccup)
    // falls back to the client-declared `req.gas` rather than blocking a
    // gasless submission over a pricing nicety. This does add one RPC
    // round trip to this endpoint's latency — a deliberate trade for data
    // quality, since the endpoint is already async/enqueue-only.
    let solo_gas = simulate::solo_call_gas(&state.config.rpc_url, body.req.from, body.req.to, &body.req.data)
        .await
        .ok();

    // Cheap, quick check now; the batch submitter re-verifies the actual
    // signature before anything goes on-chain.
    let id = uuid_like();
    state.queue.enqueue(id.clone(), body.req, sig, solo_gas).await;
    state.queue.notify.notify_one();

    (StatusCode::ACCEPTED, Json(json!({ "id": id, "simulatedGas": solo_gas }))).into_response()
}

async fn status(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    match state.queue.status(&id).await {
        Some(s) => Json(json!(s)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "unknown request id"}))).into_response(),
    }
}

#[derive(Deserialize)]
struct QuoteParams {
    /// Comma-separated per-call gas estimates, e.g. `?callGas=50000,80000`.
    /// A single value repeated `batchSize` times if `batchSize` is given
    /// and only one value is.
    #[serde(rename = "callGas")]
    call_gas: String,
    #[serde(rename = "batchSize")]
    batch_size: Option<usize>,
}

/// Real, computed numbers for "here's roughly what you'd pay sponsored vs.
/// self-paying" — never a hardcoded percentage. See fee.rs for why the
/// honest ceiling here is not automatically 50%.
async fn quote(State(state): State<Arc<AppState>>, Query(params): Query<QuoteParams>) -> impl IntoResponse {
    let mut values: Vec<u64> = params
        .call_gas
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();
    if values.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "callGas must be one or more numbers"}))).into_response();
    }
    if let Some(n) = params.batch_size {
        if values.len() == 1 && n > 1 {
            values = vec![values[0]; n];
        }
    }

    let estimate = fee::estimate_savings(&values);
    let solo_cost = 21_000 + values[0];
    let user_fee = fee::user_fee_gas(solo_cost, state.config.user_fee_bps);

    Json(json!({
        "estimate": estimate,
        "exampleFirstRequest": { "soloCostGas": solo_cost, "sponsoredFeeGas": user_fee, "feeBps": state.config.user_fee_bps },
    }))
    .into_response()
}

fn uuid_like() -> String {
    use sha3::{Digest, Keccak256};
    let seed = format!(
        "{}-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        std::process::id()
    );
    let hash = Keccak256::digest(seed.as_bytes());
    format!("req_{}", hex::encode(&hash[..12]))
}
