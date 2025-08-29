#!/bin/bash

echo "=== Running PoSC Consensus Tests ==="
echo ""
echo "Testing PoSC primitives (EIP and EABS algorithms)..."

# Run specific test modules
cargo test eip --lib 2>&1 | grep -A10 "test result"
cargo test eabs --lib 2>&1 | grep -A10 "test result"
cargo test schedule --lib 2>&1 | grep -A10 "test result"
cargo test posc --lib 2>&1 | grep -A10 "test result"

echo ""
echo "=== Quick Test Summary ==="
cargo test --lib 2>&1 | grep "test result" | tail -5