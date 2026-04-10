use std::sync::Arc;

use axum::{
    Router,
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::config::Config;
use crate::proof::{self, CommitmentProof};
use crate::x402;

/// Request body for `/verify` and `/access`.
#[derive(Debug, Deserialize)]
struct ProofRequest {
    proof: CommitmentProof,
}

/// Protected resource returned after a successful proof + payment.
#[derive(Debug, Serialize)]
struct ProtectedResource {
    id: String,
    content: String,
    verified_by: String,
    payment_status: String,
}

pub fn create_router(config: Arc<Config>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/verify", post(verify_proof))
        .route("/access", post(access_resource))
        .route("/payment-info", get(payment_info))
        .with_state(config)
        .layer(CorsLayer::permissive())
}

async fn index(State(cfg): State<Arc<Config>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "ProofPay",
        "description": "ZK-gated x402 access for autonomous agents on Stellar",
        "version": env!("CARGO_PKG_VERSION"),
        "network": cfg.network,
        "endpoints": {
            "POST /verify": "Verify a ZK commitment proof",
            "POST /access": "Access a gated resource (requires valid proof + x402 payment)",
            "GET /payment-info": "Get x402 payment requirements",
            "GET /health": "Health check"
        },
        "flow": [
            "1. Agent generates a commitment proof (SHA-256 of a secret credential, signed with Ed25519)",
            "2. POST /verify to check the proof is accepted",
            "3. POST /access without payment returns HTTP 402 with x402 requirements",
            "4. Agent builds an X-Payment header from a Stellar payment and retries",
            "5. Server verifies proof + payment and returns the gated content"
        ]
    }))
}

async fn health(State(cfg): State<Arc<Config>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "proofpay",
        "network": cfg.network,
        "mock_payments": cfg.mock_payments
    }))
}

/// Verify a ZK commitment proof without requiring payment.
async fn verify_proof(
    State(_cfg): State<Arc<Config>>,
    Json(req): Json<ProofRequest>,
) -> impl IntoResponse {
    let result = proof::verify_commitment(&req.proof);

    let status = if result.valid {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(serde_json::to_value(result).unwrap()))
}

/// Access a gated resource.
///
/// Flow:
/// 1. Verify the ZK proof
/// 2. If no X-Payment header → return 402 with x402 payment requirements
/// 3. Otherwise verify the payment via the configured facilitator
/// 4. If both succeed, return the gated content
async fn access_resource(
    State(cfg): State<Arc<Config>>,
    headers: HeaderMap,
    Json(req): Json<ProofRequest>,
) -> impl IntoResponse {
    // Step 1: ZK proof
    let proof_result = proof::verify_commitment(&req.proof);
    if !proof_result.valid {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Invalid proof",
                "details": proof_result.error,
                "hint": "Your zero-knowledge proof is invalid. Generate a new commitment and sign it correctly."
            })),
        );
    }

    // Step 2: x402 payment header
    let payment_header = headers.get("X-Payment").and_then(|v| v.to_str().ok());

    match payment_header {
        None => {
            let payment_req = x402::payment_required_response(&cfg);
            (
                StatusCode::PAYMENT_REQUIRED,
                Json(serde_json::json!({
                    "error": "Payment required",
                    "proof_status": "valid",
                    "x402": payment_req,
                    "hint": "Your proof is valid. Include an X-Payment header with a signed x402 payment to access the resource."
                })),
            )
        }
        Some(payment) => {
            // Step 3: verify payment
            let payment_result = x402::verify_payment(&cfg, payment).await;

            if !payment_result.valid {
                return (
                    StatusCode::PAYMENT_REQUIRED,
                    Json(serde_json::json!({
                        "error": "Payment verification failed",
                        "proof_status": "valid",
                        "payment_error": payment_result.error,
                        "hint": "Your proof is valid but the payment could not be verified."
                    })),
                );
            }

            // Step 4: gated content
            let resource = ProtectedResource {
                id: Uuid::new_v4().to_string(),
                content: "ZK-gated resource unlocked. The agent proved knowledge of a secret credential and settled a micropayment on Stellar. No identity, no credential contents — only cryptographic attestation and settlement.".to_string(),
                verified_by: "proofpay".to_string(),
                payment_status: format!(
                    "settled via x402 (tx: {})",
                    payment_result.tx_hash.unwrap_or_default()
                ),
            };

            (StatusCode::OK, Json(serde_json::to_value(resource).unwrap()))
        }
    }
}

/// Return the x402 payment requirements and agent-facing instructions.
async fn payment_info(State(cfg): State<Arc<Config>>) -> impl IntoResponse {
    let payment_req = x402::payment_required_response(&cfg);

    Json(serde_json::json!({
        "x402": payment_req,
        "instructions": {
            "1": "Generate a commitment: SHA-256 hash of your secret credential",
            "2": "Sign the commitment with an Ed25519 key the agent controls",
            "3": "POST /verify to confirm the proof is accepted",
            "4": "Build an x402 payment on Stellar and include it as X-Payment header",
            "5": "POST /access with the proof and the X-Payment header"
        },
        "network": cfg.network,
        "facilitator": cfg.facilitator_url,
        "mock_mode": cfg.mock_payments
    }))
}
