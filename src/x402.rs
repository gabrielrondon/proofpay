use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;

/// x402 payment verification.
///
/// The x402 protocol uses HTTP 402 Payment Required to gate access to a
/// resource: the server advertises payment requirements, the client settles
/// on-chain, and a facilitator verifies the settlement. ProofPay delegates
/// verification to a configurable facilitator endpoint so it can target any
/// Stellar-compatible x402 facilitator.

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentRequirement {
    pub network: String,
    pub amount: String,
    pub asset: String,
    pub recipient: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentVerification {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Build the payment requirements advertised in a 402 response.
pub fn payment_required_response(config: &Config) -> PaymentRequirement {
    PaymentRequirement {
        network: config.network.clone(),
        amount: config.amount_stroops.clone(),
        asset: config.asset.clone(),
        recipient: config.recipient.clone(),
        description: config.access_description.clone(),
    }
}

/// Verify an x402 payment header.
///
/// In mock mode (`X402_MOCK=true`), any non-empty header is accepted — useful
/// for local demos and for the bundled example agent. In non-mock mode the
/// header is forwarded to the configured facilitator for real settlement
/// verification.
pub async fn verify_payment(config: &Config, payment_header: &str) -> PaymentVerification {
    if payment_header.trim().is_empty() {
        return PaymentVerification {
            valid: false,
            tx_hash: None,
            error: Some("Empty X-Payment header".into()),
        };
    }

    if config.mock_payments {
        return PaymentVerification {
            valid: true,
            tx_hash: Some(format!("mock-{}", Uuid::new_v4())),
            error: None,
        };
    }

    let client = reqwest::Client::new();
    let url = format!("{}/verify", config.facilitator_url.trim_end_matches('/'));

    match client
        .post(&url)
        .json(&serde_json::json!({ "payment": payment_header }))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => PaymentVerification {
                        valid: body.get("valid").and_then(|v| v.as_bool()).unwrap_or(false),
                        tx_hash: body
                            .get("txHash")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        error: None,
                    },
                    Err(e) => PaymentVerification {
                        valid: false,
                        tx_hash: None,
                        error: Some(format!("Failed to parse facilitator response: {e}")),
                    },
                }
            } else {
                PaymentVerification {
                    valid: false,
                    tx_hash: None,
                    error: Some(format!("Facilitator returned {}", resp.status())),
                }
            }
        }
        Err(e) => PaymentVerification {
            valid: false,
            tx_hash: None,
            error: Some(format!("Failed to reach facilitator: {e}")),
        },
    }
}
