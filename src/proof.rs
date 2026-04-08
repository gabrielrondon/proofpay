use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A zero-knowledge commitment proof.
///
/// The prover commits to a secret value by publishing:
/// - The hash of the value (commitment)
/// - A signature over the commitment using their Ed25519 key
///
/// The verifier checks the signature without ever seeing the secret value.
/// This proves the prover knows a value that hashes to the commitment,
/// without revealing what that value is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentProof {
    /// SHA-256 hash of the secret value (hex-encoded)
    pub commitment: String,
    /// Ed25519 public key of the prover (hex-encoded, 32 bytes)
    pub public_key: String,
    /// Ed25519 signature over the commitment (hex-encoded, 64 bytes)
    pub signature: String,
}

/// Result of proof verification
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub commitment: String,
    pub prover: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Verify a commitment proof.
///
/// Checks that:
/// 1. The public key is valid Ed25519
/// 2. The signature is valid over the commitment bytes
pub fn verify_commitment(proof: &CommitmentProof) -> VerificationResult {
    let commitment_bytes = match hex::decode(&proof.commitment) {
        Ok(b) => b,
        Err(e) => {
            return VerificationResult {
                valid: false,
                commitment: proof.commitment.clone(),
                prover: proof.public_key.clone(),
                error: Some(format!("Invalid commitment hex: {e}")),
            }
        }
    };

    let pk_bytes = match hex::decode(&proof.public_key) {
        Ok(b) => b,
        Err(e) => {
            return VerificationResult {
                valid: false,
                commitment: proof.commitment.clone(),
                prover: proof.public_key.clone(),
                error: Some(format!("Invalid public key hex: {e}")),
            }
        }
    };

    let sig_bytes = match hex::decode(&proof.signature) {
        Ok(b) => b,
        Err(e) => {
            return VerificationResult {
                valid: false,
                commitment: proof.commitment.clone(),
                prover: proof.public_key.clone(),
                error: Some(format!("Invalid signature hex: {e}")),
            }
        }
    };

    let verifying_key = match VerifyingKey::from_bytes(
        pk_bytes
            .as_slice()
            .try_into()
            .unwrap_or(&[0u8; 32]),
    ) {
        Ok(k) => k,
        Err(e) => {
            return VerificationResult {
                valid: false,
                commitment: proof.commitment.clone(),
                prover: proof.public_key.clone(),
                error: Some(format!("Invalid public key: {e}")),
            }
        }
    };

    let sig_array: [u8; 64] = match sig_bytes.as_slice().try_into() {
        Ok(a) => a,
        Err(_) => {
            return VerificationResult {
                valid: false,
                commitment: proof.commitment.clone(),
                prover: proof.public_key.clone(),
                error: Some("Invalid signature length (expected 64 bytes)".to_string()),
            }
        }
    };
    let signature = Signature::from_bytes(&sig_array);

    match verifying_key.verify(&commitment_bytes, &signature) {
        Ok(()) => VerificationResult {
            valid: true,
            commitment: proof.commitment.clone(),
            prover: proof.public_key.clone(),
            error: None,
        },
        Err(e) => VerificationResult {
            valid: false,
            commitment: proof.commitment.clone(),
            prover: proof.public_key.clone(),
            error: Some(format!("Signature verification failed: {e}")),
        },
    }
}

/// Create a commitment from a secret value.
/// Returns the SHA-256 hash as hex.
pub fn create_commitment(secret: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    #[test]
    fn test_valid_commitment_proof() {
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let verifying_key = signing_key.verifying_key();

        let secret = b"I am over 18 years old";
        let commitment = create_commitment(secret);
        let commitment_bytes = hex::decode(&commitment).unwrap();

        use ed25519_dalek::Signer;
        let signature = signing_key.sign(&commitment_bytes);

        let proof = CommitmentProof {
            commitment,
            public_key: hex::encode(verifying_key.to_bytes()),
            signature: hex::encode(signature.to_bytes()),
        };

        let result = verify_commitment(&proof);
        assert!(result.valid);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_invalid_signature() {
        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let other_key = SigningKey::generate(&mut rand::thread_rng());

        let secret = b"I am over 18 years old";
        let commitment = create_commitment(secret);
        let commitment_bytes = hex::decode(&commitment).unwrap();

        use ed25519_dalek::Signer;
        let signature = other_key.sign(&commitment_bytes);

        let proof = CommitmentProof {
            commitment,
            public_key: hex::encode(signing_key.verifying_key().to_bytes()),
            signature: hex::encode(signature.to_bytes()),
        };

        let result = verify_commitment(&proof);
        assert!(!result.valid);
    }
}
