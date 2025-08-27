//! # PoSc Consensus Pallet
//!
//! This pallet implements the on-chain logic for PoSc (Proof of Schedule) consensus.
//! It manages epoch transitions, schedule generation, and authority validation.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::{Get, OneSessionHandler, Randomness},
    BoundedVec,
};
use frame_system::pallet_prelude::*;
use sp_consensus_posc::{
    AuthorityId, AuthorityIndex, BlockGenerationScore, EpochSchedule, 
    ScheduleConfig, ScheduleGenerator, ScheduledBlock,
};
use sp_consensus_slots::Slot;
use sp_runtime::{
    traits::{One, SaturatedConversion, Saturating, Zero},
    Permill,
};
use sp_staking::SessionIndex;
use sp_std::{vec, vec::Vec};

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        /// The runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Number of slots per epoch.
        #[pallet::constant]
        type EpochDuration: Get<u64>;
        
        /// Expected average block time.
        #[pallet::constant]
        type ExpectedBlockTime: Get<Self::Moment>;
        
        /// Source of randomness for schedule generation.
        type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;
        
        /// Minimum stake required to be a validator.
        #[pallet::constant]
        type MinimumStake: Get<u128>;
        
        /// Default security score for validators.
        #[pallet::constant]
        type DefaultSecurityScore: Get<u64>;
        
        /// Minimum security score to participate.
        #[pallet::constant]
        type MinimumSecurityScore: Get<u64>;
        
        /// Maximum authorities that can be registered.
        #[pallet::constant]
        type MaxAuthorities: Get<u32>;
    }

    /// Current epoch index.
    #[pallet::storage]
    pub type EpochIndex<T> = StorageValue<_, u64, ValueQuery>;
    
    /// Current slot number.
    #[pallet::storage]
    pub type CurrentSlot<T> = StorageValue<_, Slot, ValueQuery>;
    
    /// The current epoch schedule (simplified to store just validator list).
    #[pallet::storage]
    pub type CurrentSchedule<T: Config> = StorageValue<_, BoundedVec<AuthorityId, T::MaxAuthorities>, ValueQuery>;
    
    /// The next epoch schedule (simplified to store just validator list).
    #[pallet::storage]
    pub type NextSchedule<T: Config> = StorageValue<_, BoundedVec<AuthorityId, T::MaxAuthorities>, ValueQuery>;
    
    /// Current authorities.
    #[pallet::storage]
    pub type Authorities<T: Config> = StorageValue<
        _,
        BoundedVec<AuthorityId, T::MaxAuthorities>,
        ValueQuery,
    >;
    
    /// Next authorities (for next epoch).
    #[pallet::storage]
    pub type NextAuthorities<T: Config> = StorageValue<
        _,
        BoundedVec<AuthorityId, T::MaxAuthorities>,
        ValueQuery,
    >;
    
    /// BGS scores for current authorities.
    #[pallet::storage]
    pub type AuthorityScores<T> = StorageMap<
        _,
        Blake2_128Concat,
        AuthorityId,
        BlockGenerationScore,
        OptionQuery,
    >;
    
    /// Randomness for the current epoch.
    #[pallet::storage]
    pub type EpochRandomness<T> = StorageValue<_, [u8; 32], ValueQuery>;
    
    /// Genesis slot.
    #[pallet::storage]
    pub type GenesisSlot<T> = StorageValue<_, Slot, ValueQuery>;
    
    /// Information about the last epoch change.
    #[pallet::storage]
    pub type LastEpochChange<T> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New epoch has started.
        EpochChanged {
            epoch_index: u64,
            start_slot: Slot,
            authorities: u32,
        },
        
        /// Schedule generated for next epoch.
        ScheduleGenerated {
            epoch_index: u64,
            schedule_length: u32,
        },
        
        /// Authority BGS calculated.
        AuthorityScoreCalculated {
            authority: AuthorityId,
            score: u64,
            stake: u128,
        },
        
        /// Invalid schedule detected.
        InvalidSchedule {
            slot: Slot,
            expected: Option<AuthorityId>,
            actual: AuthorityId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// No authorities available.
        NoAuthorities,
        /// Schedule not found for current epoch.
        ScheduleNotFound,
        /// Authority not scheduled for this slot.
        NotScheduled,
        /// Invalid slot number.
        InvalidSlot,
        /// Too many authorities.
        TooManyAuthorities,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Update current slot
            let timestamp = pallet_timestamp::Pallet::<T>::get();
            let slot_duration = T::ExpectedBlockTime::get();
            
            if !slot_duration.is_zero() {
                let current_slot = Slot::from(
                    (timestamp / slot_duration).saturated_into::<u64>()
                );
                CurrentSlot::<T>::put(current_slot);
                
                // Check for epoch change
                if Self::should_epoch_change(current_slot) {
                    Self::change_epoch(now, current_slot);
                }
            }
            
            T::DbWeight::get().reads_writes(3, 2)
        }
        
        fn on_finalize(_now: BlockNumberFor<T>) {
            // Clean up temporary storage if needed
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Force a new epoch change (sudo only).
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(5, 10))]
        pub fn force_new_epoch(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            
            let current_slot = Self::current_slot();
            let block_number = frame_system::Pallet::<T>::block_number();
            
            Self::change_epoch(block_number, current_slot);
            
            Ok(())
        }
        
        /// Report an equivocation (producing multiple blocks in same slot).
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
        pub fn report_equivocation(
            origin: OriginFor<T>,
            _equivocation_proof: Vec<u8>, // Simplified for now
        ) -> DispatchResult {
            ensure_signed(origin)?;
            
            // TODO: Implement equivocation handling
            // This would slash the offending validator
            
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if we should change epoch.
        pub fn should_epoch_change(slot: Slot) -> bool {
            let epoch_duration = T::EpochDuration::get();
            
            if epoch_duration == 0 {
                return false;
            }
            
            // Change epoch when slot number is divisible by epoch duration
            (slot.as_u64() % epoch_duration) == 0
        }
        
        /// Perform epoch change.
        pub fn change_epoch(now: BlockNumberFor<T>, start_slot: Slot) {
            let new_epoch = EpochIndex::<T>::get() + 1;
            EpochIndex::<T>::put(new_epoch);
            LastEpochChange::<T>::put(now);
            
            // Rotate authorities
            let next_authorities = NextAuthorities::<T>::get();
            if !next_authorities.is_empty() {
                Authorities::<T>::put(next_authorities.clone());
            }
            
            // Rotate schedules
            if let Some(next_schedule) = NextSchedule::<T>::get() {
                CurrentSchedule::<T>::put(Some(next_schedule));
                NextSchedule::<T>::kill();
            }
            
            // Generate schedule for next epoch
            Self::generate_next_schedule(new_epoch + 1, start_slot + T::EpochDuration::get());
            
            Self::deposit_event(Event::EpochChanged {
                epoch_index: new_epoch,
                start_slot,
                authorities: Authorities::<T>::get().len() as u32,
            });
            
            log::info!(
                target: "throughput",
                "Epoch changed to {} at slot {} with {} authorities",
                new_epoch,
                start_slot.as_u64(),
                Authorities::<T>::get().len()
            );
        }
        
        /// Generate schedule for the next epoch.
        pub fn generate_next_schedule(epoch_index: u64, start_slot: Slot) {
            let authorities = Authorities::<T>::get();
            
            if authorities.is_empty() {
                log::warn!(target: "throughput", "No authorities for schedule generation");
                return;
            }
            
            // Calculate BGS for each authority
            let mut scores = Vec::new();
            for authority in authorities.iter() {
                let score = Self::calculate_bgs(authority);
                AuthorityScores::<T>::insert(authority, score.clone());
                scores.push(score);
                
                Self::deposit_event(Event::AuthorityScoreCalculated {
                    authority: authority.clone(),
                    score: score.score,
                    stake: score.stake,
                });
            }
            
            // Generate randomness for schedule
            let (random_seed, _) = T::Randomness::random(&epoch_index.to_le_bytes());
            let seed_bytes: [u8; 32] = random_seed.as_ref()
                .try_into()
                .unwrap_or([0u8; 32]);
            
            // Generate schedule using eABS algorithm
            let config = ScheduleConfig::default();
            let epoch_duration = T::EpochDuration::get();
            let schedule_indices = ScheduleGenerator::generate_schedule(
                &scores,
                epoch_duration,
                seed_bytes,
                &config,
            );
            
            // Convert indices to scheduled blocks
            let mut scheduled_blocks = Vec::new();
            for (slot_offset, &authority_index) in schedule_indices.iter().enumerate() {
                if let Some(authority) = authorities.get(authority_index as usize) {
                    scheduled_blocks.push(ScheduledBlock {
                        slot: start_slot + (slot_offset as u64),
                        authority: authority.clone(),
                        authority_index,
                    });
                }
            }
            
            // Create and store epoch schedule
            let epoch_schedule = EpochSchedule {
                epoch: epoch_index,
                start_slot,
                duration: epoch_duration,
                schedule: scheduled_blocks,
                randomness: seed_bytes,
            };
            
            NextSchedule::<T>::put(Some(epoch_schedule));
            
            Self::deposit_event(Event::ScheduleGenerated {
                epoch_index,
                schedule_length: schedule_indices.len() as u32,
            });
            
            log::info!(
                target: "throughput",
                "Generated schedule for epoch {} with {} blocks",
                epoch_index,
                schedule_indices.len()
            );
        }
        
        /// Calculate BGS for an authority.
        pub fn calculate_bgs(authority: &AuthorityId) -> BlockGenerationScore {
            // In production, get actual stake from staking pallet
            // For now, use a mock calculation
            let security_score = T::DefaultSecurityScore::get();
            let stake = T::MinimumStake::get() * 10; // Mock stake
            
            let score = BlockGenerationScore::calculate(security_score, stake);
            
            BlockGenerationScore {
                authority: authority.clone(),
                security_score,
                stake,
                score,
            }
        }
        
        /// Check if an authority is scheduled for a slot.
        pub fn is_scheduled(slot: Slot, authority: &AuthorityId) -> bool {
            CurrentSchedule::<T>::get()
                .map(|schedule| schedule.is_scheduled(slot, authority))
                .unwrap_or(false)
        }
        
        /// Get the scheduled authority for a slot.
        pub fn scheduled_authority(slot: Slot) -> Option<AuthorityId> {
            CurrentSchedule::<T>::get()
                .and_then(|schedule| schedule.get_authority_for_slot(slot).cloned())
        }
        
        /// Initialize genesis authorities.
        pub fn initialize_authorities(authorities: Vec<AuthorityId>) -> Result<(), DispatchError> {
            let bounded_authorities: BoundedVec<_, T::MaxAuthorities> = authorities
                .try_into()
                .map_err(|_| Error::<T>::TooManyAuthorities)?;
            
            Authorities::<T>::put(bounded_authorities);
            
            // Generate initial schedule
            Self::generate_next_schedule(0, Slot::from(0u64));
            
            // Move next schedule to current
            if let Some(schedule) = NextSchedule::<T>::get() {
                CurrentSchedule::<T>::put(Some(schedule));
                NextSchedule::<T>::kill();
            }
            
            Ok(())
        }
    }

    impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
        type Key = AuthorityId;
        
        fn on_genesis_session<'a, I: 'a>(validators: I)
        where
            I: Iterator<Item = (&'a T::AccountId, AuthorityId)>,
        {
            let authorities = validators.map(|(_, id)| id).collect::<Vec<_>>();
            let _ = Self::initialize_authorities(authorities);
        }
        
        fn on_new_session<'a, I: 'a>(changed: bool, validators: I, _queued: I)
        where
            I: Iterator<Item = (&'a T::AccountId, AuthorityId)>,
        {
            if changed {
                let authorities = validators.map(|(_, id)| id).collect::<Vec<_>>();
                let bounded_authorities: BoundedVec<_, T::MaxAuthorities> = authorities
                    .try_into()
                    .expect("Too many authorities");
                
                NextAuthorities::<T>::put(bounded_authorities);
            }
        }
        
        fn on_disabled(_i: u32) {
            // Handle validator being disabled
        }
    }
}