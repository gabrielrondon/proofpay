//! ProofPay — ZK-gated access for autonomous agents paying with x402 on Stellar.
//!
//! Agents prove knowledge of a secret credential (Ed25519 commitment proof) and
//! pay per request via the x402 protocol. The server never learns who the agent
//! is or what the credential contains — only that the agent holds it and has paid.

pub mod config;
pub mod proof;
pub mod server;
pub mod x402;
