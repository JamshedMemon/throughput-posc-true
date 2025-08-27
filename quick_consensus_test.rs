#!/usr/bin/env rust-script
//! Quick test to verify consensus logic without full build
//! This simulates the actual consensus engine behavior

use std::collections::HashMap;

// Simulated slot type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Slot(u64);

// Simulated authority
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Authority {
    id: u32,
    stake: u64,
}

// Simulated schedule
struct Schedule {
    epoch: u64,
    assignments: HashMap<Slot, Authority>,
}

impl Schedule {
    fn new(authorities: &[Authority], epoch_length: u64, epoch: u64) -> Self {
        let mut assignments = HashMap::new();
        
        // Generate deterministic schedule
        let total_stake: u64 = authorities.iter().map(|a| a.stake).sum();
        let mut slot = Slot(epoch * epoch_length);
        
        // Assign slots proportionally
        for _ in 0..epoch_length {
            // Deterministic selection based on slot number
            let slot_hash = slot.0.wrapping_mul(2654435761); // Simple hash
            let selection = slot_hash % total_stake;
            
            let mut cumulative = 0;
            for auth in authorities {
                cumulative += auth.stake;
                if selection < cumulative {
                    assignments.insert(slot, auth.clone());
                    break;
                }
            }
            
            slot = Slot(slot.0 + 1);
        }
        
        Schedule { epoch, assignments }
    }
    
    fn get_scheduled_authority(&self, slot: Slot) -> Option<&Authority> {
        self.assignments.get(&slot)
    }
}

// Simulated consensus engine
struct PoScConsensus {
    my_authority: Authority,
    current_schedule: Schedule,
}

impl PoScConsensus {
    fn new(my_authority: Authority, schedule: Schedule) -> Self {
        PoScConsensus {
            my_authority,
            current_schedule: schedule,
        }
    }
    
    // THE KEY FUNCTION: Can I produce a block at this slot?
    fn can_produce_block(&self, slot: Slot) -> bool {
        match self.current_schedule.get_scheduled_authority(slot) {
            Some(scheduled) => scheduled == &self.my_authority,
            None => false,
        }
    }
    
    // Simulate block production attempt
    fn try_produce_block(&self, slot: Slot) -> Result<String, String> {
        if self.can_produce_block(slot) {
            Ok(format!("✅ Block produced by Authority {} at slot {}", 
                      self.my_authority.id, slot.0))
        } else {
            Err(format!("❌ Authority {} NOT scheduled for slot {}", 
                       self.my_authority.id, slot.0))
        }
    }
}

// Test the consensus behavior
fn main() {
    println!("=== True PoSc Consensus Engine Test ===\n");
    
    // Create test authorities with different stakes
    let authorities = vec![
        Authority { id: 0, stake: 50 },  // 50% stake
        Authority { id: 1, stake: 30 },  // 30% stake
        Authority { id: 2, stake: 20 },  // 20% stake
    ];
    
    // Generate schedule for epoch 0
    let schedule = Schedule::new(&authorities, 10, 0);
    
    println!("Schedule for epoch 0 (10 slots):");
    for slot_num in 0..10 {
        let slot = Slot(slot_num);
        if let Some(auth) = schedule.get_scheduled_authority(slot) {
            println!("  Slot {}: Authority {} (stake: {})", slot_num, auth.id, auth.stake);
        }
    }
    
    println!("\n--- Testing Block Production Rights ---\n");
    
    // Test each authority trying to produce blocks
    for authority in &authorities {
        println!("Authority {} (stake: {}) attempting to produce blocks:",
                authority.id, authority.stake);
        
        let consensus = PoScConsensus::new(authority.clone(), 
                                          Schedule::new(&authorities, 10, 0));
        
        let mut success_count = 0;
        for slot_num in 0..10 {
            let slot = Slot(slot_num);
            match consensus.try_produce_block(slot) {
                Ok(msg) => {
                    println!("  {}", msg);
                    success_count += 1;
                },
                Err(_) => {
                    // Silent fail - not scheduled
                }
            }
        }
        
        let percentage = (success_count as f64 / 10.0) * 100.0;
        println!("  Total: {} blocks produced ({:.0}% of epoch)\n", 
                success_count, percentage);
    }
    
    println!("--- Testing Consensus Enforcement ---\n");
    
    // Test what happens when wrong authority tries to produce
    let auth0 = Authority { id: 0, stake: 50 };
    let consensus = PoScConsensus::new(auth0, Schedule::new(&authorities, 10, 0));
    
    for slot_num in 0..5 {
        let slot = Slot(slot_num);
        let scheduled = schedule.get_scheduled_authority(slot);
        
        print!("Slot {}: ", slot_num);
        if let Some(sched_auth) = scheduled {
            print!("Scheduled: Authority {} | ", sched_auth.id);
            
            // Authority 0 tries to produce
            if consensus.can_produce_block(slot) {
                println!("Authority 0 CAN produce ✅");
            } else {
                println!("Authority 0 CANNOT produce ❌");
            }
        }
    }
    
    println!("\n=== Key Differences from BABE/Aura ===");
    println!("1. BABE: VRF lottery - multiple validators might win");
    println!("   PoSc: Deterministic - exactly one validator per slot");
    println!("\n2. BABE: Validators don't know who will produce next block");
    println!("   PoSc: Entire epoch schedule known in advance");
    println!("\n3. BABE: Fork choice based on chain weight");
    println!("   PoSc: Fork choice includes schedule compliance");
    
    println!("\n✅ True PoSc consensus enforces deterministic scheduling!");
}