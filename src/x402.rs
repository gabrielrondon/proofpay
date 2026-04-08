use serde::{Deserialize, Serialize};

/// x402 payment verification via Stellar facilitator.
///
/// The x402 protocol uses HTTP 402 Payment Required to gate access.
/// A facilitator verifies and settles payments on-chain.

const FACILITATOR_URL: &str = "https://channels.openzeppelin.com/x402/testnet";

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

/// Build a 402 Payment Required response with x402 headers.
pub fn payment_required_response(description: &str, amount: &str, recipient: &str) -> PaymentRequirement {
    PaymentRequirement {
        network: "stellar:testnet".to_string(),
        amount: amount.to_string(),
        asset: "native".to_string(), // XLM
        recipient: recipient.to_string(),
        description: description.to_string(),
    }
}

/// Verify an x402 payment header against the facilitator.
pub async fn verify_payment(payment_header: &str) -> PaymentVerification {
    let client = reqwest::Client::new();

    match client
        .post(format!("{}/verify", FACILITATOR_URL))
        .json(&serde_json::json!({
            "payment": payment_header,
        }))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => PaymentVerification {
                        valid: body.get("valid").and_then(|v| v.as_bool()).unwrap_or(false),
                        tx_hash: body.get("txHash").and_then(|v| v.as_str()).map(String::from),
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

/// Settle an x402 payment via the facilitator.
pub async fn settle_payment(payment_header: &str) -> PaymentVerification {
    let client = reqwest::Client::new();

    match client
        .post(format!("{}/settle", FACILITATOR_URL))
        .json(&serde_json::json!({
            "payment": payment_header,
        }))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => PaymentVerification {
                        valid: true,
                        tx_hash: body.get("txHash").and_then(|v| v.as_str()).map(String::from),
                        error: None,
                    },
                    Err(e) => PaymentVerification {
                        valid: false,
                        tx_hash: None,
                        error: Some(format!("Failed to parse settle response: {e}")),
                    },
                }
            } else {
                PaymentVerification {
                    valid: false,
                    tx_hash: None,
                    error: Some(format!("Settle returned {}", resp.status())),
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
