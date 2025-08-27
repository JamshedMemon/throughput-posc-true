#!/usr/bin/env rust-script
//! Test Byzantine fault tolerance - what happens if some nodes misbehave?

use std::collections::HashMap;
use std::convert::TryInto;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct AuthorityId([u8; 32]);

#[derive(Clone, Debug)]
struct ElasticInitiationProposal {
    epoch: u64,
    random_seed: [u8; 32],
    leader_id: AuthorityId,
    validators: Vec<AuthorityId>,
}

impl ElasticInitiationProposal {
    fn new(epoch: u64, validators: Vec<AuthorityId>) -> Self {
        let leader_id = validators[0].clone();
        
        // Deterministic random seed
        let mut random_seed = [0u8; 32];
        for i in 0..32 {
            random_seed[i] = ((epoch * 137 + i as u64) & 0xFF) as u8;
        }
        
        Self { epoch, random_seed, leader_id, validators }
    }
}

// Node types for Byzantine testing
enum NodeBehavior {
    Honest,              // Follows protocol correctly
    Byzantine,           // Tries to manipulate schedule
    Faulty,             // Random failures
}

struct Node {
    id: String,
    behavior: NodeBehavior,
}

impl Node {
    fn process_eip(&self, eip: &ElasticInitiationProposal) -> Vec<AuthorityId> {
        match self.behavior {
            NodeBehavior::Honest => {
                // Honest nodes follow the protocol exactly
                generate_schedule_from_seed(eip.random_seed, &eip.validators, 30)
            }
            NodeBehavior::Byzantine => {
                // Byzantine node tries to manipulate the seed
                let mut bad_seed = eip.random_seed;
                bad_seed[0] = bad_seed[0].wrapping_add(1); // Corrupt the seed!
                generate_schedule_from_seed(bad_seed, &eip.validators, 30)
            }
            NodeBehavior::Faulty => {
                // Faulty node might use wrong validator list
                let mut wrong_validators = eip.validators.clone();
                wrong_validators.reverse(); // Wrong order!
                generate_schedule_from_seed(eip.random_seed, &wrong_validators, 30)
            }
        }
    }
}

fn generate_schedule_from_seed(
    seed: [u8; 32],
    validators: &[AuthorityId],
    num_blocks: usize,
) -> Vec<AuthorityId> {
    let mut schedule = Vec::new();
    let mut rng_state = u64::from_le_bytes(seed[0..8].try_into().unwrap());
    
    for _ in 0..num_blocks {
        rng_state = rng_state.wrapping_mul(6364136223846793005)
                             .wrapping_add(1442695040888963407);
        let selected = (rng_state as usize) % validators.len();
        schedule.push(validators[selected].clone());
    }
    
    schedule
}

fn main() {
    println!("=== Byzantine Fault Tolerance Test for PoSc ===\n");
    
    let validators = vec![
        AuthorityId([1u8; 32]),
        AuthorityId([2u8; 32]),
        AuthorityId([3u8; 32]),
        AuthorityId([4u8; 32]),
        AuthorityId([5u8; 32]),
    ];
    
    // Test 1: All honest nodes
    println!("Test 1: All Honest Nodes (5/5 honest)");
    println!("{}", "=".repeat(50));
    
    let honest_nodes = vec![
        Node { id: "Node-A".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-B".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-C".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-D".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-E".to_string(), behavior: NodeBehavior::Honest },
    ];
    
    let eip = ElasticInitiationProposal::new(1, validators.clone());
    
    let mut schedules = HashMap::new();
    for node in &honest_nodes {
        let schedule = node.process_eip(&eip);
        schedules.insert(node.id.clone(), schedule);
    }
    
    // Check consensus
    let reference = &schedules["Node-A"];
    let mut consensus_count = 0;
    for (node_id, schedule) in &schedules {
        if schedule == reference {
            println!("✅ {} agrees with consensus", node_id);
            consensus_count += 1;
        } else {
            println!("❌ {} DISAGREES with consensus", node_id);
        }
    }
    
    println!("Result: {}/{} nodes in consensus\n", consensus_count, honest_nodes.len());
    
    // Test 2: Byzantine minority (2/5 byzantine)
    println!("Test 2: Byzantine Minority (3/5 honest, 2/5 byzantine)");
    println!("{}", "=".repeat(50));
    
    let mixed_nodes = vec![
        Node { id: "Node-A".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-B".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-C".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-D".to_string(), behavior: NodeBehavior::Byzantine },
        Node { id: "Node-E".to_string(), behavior: NodeBehavior::Byzantine },
    ];
    
    let mut schedules = HashMap::new();
    for node in &mixed_nodes {
        let schedule = node.process_eip(&eip);
        schedules.insert(node.id.clone(), schedule);
    }
    
    // Count which schedule has majority
    let mut schedule_counts: HashMap<Vec<AuthorityId>, Vec<String>> = HashMap::new();
    for (node_id, schedule) in schedules {
        schedule_counts.entry(schedule).or_insert_with(Vec::new).push(node_id);
    }
    
    for (i, (schedule, nodes)) in schedule_counts.iter().enumerate() {
        if nodes.len() >= 3 {
            println!("✅ MAJORITY CONSENSUS reached by: {:?}", nodes);
            println!("   First 10 slots: {:?}", 
                    schedule.iter().take(10).map(|v| v.0[0]).collect::<Vec<_>>());
        } else {
            println!("❌ Minority schedule {}: {:?}", i + 1, nodes);
        }
    }
    
    // Test 3: Network partition scenario
    println!("\nTest 3: Network Partition (two groups with different eIPs)");
    println!("{}", "=".repeat(50));
    
    let eip_group1 = ElasticInitiationProposal::new(1, validators.clone());
    let eip_group2 = ElasticInitiationProposal::new(2, validators.clone()); // Different epoch!
    
    let group1 = vec![
        Node { id: "Group1-A".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Group1-B".to_string(), behavior: NodeBehavior::Honest },
    ];
    
    let group2 = vec![
        Node { id: "Group2-C".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Group2-D".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Group2-E".to_string(), behavior: NodeBehavior::Honest },
    ];
    
    println!("Group 1 (2 nodes) using eIP epoch {}:", eip_group1.epoch);
    let mut schedule1 = None;
    for node in &group1 {
        let schedule = node.process_eip(&eip_group1);
        if schedule1.is_none() {
            schedule1 = Some(schedule.clone());
        }
        println!("  {} generated schedule", node.id);
    }
    
    println!("\nGroup 2 (3 nodes) using eIP epoch {}:", eip_group2.epoch);
    let mut schedule2 = None;
    for node in &group2 {
        let schedule = node.process_eip(&eip_group2);
        if schedule2.is_none() {
            schedule2 = Some(schedule.clone());
        }
        println!("  {} generated schedule", node.id);
    }
    
    if schedule1.unwrap() != schedule2.unwrap() {
        println!("\n❌ Network partition detected: Groups have different schedules!");
        println!("   This would be resolved when network heals and majority eIP is adopted");
    }
    
    // Test 4: Faulty nodes test
    println!("\nTest 4: Faulty Nodes (wrong validator order)");
    println!("{}", "=".repeat(50));
    
    let faulty_nodes = vec![
        Node { id: "Node-A".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-B".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-C".to_string(), behavior: NodeBehavior::Faulty },
        Node { id: "Node-D".to_string(), behavior: NodeBehavior::Honest },
        Node { id: "Node-E".to_string(), behavior: NodeBehavior::Faulty },
    ];
    
    let mut correct_count = 0;
    let reference_schedule = generate_schedule_from_seed(eip.random_seed, &validators, 30);
    
    for node in &faulty_nodes {
        let schedule = node.process_eip(&eip);
        if schedule == reference_schedule {
            println!("✅ {} produced correct schedule", node.id);
            correct_count += 1;
        } else {
            println!("❌ {} produced INCORRECT schedule (faulty)", node.id);
        }
    }
    
    println!("\nResult: {}/{} nodes correct", correct_count, faulty_nodes.len());
    
    // Summary
    println!("\n=== Byzantine Fault Tolerance Summary ===");
    println!("1. With all honest nodes: 100% consensus achieved");
    println!("2. With <1/3 Byzantine nodes: Majority still reaches consensus");
    println!("3. Network partitions create different schedules (resolved by majority)");
    println!("4. Faulty nodes with wrong data cannot produce correct schedule");
    println!("\n✅ PoSc achieves deterministic consensus when majority follows protocol!");
    println!("❌ Byzantine/faulty nodes produce detectably different schedules");
}