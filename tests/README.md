# Throughput PoSC Tests

This directory contains test utilities and scripts for the Throughput blockchain's Proof of Schedule (PoSC) consensus implementation.

## Test Files

### `/posc/` - PoSC Consensus Tests
- `test_posc.rs` - Standalone test binary for PoSC algorithms
- `run_posc_tests.sh` - Script to run PoSC unit tests
- `extract_test.sh` - Script to extract and display test functions from source

## Running Tests

### Quick Algorithm Tests (No compilation needed)
```bash
cd tests/posc
rustc test_posc.rs -o test_posc && ./test_posc
```

### Full Test Suite
```bash
# From project root
cargo test --all
```

### PoSC Specific Tests
```bash
# Test EIP (Algorithm 1)
cargo test test_eip_creation_and_verification

# Test EABS (Algorithm 3)  
cargo test test_eabs_schedule_generation

# Test Pallet
cargo test --package pallet-posc
```

## Test Coverage

The tests validate:
- **Algorithm 1 (EIP)**: Elastic Initiation Proposal leader selection
- **Algorithm 3 (EABS)**: Elastic Advanced Block Schedule generation
- **Consensus Flow**: Complete epoch cycle and block production
- **Stake Distribution**: Proportional block allocation based on stake

## Integration Tests

For multi-node testing, see `/docs/DISTRIBUTION_GUIDE.md` for instructions on running local testnets.