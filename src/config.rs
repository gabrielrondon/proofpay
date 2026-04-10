use std::env;

/// Runtime configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub network: String,
    pub recipient: String,
    pub amount_stroops: String,
    pub asset: String,
    pub access_description: String,
    pub facilitator_url: String,
    pub mock_payments: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("PROOFPAY_BIND").unwrap_or_else(|_| "0.0.0.0:3402".into()),
            network: env::var("STELLAR_NETWORK").unwrap_or_else(|_| "stellar:testnet".into()),
            recipient: env::var("STELLAR_RECIPIENT").unwrap_or_else(|_| {
                "GDQOE23CFSUMSVQK4Y5JHPPYK73VYCNHZHA7ENKCV37P6SUEO6XQBKPP".into()
            }),
            amount_stroops: env::var("PAYMENT_AMOUNT_STROOPS")
                .unwrap_or_else(|_| "1000000".into()),
            asset: env::var("PAYMENT_ASSET").unwrap_or_else(|_| "native".into()),
            access_description: env::var("ACCESS_DESCRIPTION")
                .unwrap_or_else(|_| "Access to ZK-verified agent resource".into()),
            facilitator_url: env::var("X402_FACILITATOR_URL")
                .unwrap_or_else(|_| "https://channels.openzeppelin.com/x402/testnet".into()),
            mock_payments: env::var("X402_MOCK")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
                .unwrap_or(false),
        }
    }
}
