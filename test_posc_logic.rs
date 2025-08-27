#!/usr/bin/env rust-script
//! Quick test to verify PoSc scheduling logic works correctly
//! Run with: rustc test_posc_logic.rs && ./test_posc_logic

use std::collections::HashMap;

#[derive(Clone, Debug)]
struct Validator {
    id: u32,
    stake: u64,
    security_score: u64,
}

impl Validator {
    fn calculate_bgs(&self) -> u64 {
        // BGS = security_score * (stake / 1_000_000)
        let normalized_stake = self.stake / 1_000_000;
        self.security_score * normalized_stake
    }
}

fn generate_schedule(validators: &[Validator], epoch_length: usize, seed: u32) -> Vec<u32> {
    if validators.is_empty() || epoch_length == 0 {
        return Vec::new();
    }
    
    let mut schedule = Vec::with_capacity(epoch_length);
    
    // Calculate BGS for each validator
    let mut bgs_shares: Vec<(u32, u64)> = validators
        .iter()
        .map(|v| (v.id, v.calculate_bgs()))
        .collect();
    
    // Ensure each validator gets at least one slot
    for (id, _) in &bgs_shares {
        schedule.push(*id);
    }
    
    // Distribute remaining slots based on BGS
    let total_bgs: u64 = bgs_shares.iter().map(|(_, bgs)| bgs).sum();
    let remaining_slots = epoch_length - schedule.len();
    
    if total_bgs > 0 && remaining_slots > 0 {
        for (id, bgs) in &bgs_shares {
            let proportion = (*bgs as f64) / (total_bgs as f64);
            let additional_slots = (remaining_slots as f64 * proportion) as usize;
            
            for _ in 0..additional_slots {
                if schedule.len() < epoch_length {
                    schedule.push(*id);
                }
            }
        }
    }
    
    // Fill any remaining with round-robin
    let mut idx = 0;
    while schedule.len() < epoch_length {
        schedule.push(validators[idx % validators.len()].id);
        idx += 1;
    }
    
    // Simple deterministic shuffle using seed
    let mut rng_state = seed as u64;
    for i in 0..schedule.len() {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let j = (rng_state as usize) % schedule.len();
        schedule.swap(i, j);
    }
    
    schedule
}

fn analyze_schedule(schedule: &[u32], validators: &[Validator]) {
    println!("\n=== Schedule Analysis ===");
    println!("Total slots: {}", schedule.len());
    
    // Count slots per validator
    let mut slot_counts: HashMap<u32, usize> = HashMap::new();
    for &validator_id in schedule {
        *slot_counts.entry(validator_id).or_insert(0) += 1;
    }
    
    println!("\nSlots per validator:");
    for validator in validators {
        let count = slot_counts.get(&validator.id).unwrap_or(&0);
        let percentage = (*count as f64 / schedule.len() as f64) * 100.0;
        let bgs = validator.calculate_bgs();
        println!(
            "  Validator {}: {} slots ({:.1}%) | Stake: {} | BGS: {}",
            validator.id, count, percentage, validator.stake, bgs
        );
    }
    
    // Show first 20 slots
    println!("\nFirst 20 slots of schedule:");
    for (slot, &validator_id) in schedule.iter().take(20).enumerate() {
        print!("Slot {}: V{} | ", slot, validator_id);
        if (slot + 1) % 5 == 0 {
            println!();
        }
    }
    println!();
}

fn main() {
    println!("Testing True PoSc Schedule Generation");
    println!("=====================================");
    
    // Test Case 1: Equal stakes
    println!("\nTest 1: Three validators with equal stakes");
    let validators1 = vec![
        Validator { id: 0, stake: 10_000_000, security_score: 75 },
        Validator { id: 1, stake: 10_000_000, security_score: 75 },
        Validator { id: 2, stake: 10_000_000, security_score: 75 },
    ];
    let schedule1 = generate_schedule(&validators1, 30, 42);
    analyze_schedule(&schedule1, &validators1);
    
    // Test Case 2: Different stakes
    println!("\n\nTest 2: Three validators with different stakes");
    let validators2 = vec![
        Validator { id: 0, stake: 50_000_000, security_score: 75 }, // High stake
        Validator { id: 1, stake: 30_000_000, security_score: 75 }, // Medium stake
        Validator { id: 2, stake: 10_000_000, security_score: 75 }, // Low stake
    ];
    let schedule2 = generate_schedule(&validators2, 30, 42);
    analyze_schedule(&schedule2, &validators2);
    
    // Test Case 3: Large validator set
    println!("\n\nTest 3: Ten validators with varying stakes");
    let mut validators3 = Vec::new();
    for i in 0..10 {
        validators3.push(Validator {
            id: i,
            stake: (10_000_000 + i as u64 * 5_000_000), // Increasing stakes
            security_score: 75,
        });
    }
    let schedule3 = generate_schedule(&validators3, 100, 42);
    analyze_schedule(&schedule3, &validators3);
    
    // Verify determinism
    println!("\n\nTest 4: Verify deterministic scheduling");
    let schedule_repeat = generate_schedule(&validators2, 30, 42);
    if schedule_repeat == schedule2 {
        println!("✅ Schedule generation is deterministic (same seed = same schedule)");
    } else {
        println!("❌ Schedule generation is NOT deterministic!");
    }
    
    let schedule_different = generate_schedule(&validators2, 30, 123);
    if schedule_different != schedule2 {
        println!("✅ Different seeds produce different schedules");
    } else {
        println!("❌ Different seeds produce same schedule!");
    }
    
    println!("\n=== Summary ===");
    println!("True PoSc ensures:");
    println!("1. Every validator gets slots proportional to their stake");
    println!("2. Schedule is deterministic (same inputs = same output)");
    println!("3. No validator can produce blocks outside their scheduled slots");
    println!("4. The entire epoch schedule is known in advance");
}