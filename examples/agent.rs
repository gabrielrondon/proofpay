//! Example autonomous agent using ProofPay.
//!
//! Walks through the full ZK-gated x402 flow end to end:
//!   1. Generate an Ed25519 identity
//!   2. Commit to a secret credential
//!   3. POST /verify to check the commitment proof
//!   4. POST /access without payment (expects HTTP 402)
//!   5. Build an x402 payment header
//!   6. POST /access with the X-Payment header and fetch the gated resource
//!
//! Run:
//!   Terminal A:  X402_MOCK=true cargo run
//!   Terminal B:  cargo run --example agent
//!
//! Override the target with `PROOFPAY_URL`, e.g. PROOFPAY_URL=https://proofpay.fly.dev.

use std::error::Error;

use base64::{Engine, engine::general_purpose};
use ed25519_dalek::{Signer, SigningKey};
use proofpay::proof::{CommitmentProof, create_commitment};
use serde_json::{Value, json};

const DEFAULT_URL: &str = "http://localhost:3402";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base = std::env::var("PROOFPAY_URL").unwrap_or_else(|_| DEFAULT_URL.into());
    let client = reqwest::Client::new();

    println!("[agent] target         : {base}");
    println!("[agent] bootstrap      : generating Ed25519 identity");
    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    let public_key_hex = hex::encode(verifying_key.to_bytes());
    println!("[agent] public key     : {public_key_hex}");

    // The "secret credential" — anything the agent knows but will not reveal.
    // Could be a license token, a subscription receipt, a whitelist entry, etc.
    let secret = b"agent:trading-bot-v1:license=eu-mifid-tier2";
    println!("[agent] credential     : <hidden {} bytes>", secret.len());

    let commitment = create_commitment(secret);
    let commitment_bytes = hex::decode(&commitment)?;
    let signature = signing_key.sign(&commitment_bytes);

    let proof = CommitmentProof {
        commitment: commitment.clone(),
        public_key: public_key_hex.clone(),
        signature: hex::encode(signature.to_bytes()),
    };
    println!("[agent] commitment     : {commitment}");

    // 1. Verify the proof (no payment required).
    println!("\n[agent] step 1 → POST /verify");
    let verify_resp = client
        .post(format!("{base}/verify"))
        .json(&json!({ "proof": proof }))
        .send()
        .await?;
    let status = verify_resp.status();
    let body: Value = verify_resp.json().await?;
    println!("[agent]   status       : {status}");
    println!("[agent]   body         : {body}");
    if !status.is_success() {
        return Err("proof rejected".into());
    }

    // 2. Request the gated resource without payment — expect 402.
    println!("\n[agent] step 2 → POST /access (no payment, expecting 402)");
    let unpaid = client
        .post(format!("{base}/access"))
        .json(&json!({ "proof": proof }))
        .send()
        .await?;
    let unpaid_status = unpaid.status();
    let unpaid_body: Value = unpaid.json().await?;
    println!("[agent]   status       : {unpaid_status}");
    if let Some(x402) = unpaid_body.get("x402") {
        println!("[agent]   x402         : {x402}");
    }

    // 3. Build an x402 payment header.
    //
    // In a real deployment this is a signed Stellar payment envelope that the
    // facilitator can verify. The example encodes a minimal JSON payload so the
    // flow is runnable locally without a live Stellar account. Set X402_MOCK=true
    // on the server to accept this header.
    let x402_payload = json!({
        "scheme": "x402",
        "network": unpaid_body
            .get("x402")
            .and_then(|v| v.get("network"))
            .and_then(|v| v.as_str())
            .unwrap_or("stellar:testnet"),
        "asset": unpaid_body
            .get("x402")
            .and_then(|v| v.get("asset"))
            .and_then(|v| v.as_str())
            .unwrap_or("native"),
        "amount": unpaid_body
            .get("x402")
            .and_then(|v| v.get("amount"))
            .and_then(|v| v.as_str())
            .unwrap_or("1000000"),
        "recipient": unpaid_body
            .get("x402")
            .and_then(|v| v.get("recipient"))
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "payer": public_key_hex,
        "nonce": uuid::Uuid::new_v4().to_string(),
    });
    let payment_header = general_purpose::STANDARD.encode(x402_payload.to_string());
    println!("\n[agent] step 3 → building X-Payment header ({} bytes)", payment_header.len());

    // 4. Retry /access with the payment header.
    println!("\n[agent] step 4 → POST /access with X-Payment");
    let paid = client
        .post(format!("{base}/access"))
        .header("X-Payment", payment_header)
        .json(&json!({ "proof": proof }))
        .send()
        .await?;
    let paid_status = paid.status();
    let paid_body: Value = paid.json().await?;
    println!("[agent]   status       : {paid_status}");
    println!("[agent]   body         : {paid_body}");

    if paid_status.is_success() {
        println!("\n[agent] done — gated resource unlocked without revealing the credential.");
        Ok(())
    } else {
        Err(format!("access denied: {paid_body}").into())
    }
}
