//! Algorithm 1 & 2: Elastic Initiation Proposal (eIP) implementation

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{H256, sr25519, Pair};
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_std::vec::Vec;

use crate::{AuthorityId, AuthoritySignature};

/// The elastic Initiation Proposal message (Algorithm 1)
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct ElasticInitiationProposal {
    /// The epoch number this proposal is for
    pub epoch: u64,
    
    /// Random seed for schedule generation (R_e in the paper)
    pub random_seed: [u8; 32],
    
    /// The leader node ID (first node in validator set for the epoch)
    pub leader_id: AuthorityId,
    
    /// Previous epoch's final block hash (for chain continuity)
    pub parent_hash: H256,
    
    /// Timestamp of proposal creation
    pub timestamp: u64,
    
    /// List of active validators for this epoch
    pub validators: Vec<AuthorityId>,
    
    /// Digital signature from the leader
    pub signature: AuthoritySignature,
}

impl ElasticInitiationProposal {
    /// Create a new eIP (Algorithm 1, Step 2-3: Generate and broadcast)
    pub fn new(
        epoch: u64,
        validators: Vec<AuthorityId>,
        parent_hash: H256,
        timestamp: u64,
    ) -> Self {
        // Leader is the first validator in the sorted list for this epoch
        let leader_id = validators[0].clone();
        
        // Generate random seed R_e using epoch and parent hash
        let mut seed_data = Vec::new();
        seed_data.extend_from_slice(&epoch.to_le_bytes());
        seed_data.extend_from_slice(parent_hash.as_bytes());
        seed_data.extend_from_slice(&timestamp.to_le_bytes());
        
        let random_hash = BlakeTwo256::hash(&seed_data);
        let mut random_seed = [0u8; 32];
        random_seed.copy_from_slice(random_hash.as_bytes());
        
        Self {
            epoch,
            random_seed,
            leader_id,
            parent_hash,
            timestamp,
            validators,
            signature: AuthoritySignature::from_raw([0u8; 64]), // Will be signed later
        }
    }
    
    /// Get the message bytes to sign
    pub fn signable_message(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.epoch.to_le_bytes());
        data.extend_from_slice(&self.random_seed);
        data.extend_from_slice(self.parent_hash.as_bytes());
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        
        // Include validator set hash
        for validator in &self.validators {
            data.extend_from_slice(validator.as_ref());
        }
        
        data
    }
    
    /// Sign the eIP (called by leader)
    pub fn sign(&mut self, key_pair: &sr25519::Pair) {
        let message = self.signable_message();
        self.signature = key_pair.sign(&message);
    }
    
    /// Algorithm 2: Verify the received eIP message
    pub fn verify(&self) -> Result<(), VerificationError> {
        // Step 1: Check epoch number is valid (should be current + 1)
        // This check would be done by the calling context
        
        // Step 2: Verify the leader is correct (first validator)
        if self.validators.is_empty() {
            return Err(VerificationError::NoValidators);
        }
        
        if self.leader_id != self.validators[0] {
            return Err(VerificationError::InvalidLeader);
        }
        
        // Step 3: Verify signature
        let message = self.signable_message();
        if !sp_core::sr25519::Pair::verify(&self.signature, &message, &self.leader_id) {
            return Err(VerificationError::InvalidSignature);
        }
        
        // Step 4: Check timestamp is reasonable (not too old or future)
        // This would need current time from context
        
        // Step 5: Verify random seed is deterministic from inputs
        let mut expected_seed_data = Vec::new();
        expected_seed_data.extend_from_slice(&self.epoch.to_le_bytes());
        expected_seed_data.extend_from_slice(self.parent_hash.as_bytes());
        expected_seed_data.extend_from_slice(&self.timestamp.to_le_bytes());
        
        let expected_hash = BlakeTwo256::hash(&expected_seed_data);
        let mut expected_seed = [0u8; 32];
        expected_seed.copy_from_slice(expected_hash.as_bytes());
        
        if self.random_seed != expected_seed {
            return Err(VerificationError::InvalidRandomSeed);
        }
        
        Ok(())
    }
    
    /// Check if this node is the leader for the epoch
    pub fn is_leader(&self, my_authority: &AuthorityId) -> bool {
        self.leader_id == *my_authority
    }
}

/// Errors that can occur during eIP verification
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub enum VerificationError {
    /// No validators in the list
    NoValidators,
    /// Leader doesn't match expected (first validator)
    InvalidLeader,
    /// Signature verification failed
    InvalidSignature,
    /// Random seed doesn't match expected deterministic value
    InvalidRandomSeed,
    /// Timestamp is unreasonable
    InvalidTimestamp,
    /// Epoch number is wrong
    InvalidEpoch,
}

/// Response to an eIP broadcast (for consensus gathering)
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct ElasticInitiationResponse {
    /// The epoch this response is for
    pub epoch: u64,
    
    /// Hash of the eIP being acknowledged
    pub eip_hash: H256,
    
    /// The responding validator
    pub validator: AuthorityId,
    
    /// Accept or reject
    pub accepted: bool,
    
    /// Signature from the validator
    pub signature: AuthoritySignature,
}

impl ElasticInitiationResponse {
    /// Create a response to an eIP
    pub fn new(eip: &ElasticInitiationProposal, validator: AuthorityId, accepted: bool) -> Self {
        let eip_hash = BlakeTwo256::hash(&eip.encode());
        
        Self {
            epoch: eip.epoch,
            eip_hash,
            validator,
            accepted,
            signature: AuthoritySignature::from_raw([0u8; 64]), // Will be signed
        }
    }
    
    /// Sign the response
    pub fn sign(&mut self, key_pair: &sr25519::Pair) {
        let mut message = Vec::new();
        message.extend_from_slice(&self.epoch.to_le_bytes());
        message.extend_from_slice(self.eip_hash.as_bytes());
        message.push(if self.accepted { 1 } else { 0 });
        
        self.signature = key_pair.sign(&message);
    }
    
    /// Verify the response signature
    pub fn verify(&self) -> bool {
        let mut message = Vec::new();
        message.extend_from_slice(&self.epoch.to_le_bytes());
        message.extend_from_slice(self.eip_hash.as_bytes());
        message.push(if self.accepted { 1 } else { 0 });
        
        sp_core::sr25519::Pair::verify(&self.signature, &message, &self.validator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::Pair;
    
    #[test]
    fn test_eip_creation_and_verification() {
        // Create test validators
        let key_pair = sr25519::Pair::from_seed(&[1u8; 32]);
        let leader_id = key_pair.public();
        
        let validators = vec![leader_id.clone()];
        let parent_hash = H256::from([0u8; 32]);
        
        // Create eIP
        let mut eip = ElasticInitiationProposal::new(
            1,
            validators,
            parent_hash,
            1000,
        );
        
        // Sign it
        eip.sign(&key_pair);
        
        // Verify it
        assert!(eip.verify().is_ok());
    }
}