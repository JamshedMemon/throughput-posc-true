# True PoSc Consensus Implementation Design

## Core Concept: Deterministic Schedule-Based Block Production

Unlike BABE/Aura which use lottery/rounds, true PoSc uses a **predetermined schedule** where:
1. Every validator knows exactly which blocks they will produce
2. No competition or racing between validators  
3. The schedule is binding for the entire epoch
4. Validators can only produce blocks at their scheduled slots

## Implementation Strategy

### Step 1: Replace Aura with Custom PoSc Consensus

Instead of modifying BABE (complex), we'll create a new consensus engine based on Aura's simpler structure but with deterministic scheduling.

### Step 2: Core Components

#### A. PoSc Client Consensus (`sc-consensus-posc`)
- Schedule-based block production
- No VRF/lottery - just follow the schedule
- Deterministic validator selection per slot

#### B. PoSc Primitives (`sp-consensus-posc`)
- Schedule generation algorithm (eABS)
- BGS calculation
- Epoch management

#### C. PoSc Runtime Pallet (`pallet-posc`)
- On-chain schedule storage
- Epoch transitions
- Stake-based BGS calculation

### Step 3: Key Differences from Current Implementation

| Aspect | Current (Modified BABE) | True PoSc |
|--------|------------------------|-----------|
| Block Producer Selection | VRF lottery with stake bias | Deterministic schedule lookup |
| Competition | Validators compete each slot | No competition - assigned slots |
| Schedule | Generated but not enforced | Strictly enforced |
| Consensus Type | Probabilistic | Deterministic |
| Fork Choice | Longest chain | Schedule-aware fork choice |

### Step 4: Schedule Enforcement

The critical innovation is **schedule enforcement**:
```rust
fn can_produce_block(slot: Slot, authority: &AuthorityId) -> bool {
    let schedule = get_current_schedule();
    let scheduled_producer = schedule[slot.as_u64() % schedule.len()];
    scheduled_producer == authority
}
```

### Step 5: Fork Choice Rule

With deterministic scheduling, the fork choice must consider:
1. Is the block producer the scheduled one?
2. Did they produce at the correct slot?
3. Is this the heaviest valid chain following the schedule?

## File Structure

```
throughput-posc-true/
├── consensus/
│   └── posc/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs           # PoSc consensus engine
│           ├── import.rs        # Block import with schedule verification
│           └── authorship.rs    # Schedule-based authorship
├── primitives/
│   └── posc/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs           # PoSc primitives
│           └── schedule.rs      # eABS algorithm
├── pallets/
│   └── posc/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs           # Runtime pallet
└── template/
    ├── node/                     # Modified to use PoSc
    └── runtime/                  # Modified to use PoSc
```

## Implementation Phases

### Phase 1: Basic Structure (Today)
1. Create consensus engine skeleton
2. Implement schedule generation
3. Basic slot-to-validator mapping

### Phase 2: Integration
1. Replace Aura in template
2. Add schedule enforcement
3. Implement fork choice rules

### Phase 3: Testing
1. Multi-validator testing
2. Schedule compliance verification
3. Fork resolution testing

## Advantages of True PoSc

1. **Predictability**: Validators know exactly when they'll produce blocks
2. **Fairness**: Guaranteed proportional representation
3. **Efficiency**: No wasted computation on losing bids
4. **Cross-chain Ready**: Schedules can be coordinated across chains
5. **Lower Latency**: No need to wait for VRF reveals

## Challenges to Address

1. **Time Synchronization**: Validators must agree on slot timing
2. **Schedule Distribution**: All validators must have the same schedule
3. **Missed Slots**: Handling when scheduled validator is offline
4. **Security**: Preventing schedule manipulation

This is the true PoSc as described in your whitepaper!