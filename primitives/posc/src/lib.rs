//! Primitives for PoSc (Proof of Schedule) consensus.
//!
//! This module implements the deterministic scheduling consensus as described
//! in the Throughput Network whitepaper. Unlike probabilistic consensus (BABE/Aura),
//! PoSc uses predetermined schedules for block production.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod eip;
pub mod eabs;

pub use eip::{ElasticInitiationProposal, ElasticInitiationResponse, VerificationError};
pub use eabs::ElasticScheduleGenerator;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_application_crypto::sr25519;
use sp_consensus_slots::Slot;
use sp_runtime::{traits::Zero, ConsensusEngineId};
use sp_std::vec::Vec;

/// The `ConsensusEngineId` of PoSc.
pub const POSC_ENGINE_ID: ConsensusEngineId = *b"PoSc";

/// The index of an authority.
pub type AuthorityIndex = u32;

/// PoSc authority identifier.
pub type AuthorityId = sr25519::Public;

/// PoSc authority signature.
pub type AuthoritySignature = sr25519::Signature;

/// An equivocation proof for multiple block authorships on the same slot.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct EquivocationProof<Header> {
    /// The authority that produced the equivocation.
    pub offender: AuthorityId,
    /// The slot at which the equivocation happened.
    pub slot: Slot,
    /// The first header involved in the equivocation.
    pub first_header: Header,
    /// The second header involved in the equivocation.
    pub second_header: Header,
}

/// A scheduled block assignment for PoSc.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct ScheduledBlock {
    /// The slot number for this block.
    pub slot: Slot,
    /// The authority assigned to produce this block.
    pub authority: AuthorityId,
    /// The authority index.
    pub authority_index: AuthorityIndex,
}

/// The complete schedule for an epoch.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct EpochSchedule {
    /// The epoch number.
    pub epoch: u64,
    /// The starting slot of this epoch.
    pub start_slot: Slot,
    /// Number of slots in this epoch.
    pub duration: u64,
    /// The scheduled blocks for this epoch.
    pub schedule: Vec<ScheduledBlock>,
    /// Randomness seed used to generate this schedule.
    pub randomness: [u8; 32],
}

impl EpochSchedule {
    /// Get the authority assigned to a specific slot.
    pub fn get_authority_for_slot(&self, slot: Slot) -> Option<&AuthorityId> {
        // Check if slot is within this epoch
        if slot < self.start_slot {
            return None;
        }
        
        let slot_number: u64 = slot.into();
        let start_slot_number: u64 = self.start_slot.into();
        let slot_offset = slot_number.saturating_sub(start_slot_number);
        
        if slot_offset >= self.duration {
            return None;
        }
        
        // Find the scheduled block for this slot
        self.schedule
            .iter()
            .find(|sb| sb.slot == slot)
            .map(|sb| &sb.authority)
    }
    
    /// Check if an authority is scheduled for a specific slot.
    pub fn is_scheduled(&self, slot: Slot, authority: &AuthorityId) -> bool {
        self.get_authority_for_slot(slot)
            .map(|scheduled| scheduled == authority)
            .unwrap_or(false)
    }
}

/// Block Generation Score (BGS) for an authority.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct BlockGenerationScore {
    /// The authority.
    pub authority: AuthorityId,
    /// Security score (0-100).
    pub security_score: u64,
    /// Stake amount.
    pub stake: u128,
    /// Calculated BGS value.
    pub score: u64,
}

impl BlockGenerationScore {
    /// Calculate the BGS based on security score and stake.
    pub fn calculate(security_score: u64, stake: u128) -> u64 {
        // BGS = security_score * (stake / 1_000_000) to keep numbers manageable
        let normalized_stake = (stake / 1_000_000) as u64;
        security_score.saturating_mul(normalized_stake)
    }
}

/// Configuration for the eABS (elastic Advanced Block Schedule) algorithm.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct ScheduleConfig {
    /// Minimum blocks guaranteed per validator.
    pub min_blocks_per_validator: u32,
    /// Maximum consecutive blocks for a single validator.
    pub max_consecutive_blocks: u32,
    /// Whether to shuffle the final schedule.
    pub shuffle_schedule: bool,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            min_blocks_per_validator: 1,
            max_consecutive_blocks: 3,
            shuffle_schedule: true,
        }
    }
}

/// The elastic Advanced Block Schedule (eABS) algorithm implementation.
pub struct ScheduleGenerator;

impl ScheduleGenerator {
    /// Generate a deterministic schedule based on BGS scores.
    pub fn generate_schedule(
        scores: &[BlockGenerationScore],
        epoch_length: u64,
        seed: [u8; 32],
        config: &ScheduleConfig,
    ) -> Vec<AuthorityIndex> {
        if scores.is_empty() || epoch_length == 0 {
            return Vec::new();
        }
        
        let mut schedule = Vec::with_capacity(epoch_length as usize);
        
        // Step 1: Guarantee minimum blocks for each validator
        if config.min_blocks_per_validator > 0 {
            for (index, _) in scores.iter().enumerate() {
                for _ in 0..config.min_blocks_per_validator {
                    if schedule.len() < epoch_length as usize {
                        schedule.push(index as AuthorityIndex);
                    }
                }
            }
        }
        
        // Step 2: Distribute remaining blocks based on BGS scores
        let remaining_slots = epoch_length as usize - schedule.len();
        if remaining_slots > 0 {
            let total_score: u64 = scores.iter().map(|s| s.score).sum();
            
            if total_score > 0 {
                // Calculate proportional allocation
                for (index, score) in scores.iter().enumerate() {
                    let proportion = score.score as f64 / total_score as f64;
                    let additional_blocks = (remaining_slots as f64 * proportion) as usize;
                    
                    for _ in 0..additional_blocks {
                        if schedule.len() < epoch_length as usize {
                            schedule.push(index as AuthorityIndex);
                        }
                    }
                }
            }
            
            // Fill any remaining slots round-robin
            let mut index = 0;
            while schedule.len() < epoch_length as usize {
                schedule.push(index as AuthorityIndex);
                index = (index + 1) % scores.len();
            }
        }
        
        // Step 3: Apply consecutive block limits
        if config.max_consecutive_blocks > 0 {
            Self::enforce_consecutive_limits(&mut schedule, config.max_consecutive_blocks);
        }
        
        // Step 4: Shuffle if configured (using deterministic randomness from seed)
        if config.shuffle_schedule {
            Self::deterministic_shuffle(&mut schedule, seed);
        }
        
        schedule
    }
    
    /// Enforce maximum consecutive blocks constraint.
    fn enforce_consecutive_limits(schedule: &mut Vec<AuthorityIndex>, max_consecutive: u32) {
        let mut i = 0;
        while i < schedule.len() {
            let mut consecutive_count = 1;
            let current_validator = schedule[i];
            
            // Count consecutive blocks
            while i + consecutive_count < schedule.len()
                && schedule[i + consecutive_count] == current_validator
            {
                consecutive_count += 1;
            }
            
            // If exceeds limit, swap with a different validator
            if consecutive_count > max_consecutive as usize {
                // Find next different validator
                for j in (i + max_consecutive as usize)..schedule.len() {
                    if schedule[j] != current_validator {
                        schedule.swap(i + max_consecutive as usize, j);
                        break;
                    }
                }
            }
            
            i += consecutive_count.min(max_consecutive as usize);
        }
    }
    
    /// Deterministic shuffle using the seed.
    fn deterministic_shuffle(schedule: &mut Vec<AuthorityIndex>, seed: [u8; 32]) {
        // Simple deterministic shuffle using seed as source of randomness
        // This is a simplified version - in production use a proper PRNG
        let mut rng_state = u64::from_le_bytes(seed[0..8].try_into().unwrap_or_default());
        
        for i in (1..schedule.len()).rev() {
            // Linear congruential generator for deterministic randomness
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let j = (rng_state as usize) % (i + 1);
            schedule.swap(i, j);
        }
    }
}

/// PoSc digest item that is added to the block header.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub enum DigestItem {
    /// A pre-runtime digest for PoSc.
    PreRuntime(PreDigest),
    /// A seal signature for PoSc.
    Seal(AuthoritySignature),
}

/// Pre-runtime digest for PoSc.
#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
pub struct PreDigest {
    /// The slot number.
    pub slot: Slot,
    /// The authority index that created this block.
    pub authority_index: AuthorityIndex,
}

/// API necessary for PoSc.
sp_api::decl_runtime_apis! {
    /// API for PoSc consensus.
    pub trait PoScApi {
        /// Get the current epoch schedule.
        fn current_epoch_schedule() -> EpochSchedule;
        
        /// Get the next epoch schedule (if available).
        fn next_epoch_schedule() -> Option<EpochSchedule>;
        
        /// Check if an authority is scheduled for a slot.
        fn is_scheduled(slot: Slot, authority: AuthorityId) -> bool;
        
        /// Get the authorities for the current epoch.
        fn authorities() -> Vec<AuthorityId>;
    }
}