use anyhow::Result;
use async_trait::async_trait;
use prism_common::{digest::Digest, transaction::Transaction};
use prism_keys::{Signature, SigningKey, VerifyingKey};
use prism_serde::{
    binary::ToBinary,
    hex::{FromHex, ToHex},
};
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use tokio::sync::broadcast;

pub mod celestia;
pub mod consts;
pub mod memory;

// FinalizedEpoch is the data structure that represents the finalized epoch data, and is posted to the DA layer.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FinalizedEpoch {
    pub height: u64,
    pub prev_commitment: Digest,
    pub current_commitment: Digest,
    pub proof: SP1ProofWithPublicValues,
    pub signature: Option<String>,
}

impl FinalizedEpoch {
    pub fn insert_signature(&mut self, key: &SigningKey) {
        let plaintext = self.encode_to_bytes().unwrap();
        let signature = key.sign(&plaintext);
        self.signature = Some(signature.to_bytes().to_hex());
    }

    pub fn verify_signature(&self, vk: VerifyingKey) -> Result<()> {
        let epoch_without_signature = FinalizedEpoch {
            height: self.height,
            prev_commitment: self.prev_commitment,
            current_commitment: self.current_commitment,
            proof: self.proof.clone(),
            signature: None,
        };

        let message = epoch_without_signature
            .encode_to_bytes()
            .map_err(|e| anyhow::anyhow!("Failed to serialize epoch: {}", e))?;

        let signature =
            self.signature.as_ref().ok_or_else(|| anyhow::anyhow!("No signature present"))?;

        let signature_bytes = Vec::<u8>::from_hex(signature)
            .map_err(|e| anyhow::anyhow!("Failed to decode signature: {}", e))?;

        let signature: Signature =
            Signature::from_algorithm_and_bytes(vk.algorithm(), signature_bytes.as_slice())
                .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;

        vk.verify_signature(&message, &signature)
            .map_err(|e| anyhow::anyhow!("Signature verification failed: {}", e))?;
        Ok(())
    }
}

#[async_trait]
pub trait DataAvailabilityLayer: Send + Sync {
    async fn get_latest_height(&self) -> Result<u64>;
    async fn initialize_sync_target(&self) -> Result<u64>;
    async fn get_finalized_epoch(&self, height: u64) -> Result<Option<FinalizedEpoch>>;
    async fn submit_finalized_epoch(&self, epoch: FinalizedEpoch) -> Result<u64>;
    async fn get_transactions(&self, height: u64) -> Result<Vec<Transaction>>;
    async fn submit_transactions(&self, transactions: Vec<Transaction>) -> Result<u64>;
    async fn start(&self) -> Result<()>;
    fn subscribe_to_heights(&self) -> broadcast::Receiver<u64>;
}
