# ProofPay

**ZK-gated payments on Stellar** — prove access rights with zero-knowledge proofs, pay via x402.

ProofPay combines zero-knowledge commitment proofs with the x402 payment protocol on Stellar, enabling privacy-preserving access to paid resources. Prove you have the right to access something without revealing who you are or what you know — then pay with a Stellar micropayment.

## How it works

```
Client                          ProofPay Server                    Stellar (x402)
  |                                   |                                  |
  |  1. POST /verify (ZK proof)       |                                  |
  |---------------------------------->|                                  |
  |  proof valid ✓                    |                                  |
  |<----------------------------------|                                  |
  |                                   |                                  |
  |  2. POST /access (proof + X-Payment header)                         |
  |---------------------------------->|                                  |
  |         verify proof ✓            |                                  |
  |         verify payment ---------->|-- verify via facilitator ------->|
  |                                   |<-- payment valid ✓ --------------|
  |  protected content returned       |                                  |
  |<----------------------------------|                                  |
```

## The ZK Part

ProofPay uses **Ed25519 commitment proofs**:

1. **You have a secret** (e.g., "I am over 18", "I hold credential X", "I know the passphrase")
2. **You commit**: SHA-256 hash of the secret → commitment
3. **You sign**: Ed25519 signature over the commitment
4. **ProofPay verifies**: the signature is valid for the commitment, without ever seeing the secret

This is a zero-knowledge proof of knowledge: you prove you know a value that hashes to the commitment, without revealing the value itself.

## The Payment Part

ProofPay uses the **x402 protocol** on Stellar:

- Resources require micropayment (configurable amount in XLM)
- Payment is verified via an x402 facilitator (OpenZeppelin Channels on Stellar testnet)
- No accounts, no subscriptions — pay per access with cryptographic proof of payment

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET /` | | Service info and flow description |
| `GET /health` | | Health check |
| `GET /payment-info` | | Get x402 payment requirements |
| `POST /verify` | | Verify a ZK proof (free, no payment needed) |
| `POST /access` | | Access protected resource (requires valid proof + x402 payment) |

## Quick Start

```bash
# Run the server
cargo run

# In another terminal — verify a proof
curl -X POST http://localhost:3402/verify \
  -H "Content-Type: application/json" \
  -d '{
    "proof": {
      "commitment": "<sha256-hex>",
      "public_key": "<ed25519-pubkey-hex>",
      "signature": "<signature-hex>"
    }
  }'
```

## Generate a Test Proof

```bash
cargo test -- --nocapture
```

The tests generate valid proofs you can use as examples.

## Architecture

```
proofpay/
├── src/
│   ├── main.rs      # Entry point
│   ├── server.rs    # Axum HTTP server with routes
│   ├── proof.rs     # ZK commitment proof verification (Ed25519 + SHA-256)
│   └── x402.rs      # x402 payment verification via Stellar facilitator
├── Cargo.toml
└── README.md
```

## Built With

- **Rust** (axum, ed25519-dalek, sha2)
- **Stellar** testnet via x402 protocol
- **OpenZeppelin Channels** as x402 facilitator

## Use Cases

- **Age verification**: Prove you're over 18 without showing ID
- **Credential access**: Prove you hold a qualification without revealing it
- **Gated APIs**: Pay-per-query with privacy — no API keys, no accounts
- **Anonymous voting eligibility**: Prove you're eligible without revealing identity

## License

MIT

## Built for Stellar Hacks: Agents (April 2026)
