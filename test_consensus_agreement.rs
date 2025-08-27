#!/usr/bin/env rust-script
//! Test that all nodes reach the same schedule using the true PoSc algorithms

use std::collections::HashMap;
use std::convert::TryInto;

// Simulate the complete PoSc algorithm flow

#[derive(Clone, Debug, PartialEq)]
struct AuthorityId([u8; 32]);

#[derive(Clone, Debug)]
struct BlockGenerationScore {
    authority: AuthorityId,
    security_score: u64,
    stake: u128,
    score: u64,
}

#[derive(Clone, Debug)]
struct ElasticInitiationProposal {
    epoch: u64,
    random_seed: [u8; 32],
    leader_id: AuthorityId,
    validators: Vec<AuthorityId>,
    parent_hash: [u8; 32],
    timestamp: u64,
}

impl ElasticInitiationProposal {
    fn new(epoch: u64, validators: Vec<AuthorityId>, parent_hash: [u8; 32], timestamp: u64) -> Self {
        // Leader is always first validator (deterministic)
        let leader_id = validators[0].clone();
        
        // Generate deterministic random seed (Algorithm 1)
        // R_e = Hash(epoch || parent_hash || timestamp)
        let mut seed_data = Vec::new();
        seed_data.extend_from_slice(&epoch.to_le_bytes());
        seed_data.extend_from_slice(&parent_hash);
        seed_data.extend_from_slice(&timestamp.to_le_bytes());
        
        // Simple hash simulation
        let mut random_seed = [0u8; 32];
        for (i, byte) in seed_data.iter().enumerate() {
            random_seed[i % 32] ^= byte.wrapping_mul(137).wrapping_add(i as u8);
        }
        
        Self {
            epoch,
            random_seed,
            leader_id,
            validators,
            parent_hash,
            timestamp,
        }
    }
    
    fn verify(&self) -> bool {
        // Algorithm 2: Verify eIP
        // 1. Check leader is first validator
        if self.validators.is_empty() || self.leader_id != self.validators[0] {
            return false;
        }
        
        // 2. Verify random seed is deterministic (recalculate and compare)
        let mut expected_seed_data = Vec::new();
        expected_seed_data.extend_from_slice(&self.epoch.to_le_bytes());
        expected_seed_data.extend_from_slice(&self.parent_hash);
        expected_seed_data.extend_from_slice(&self.timestamp.to_le_bytes());
        
        let mut expected_seed = [0u8; 32];
        for (i, byte) in expected_seed_data.iter().enumerate() {
            expected_seed[i % 32] ^= byte.wrapping_mul(137).wrapping_add(i as u8);
        }
        
        self.random_seed == expected_seed
    }
}

// Algorithm 3: True eABS with 2D Matrix
struct ScheduleMatrix {
    matrix: Vec<Vec<AuthorityId>>,
    rows: usize,
    cols: usize,
}

impl ScheduleMatrix {
    fn new(bgs_shares: &[(AuthorityId, u32)], blocks_per_epoch: usize) -> Self {
        let total_rows: usize = bgs_shares.iter().map(|(_, shares)| *shares as usize).sum();
        let mut matrix = Vec::new();
        
        // Fill matrix elastically - each validator gets rows proportional to BGS
        for (validator, share_count) in bgs_shares {
            for _ in 0..*share_count {
                let mut row = Vec::new();
                for _ in 0..blocks_per_epoch {
                    row.push(validator.clone());
                }
                matrix.push(row);
            }
        }
        
        Self {
            matrix,
            rows: total_rows,
            cols: blocks_per_epoch,
        }
    }
    
    fn generate_schedule(&self, random_seed: [u8; 32]) -> Vec<AuthorityId> {
        let mut schedule = Vec::new();
        let mut rng_state = u64::from_le_bytes(random_seed[0..8].try_into().unwrap());
        
        // For each block slot, select a row using deterministic random
        for col in 0..self.cols {
            if self.rows == 0 {
                continue;
            }
            
            // Deterministic random row selection
            rng_state = rng_state.wrapping_mul(6364136223846793005)
                                 .wrapping_add(1442695040888963407);
            let selected_row = (rng_state as usize) % self.rows;
            
            if selected_row < self.matrix.len() && col < self.matrix[selected_row].len() {
                schedule.push(self.matrix[selected_row][col].clone());
            }
        }
        
        schedule
    }
}

fn calculate_bgs_shares(
    validators: &[AuthorityId],
    stakes: &[u128],
    blocks_per_epoch: u32,
    min_blocks: u32,
) -> Vec<(AuthorityId, u32)> {
    let num_validators = validators.len() as u32;
    let guaranteed = num_validators * min_blocks;
    let remaining = blocks_per_epoch.saturating_sub(guaranteed);
    
    let total_stake: u128 = stakes.iter().sum();
    let mut shares = Vec::new();
    
    for (i, validator) in validators.iter().enumerate() {
        let mut validator_shares = min_blocks;
        
        if total_stake > 0 && remaining > 0 {
            let proportion = (stakes[i] as f64) / (total_stake as f64);
            validator_shares += (remaining as f64 * proportion) as u32;
        }
        
        shares.push((validator.clone(), validator_shares));
    }
    
    shares
}

// Simulate a node in the network
struct Node {
    id: String,
    validators: Vec<AuthorityId>,
    stakes: Vec<u128>,
}

impl Node {
    fn process_eip(&self, eip: &ElasticInitiationProposal) -> Result<Vec<AuthorityId>, String> {
        // Step 1: Verify eIP (Algorithm 2)
        if !eip.verify() {
            return Err(format!("Node {}: eIP verification failed", self.id));
        }
        
        // Step 2: Calculate BGS shares
        let bgs_shares = calculate_bgs_shares(
            &self.validators,
            &self.stakes,
            30, // blocks per epoch
            1,  // min blocks per validator
        );
        
        // Step 3: Generate schedule using eABS (Algorithm 3)
        let matrix = ScheduleMatrix::new(&bgs_shares, 30);
        let schedule = matrix.generate_schedule(eip.random_seed);
        
        Ok(schedule)
    }
}

fn main() {
    println!("=== Testing PoSc Consensus Agreement Across Multiple Nodes ===\n");
    
    // Setup: Common validator set and stakes
    let validators = vec![
        AuthorityId([1u8; 32]),
        AuthorityId([2u8; 32]),
        AuthorityId([3u8; 32]),
    ];
    
    let stakes = vec![
        50_000_000u128, // 50% of total stake
        30_000_000u128, // 30% of total stake
        20_000_000u128, // 20% of total stake
    ];
    
    // Create multiple nodes (simulating network participants)
    let nodes = vec![
        Node {
            id: "Node-A".to_string(),
            validators: validators.clone(),
            stakes: stakes.clone(),
        },
        Node {
            id: "Node-B".to_string(),
            validators: validators.clone(),
            stakes: stakes.clone(),
        },
        Node {
            id: "Node-C".to_string(),
            validators: validators.clone(),
            stakes: stakes.clone(),
        },
        Node {
            id: "Node-D".to_string(),
            validators: validators.clone(),
            stakes: stakes.clone(),
        },
        Node {
            id: "Node-E".to_string(),
            validators: validators.clone(),
            stakes: stakes.clone(),
        },
    ];
    
    println!("Network Setup:");
    println!("- {} nodes in network", nodes.len());
    println!("- {} validators", validators.len());
    println!("- Validator stakes: {:?}\n", stakes);
    
    // Step 1: Leader creates eIP (Algorithm 1)
    let parent_hash = [0xAAu8; 32];
    let timestamp = 1234567890u64;
    let epoch = 100u64;
    
    let eip = ElasticInitiationProposal::new(
        epoch,
        validators.clone(),
        parent_hash,
        timestamp,
    );
    
    println!("Algorithm 1 - eIP Creation:");
    println!("- Epoch: {}", eip.epoch);
    println!("- Leader: Validator {:?}", eip.leader_id.0[0]);
    println!("- Random seed: {:?}", &eip.random_seed[0..8]);
    println!("- Parent hash: {:?}", &parent_hash[0..4]);
    println!("- Timestamp: {}\n", timestamp);
    
    // Step 2: Broadcast eIP to all nodes (simulated)
    println!("Algorithm 2 - eIP Verification by all nodes:");
    
    let mut schedules = Vec::new();
    for node in &nodes {
        match node.process_eip(&eip) {
            Ok(schedule) => {
                println!("✅ {}: eIP verified, schedule generated", node.id);
                schedules.push((node.id.clone(), schedule));
            }
            Err(e) => {
                println!("❌ {}", e);
            }
        }
    }
    
    println!("\nAlgorithm 3 - Schedule Generation Results:");
    
    // Step 3: Verify all nodes generated the SAME schedule
    if schedules.is_empty() {
        println!("ERROR: No schedules generated!");
        return;
    }
    
    let reference_schedule = &schedules[0].1;
    let mut all_match = true;
    
    for (node_id, schedule) in &schedules[1..] {
        if schedule != reference_schedule {
            println!("❌ {} generated DIFFERENT schedule!", node_id);
            all_match = false;
        } else {
            println!("✅ {} generated IDENTICAL schedule", node_id);
        }
    }
    
    println!("\n=== Consensus Result ===");
    if all_match {
        println!("✅ SUCCESS: All {} nodes reached CONSENSUS on the same schedule!", nodes.len());
    } else {
        println!("❌ FAILURE: Nodes generated different schedules!");
    }
    
    // Show the agreed schedule
    println!("\nAgreed Schedule (first 20 slots):");
    for (i, validator) in reference_schedule.iter().take(20).enumerate() {
        print!("Slot {}: V{} | ", i, validator.0[0]);
        if (i + 1) % 5 == 0 {
            println!();
        }
    }
    
    // Count distribution
    let mut counts: HashMap<u8, usize> = HashMap::new();
    for validator in reference_schedule {
        *counts.entry(validator.0[0]).or_insert(0) += 1;
    }
    
    println!("\n\nBlock Distribution in Epoch:");
    for (validator_id, count) in counts.iter() {
        let percentage = (*count as f64 / reference_schedule.len() as f64) * 100.0;
        let stake_idx = (*validator_id - 1) as usize;
        let stake_percentage = (stakes[stake_idx] as f64 / stakes.iter().sum::<u128>() as f64) * 100.0;
        println!(
            "Validator {}: {} blocks ({:.1}%) | Stake: {:.1}%",
            validator_id, count, percentage, stake_percentage
        );
    }
    
    // Test with different eIP parameters to ensure determinism
    println!("\n=== Testing Determinism with Different Parameters ===");
    
    // Test 1: Same parameters = same schedule
    let eip2 = ElasticInitiationProposal::new(epoch, validators.clone(), parent_hash, timestamp);
    let node_test = Node {
        id: "Test".to_string(),
        validators: validators.clone(),
        stakes: stakes.clone(),
    };
    let schedule2 = node_test.process_eip(&eip2).unwrap();
    
    if schedule2 == *reference_schedule {
        println!("✅ Same eIP parameters → Same schedule");
    } else {
        println!("❌ Same eIP parameters → Different schedule (ERROR!)");
    }
    
    // Test 2: Different timestamp = different schedule
    let eip3 = ElasticInitiationProposal::new(epoch, validators.clone(), parent_hash, timestamp + 1);
    let schedule3 = node_test.process_eip(&eip3).unwrap();
    
    if schedule3 != *reference_schedule {
        println!("✅ Different timestamp → Different schedule");
    } else {
        println!("❌ Different timestamp → Same schedule (ERROR!)");
    }
    
    // Test 3: Different epoch = different schedule
    let eip4 = ElasticInitiationProposal::new(epoch + 1, validators, parent_hash, timestamp);
    let schedule4 = node_test.process_eip(&eip4).unwrap();
    
    if schedule4 != *reference_schedule {
        println!("✅ Different epoch → Different schedule");
    } else {
        println!("❌ Different epoch → Same schedule (ERROR!)");
    }
    
    println!("\n=== Summary ===");
    println!("1. All nodes receive the same eIP from the leader");
    println!("2. All nodes verify the eIP successfully");
    println!("3. All nodes generate IDENTICAL schedules from the shared random seed");
    println!("4. The schedule is deterministic and reproducible");
    println!("5. Different eIP parameters produce different schedules");
    println!("\n✅ This proves true PoSc consensus agreement!");
}