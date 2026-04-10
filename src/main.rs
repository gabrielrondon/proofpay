use std::sync::Arc;

use proofpay::{config::Config, server};

#[tokio::main]
async fn main() {
    // Load .env if present — silent if the file does not exist.
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt::init();

    let config = Arc::new(Config::from_env());
    tracing::info!(
        bind = %config.bind_addr,
        network = %config.network,
        mock = config.mock_payments,
        "ProofPay starting"
    );

    let app = server::create_router(config.clone());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("failed to bind address");
    tracing::info!("ProofPay ready — ZK-gated x402 access on Stellar");
    axum::serve(listener, app)
        .await
        .expect("server error");
}
