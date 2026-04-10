# ProofPay

**ZK-gated x402 access for autonomous agents on Stellar.**

Autonomous agents increasingly need to call paid APIs on behalf of humans: trading bots, research agents, data pipelines, procurement assistants. Two problems show up together:

1. **Who is allowed to call this?** Some resources are gated on eligibility — age, jurisdiction, a paid tier, a whitelist, a license. The agent needs to prove it qualifies.
2. **How does it pay per request?** HTTP was built with a `402 Payment Required` status and no protocol behind it. x402 finally fills that in.

ProofPay combines both: the agent proves knowledge of a secret credential with a zero-knowledge commitment proof, then settles a micropayment on Stellar via x402. The server verifies both and returns the gated content. It never sees who the agent is, what the credential contains, or which human is behind it.

## Flow

```
Agent                           ProofPay Server                  x402 Facilitator
  |                                   |                                  |
  |  1. POST /verify (ZK proof)       |                                  |
  |---------------------------------->|                                  |
  |         proof valid               |                                  |
  |<----------------------------------|                                  |
  |                                   |                                  |
  |  2. POST /access (no payment)     |                                  |
  |---------------------------------->|                                  |
  |         402 Payment Required      |                                  |
  |         + x402 requirements       |                                  |
  |<----------------------------------|                                  |
  |                                   |                                  |
  |  3. POST /access (X-Payment hdr)  |                                  |
  |---------------------------------->|--  verify payment  ------------->|
  |                                   |<-- settled on Stellar -----------|
  |         gated resource            |                                  |
  |<----------------------------------|                                  |
```

## What the ZK proof actually proves

ProofPay uses an Ed25519 commitment proof:

1. The agent has a secret credential — anything it should not reveal: a license token, a jurisdiction attestation, a paid-tier receipt, a whitelist entry.
2. The agent publishes `commitment = SHA-256(credential)`.
3. The agent signs `commitment` with an Ed25519 key it controls.
4. ProofPay verifies the signature against the commitment.

The server learns three things: a hash, a public key, and a signature. It never sees the credential. Two agents with the same credential can prove it independently without ever linking their identities. The public key doubles as a stable pseudonymous identifier when the agent wants one.

This is not a SNARK. It is the minimum viable ZK primitive for "prove you know a pre-image of this hash" — deliberately simple, with no trusted setup and no exotic dependencies. It runs in any language that has SHA-256 and Ed25519.

## Why agents care

- **Trading agents** prove operating a whitelisted wallet before hitting premium market-data APIs, without broadcasting the wallet.
- **Research agents** prove institutional access before paying for paper retrieval, without revealing which institution.
- **Compliance-aware agents** prove jurisdiction eligibility before calling a regulated endpoint, without doxxing their principal.
- **Pseudonymous subscriptions** let an agent prove it has paid for a tier without binding that tier to a public identity.

Every interaction is one HTTP request + one on-chain settlement. No accounts, no API keys, no session state.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET /` | Service info and flow description |
| `GET /health` | Health check |
| `GET /payment-info` | x402 payment requirements and agent instructions |
| `POST /verify` | Verify a ZK commitment proof (free) |
| `POST /access` | Access the gated resource (requires valid proof + x402 payment) |

## Quick start

```bash
cp .env.example .env
# set X402_MOCK=true to run the demo without a live facilitator
cargo run
```

In another terminal, run the bundled agent — it walks through the full flow end to end:

```bash
cargo run --example agent
```

Expected output:

```
[agent] step 1 → POST /verify
[agent]   status       : 200 OK
[agent] step 2 → POST /access (no payment, expecting 402)
[agent]   status       : 402 Payment Required
[agent] step 3 → building X-Payment header
[agent] step 4 → POST /access with X-Payment
[agent]   status       : 200 OK
[agent] done — gated resource unlocked without revealing the credential.
```

## Configuration

All configuration is environment-driven. `.env.example` documents every knob.

| Variable | Default | Purpose |
|----------|---------|---------|
| `PROOFPAY_BIND` | `0.0.0.0:3402` | HTTP bind address |
| `STELLAR_NETWORK` | `stellar:testnet` | Network advertised in the 402 response |
| `STELLAR_RECIPIENT` | testnet address | Account that receives access micropayments |
| `PAYMENT_AMOUNT_STROOPS` | `1000000` | Price per access in stroops (1 XLM = 10,000,000 stroops) |
| `PAYMENT_ASSET` | `native` | Asset code (`native` for XLM) |
| `ACCESS_DESCRIPTION` | — | Human-readable description in the 402 response |
| `X402_FACILITATOR_URL` | OpenZeppelin Channels testnet | x402 facilitator that verifies settlement |
| `X402_MOCK` | `false` | When `true`, any non-empty `X-Payment` header is accepted — useful for local demos |

## Architecture

```
proofpay/
├── src/
│   ├── lib.rs       library entry point (re-exports the modules below)
│   ├── main.rs      binary entry point — loads .env, starts axum
│   ├── config.rs    environment-driven runtime configuration
│   ├── server.rs    axum router and request handlers
│   ├── proof.rs     Ed25519 commitment proof verification (SHA-256 + signature)
│   └── x402.rs      x402 payment verification (facilitator or mock)
├── examples/
│   └── agent.rs     end-to-end agent client using the public library API
├── Dockerfile       multi-stage release build
├── fly.toml         Fly.io deployment config
├── .env.example     configuration template
├── Cargo.toml
└── README.md
```

## Deployment

The repository ships a multi-stage `Dockerfile` and a `fly.toml` targeting `gru` (São Paulo). Any container host works:

```bash
# Fly.io
fly launch --no-deploy      # first time only
fly deploy

# or any other container host
docker build -t proofpay .
docker run --rm -p 3402:3402 --env-file .env proofpay
```

Once deployed, point the example agent at the live URL:

```bash
PROOFPAY_URL=https://proofpay.fly.dev cargo run --example agent
```

## Tests

```bash
cargo test
```

Covers Ed25519 commitment verification for both valid and tampered signatures.

## Built With

- Rust — axum, tokio, ed25519-dalek, sha2, reqwest, dotenvy
- Stellar testnet via the x402 protocol
- Pluggable x402 facilitator (default: OpenZeppelin Channels on Stellar testnet)

## License

MIT.

## Hackathon

Built for [Stellar Hacks: Agents](https://dorahacks.io/hackathon/stellar-agents-x402-stripe-mpp/detail) — April 2026.
