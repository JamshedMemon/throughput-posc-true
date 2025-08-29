#!/bin/bash

echo "=== Extracting and Running PoSC Test Functions ==="
echo ""

# Extract test functions from the actual code
echo "Test functions found in PoSC implementation:"
echo ""

echo "1. EIP Tests (primitives/posc/src/eip.rs):"
grep -A 15 "fn test_" primitives/posc/src/eip.rs | head -20

echo ""
echo "2. EABS Tests (primitives/posc/src/eabs.rs):"
grep -A 15 "fn test_" primitives/posc/src/eabs.rs | head -40

echo ""
echo "3. Pallet Tests (pallets/posc/src/tests.rs):"
grep -A 10 "fn test_\|fn it_" pallets/posc/src/tests.rs | head -30

echo ""
echo "=== Test Summary ==="
echo "Total test functions in PoSC:"
find pallets/posc primitives/posc -name "*.rs" -exec grep -c "fn test_\|fn it_" {} + | awk '{sum+=$1} END {print sum}'