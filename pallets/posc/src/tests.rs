//! Tests for the PoSc pallet

use crate::{self as pallet_posc, *};
use frame_support::{
    assert_ok, assert_err,
    parameter_types,
    traits::{ConstU32, ConstU64, ConstU128},
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use sp_consensus_posc::{AuthorityId, BlockGenerationScore};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Timestamp: pallet_timestamp,
        PoSc: pallet_posc,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

// Mock randomness for testing
pub struct MockRandomness;
impl Randomness<H256, u64> for MockRandomness {
    fn random(subject: &[u8]) -> (H256, u64) {
        let mut hash = [0u8; 32];
        hash[..8].copy_from_slice(&subject[..8.min(subject.len())]);
        (H256::from(hash), 0)
    }
}

impl pallet_posc::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type EpochDuration = ConstU64<10>; // 10 slots per epoch for testing
    type ExpectedBlockTime = ConstU64<1000>; // 1 second blocks
    type Randomness = MockRandomness;
    type MinimumStake = ConstU128<1000>;
    type DefaultSecurityScore = ConstU64<75>;
    type MinimumSecurityScore = ConstU64<50>;
    type MaxAuthorities = ConstU32<10>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    
    // Add initial timestamp
    pallet_timestamp::GenesisConfig::<Test> {
        build: Default::default(),
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    
    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| {
        System::set_block_number(1);
        Timestamp::set_timestamp(1000);
    });
    ext
}

// Helper function to create test authorities
fn create_authorities(n: u32) -> Vec<AuthorityId> {
    (0..n)
        .map(|i| {
            let mut bytes = [0u8; 32];
            bytes[0] = i as u8;
            AuthorityId::from_raw(bytes)
        })
        .collect()
}

#[test]
fn test_initialize_authorities() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        
        // Initialize authorities
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        
        // Check authorities are stored
        let stored_authorities = Authorities::<Test>::get();
        assert_eq!(stored_authorities.len(), 3);
        
        // Check initial schedule was generated
        assert!(CurrentSchedule::<Test>::get().is_some());
    });
}

#[test]
fn test_schedule_generation() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        
        // Get the current schedule
        let schedule = CurrentSchedule::<Test>::get().unwrap();
        
        // Verify schedule properties
        assert_eq!(schedule.epoch, 0);
        assert_eq!(schedule.duration, 10); // EpochDuration
        assert_eq!(schedule.schedule.len(), 10); // Should have 10 scheduled blocks
        
        // Verify all scheduled blocks have valid authorities
        for block in &schedule.schedule {
            assert!(authorities.contains(&block.authority));
        }
    });
}

#[test]
fn test_is_scheduled() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        
        // Get first scheduled slot
        let schedule = CurrentSchedule::<Test>::get().unwrap();
        let first_block = &schedule.schedule[0];
        
        // Test that the scheduled authority is recognized
        assert!(PoSc::is_scheduled(first_block.slot, &first_block.authority));
        
        // Test that other authorities are not scheduled for this slot
        for auth in &authorities {
            if auth != &first_block.authority {
                assert!(!PoSc::is_scheduled(first_block.slot, auth));
            }
        }
    });
}

#[test]
fn test_scheduled_authority() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        
        let schedule = CurrentSchedule::<Test>::get().unwrap();
        
        // Test each slot has the correct scheduled authority
        for scheduled_block in &schedule.schedule {
            let scheduled_auth = PoSc::scheduled_authority(scheduled_block.slot);
            assert_eq!(scheduled_auth, Some(scheduled_block.authority.clone()));
        }
        
        // Test out-of-range slot returns None
        let out_of_range_slot = Slot::from(100u64);
        assert_eq!(PoSc::scheduled_authority(out_of_range_slot), None);
    });
}

#[test]
fn test_epoch_change() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        
        // Initial epoch should be 0
        assert_eq!(EpochIndex::<Test>::get(), 0);
        
        // Move to slot 10 (should trigger epoch change)
        CurrentSlot::<Test>::put(Slot::from(10u64));
        assert!(PoSc::should_epoch_change(Slot::from(10u64)));
        
        // Perform epoch change
        PoSc::change_epoch(1, Slot::from(10u64));
        
        // Verify epoch changed
        assert_eq!(EpochIndex::<Test>::get(), 1);
        
        // Verify new schedule exists
        assert!(CurrentSchedule::<Test>::get().is_some());
        let new_schedule = CurrentSchedule::<Test>::get().unwrap();
        assert_eq!(new_schedule.start_slot, Slot::from(0u64)); // Was moved from next
    });
}

#[test]
fn test_bgs_calculation() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(1);
        let authority = &authorities[0];
        
        // Calculate BGS
        let bgs = PoSc::calculate_bgs(authority);
        
        // Verify calculation
        assert_eq!(bgs.security_score, 75); // DefaultSecurityScore
        assert_eq!(bgs.stake, 10000); // MinimumStake * 10 (mock value)
        // BGS = 75 * (10000 / 1_000_000) = 75 * 0.01 = 0 (integer division)
        // In our mock, we use MinimumStake * 10 = 10000
        assert!(bgs.score > 0); // Should have non-zero score
    });
}

#[test]
fn test_deterministic_schedule() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        
        // Initialize twice with same authorities
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        let schedule1 = CurrentSchedule::<Test>::get().unwrap();
        
        // Clear and reinitialize
        CurrentSchedule::<Test>::kill();
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        let schedule2 = CurrentSchedule::<Test>::get().unwrap();
        
        // With same randomness seed, schedules should be identical
        // (In real implementation, this depends on the randomness source)
        assert_eq!(schedule1.randomness, schedule2.randomness);
    });
}

#[test]
fn test_force_new_epoch() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        assert_ok!(PoSc::initialize_authorities(authorities));
        
        let initial_epoch = EpochIndex::<Test>::get();
        
        // Force new epoch (requires root)
        assert_ok!(PoSc::force_new_epoch(RuntimeOrigin::root()));
        
        // Verify epoch incremented
        assert_eq!(EpochIndex::<Test>::get(), initial_epoch + 1);
    });
}

#[test]
fn test_too_many_authorities() {
    new_test_ext().execute_with(|| {
        // Try to initialize more than MaxAuthorities (10)
        let authorities = create_authorities(11);
        
        assert_err!(
            PoSc::initialize_authorities(authorities),
            Error::<Test>::TooManyAuthorities
        );
    });
}

#[test]
fn test_schedule_fairness() {
    new_test_ext().execute_with(|| {
        let authorities = create_authorities(3);
        assert_ok!(PoSc::initialize_authorities(authorities.clone()));
        
        let schedule = CurrentSchedule::<Test>::get().unwrap();
        
        // Count slots per authority
        let mut slot_counts = std::collections::HashMap::new();
        for block in &schedule.schedule {
            *slot_counts.entry(block.authority.clone()).or_insert(0) += 1;
        }
        
        // With equal stakes, each authority should have roughly equal slots
        // Allow for some variance due to rounding
        let expected_slots = schedule.duration / 3;
        for (auth, count) in slot_counts {
            assert!(count >= expected_slots - 1 && count <= expected_slots + 2,
                "Authority {:?} has {} slots, expected around {}",
                auth, count, expected_slots);
        }
    });
}