use axum::{
    Router,
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::proof::{self, CommitmentProof, VerificationResult};
use crate::x402;

/// Access token issued after successful proof verification + payment
#[derive(Debug, Serialize)]
struct AccessToken {
    token: String,
    resource: String,
    expires_in: u64,
}

/// Request to verify a proof and get a payment requirement
#[derive(Debug, Deserialize)]
struct VerifyRequest {
    proof: CommitmentProof,
}

/// Request to access a gated resource with proof + payment
#[derive(Debug, Deserialize)]
struct AccessRequest {
    proof: CommitmentProof,
}

/// Protected resource content
#[derive(Debug, Serialize)]
struct ProtectedResource {
    id: String,
    content: String,
    verified_by: String,
    payment_status: String,
}

pub fn create_router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/verify", post(verify_proof))
        .route("/access", post(access_resource))
        .route("/payment-info", get(payment_info))
        .layer(CorsLayer::permissive())
}

async fn index() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "ProofPay",
        "description": "ZK-gated payments on Stellar — prove access rights with zero-knowledge proofs, pay via x402",
        "version": "0.1.0",
        "endpoints": {
            "POST /verify": "Verify a ZK commitment proof",
            "POST /access": "Access a protected resource (requires valid proof + x402 payment)",
            "GET /payment-info": "Get x402 payment requirements",
            "GET /health": "Health check"
        },
        "flow": [
            "1. Generate a commitment proof (hash your secret, sign with Ed25519)",
            "2. POST /verify to check your proof is valid",
            "3. POST /access with your proof + X-Payment header (x402) to access the resource",
            "4. If proof is valid and payment is verified, you get the protected content"
        ]
    }))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "proofpay",
        "network": "stellar:testnet"
    }))
}

/// Verify a ZK commitment proof without requiring payment.
/// Use this to check if your proof is valid before paying.
async fn verify_proof(Json(req): Json<VerifyRequest>) -> impl IntoResponse {
    let result = proof::verify_commitment(&req.proof);

    if result.valid {
        (StatusCode::OK, Json(serde_json::to_value(result).unwrap()))
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::to_value(result).unwrap()),
        )
    }
}

/// Access a protected resource.
///
/// Flow:
/// 1. Verify the ZK proof
/// 2. Check for x402 payment header
/// 3. If no payment header → return 402 with payment requirements
/// 4. If payment header present → verify payment via facilitator
/// 5. If both valid → return protected content
async fn access_resource(
    headers: HeaderMap,
    Json(req): Json<AccessRequest>,
) -> impl IntoResponse {
    // Step 1: Verify ZK proof
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

    // Step 2: Check for x402 payment header
    let payment_header = headers
        .get("X-Payment")
        .and_then(|v| v.to_str().ok());

    match payment_header {
        None => {
            // Return 402 Payment Required with x402 payment info
            let payment_req = x402::payment_required_response(
                "Access to ZK-verified resource",
                "1000000", // 0.1 XLM in stroops
                "GDQOE23CFSUMSVQK4Y5JHPPYK73VYCNHZHA7ENKCV37P6SUEO6XQBKPP", // testnet recipient
            );
            (
                StatusCode::PAYMENT_REQUIRED,
                Json(serde_json::json!({
                    "error": "Payment required",
                    "proof_status": "valid",
                    "x402": payment_req,
                    "hint": "Your proof is valid. Now include an X-Payment header with a signed x402 payment to access the resource."
                })),
            )
        }
        Some(payment) => {
            // Step 3: Verify payment via facilitator
            let payment_result = x402::verify_payment(payment).await;

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

            // Step 4: Both valid — return protected content
            let resource = ProtectedResource {
                id: Uuid::new_v4().to_string(),
                content: "This is ZK-gated content. You proved knowledge of a secret and paid via x402 on Stellar. No identity revealed, no data exposed — just math and micropayments.".to_string(),
                verified_by: "proofpay".to_string(),
                payment_status: format!("settled via x402 (tx: {})", payment_result.tx_hash.unwrap_or_default()),
            };

            (StatusCode::OK, Json(serde_json::to_value(resource).unwrap()))
        }
    }
}

/// Get x402 payment requirements for accessing protected resources.
async fn payment_info() -> impl IntoResponse {
    let payment_req = x402::payment_required_response(
        "Access to ZK-verified resource",
        "1000000",
        "GDQOE23CFSUMSVQK4Y5JHPPYK73VYCNHZHA7ENKCV37P6SUEO6XQBKPP",
    );

    Json(serde_json::json!({
        "x402": payment_req,
        "instructions": {
            "1": "Generate a commitment proof: SHA-256 hash your secret, sign with Ed25519",
            "2": "Verify your proof: POST /verify with your proof",
            "3": "Get a signed x402 payment from your Stellar wallet",
            "4": "Access the resource: POST /access with proof + X-Payment header"
        },
        "network": "stellar:testnet",
        "facilitator": "OpenZeppelin Channels (testnet)"
    }))
}
