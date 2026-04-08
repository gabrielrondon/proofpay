mod proof;
mod server;
mod x402;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let addr = "0.0.0.0:3402";
    tracing::info!("ProofPay starting on {}", addr);

    let app = server::create_router();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("ProofPay ready — ZK-gated payments on Stellar");
    axum::serve(listener, app).await.unwrap();
}
