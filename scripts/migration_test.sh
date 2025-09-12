#!/bin/bash

# migration_test.sh - Script to run migration tests systematically

echo "=== PROTTO MIGRATION TESTING ==="
echo "Testing the new FieldConversionStrategy system against existing implementation"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_test() {
    local test_name="$1"
    local migration_mode="$2"
    local test_filter="$3"

    echo -e "${YELLOW}Running: $test_name (mode: $migration_mode)${NC}"

    export PROTTO_MIGRATION_MODE="$migration_mode"

    if [ -n "$test_filter" ]; then
        cargo test "$test_filter" -- --nocapture
    else
        cargo test migration_tests -- --nocapture
    fi

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ PASSED: $test_name${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗ FAILED: $test_name${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo
}

echo "Phase 1: Testing existing system (baseline)"
run_test "Baseline - Old System Only" "old_only" "test_old_only_mode"

echo "Phase 2: Testing challenging cases with validation mode"
run_test "Complex Expect Struct" "validate_both" "test_complex_expect_struct_migration"
run_test "Bidirectional Custom Functions" "validate_both" "test_bidirectional_custom_functions"
run_test "Transparent Error Modes" "validate_both" "test_transparent_with_error_modes"
run_test "Collection Error Strategies" "validate_both" "test_collection_error_strategies"

echo "Phase 3: Comprehensive validation"
run_test "Comprehensive Migration Validation" "validate_both" "test_comprehensive_migration_validation"

echo "Phase 4: Testing fallback mode"
run_test "New with Fallback Mode" "new_with_fallback" "test_new_with_fallback_mode"

echo "=== MIGRATION TEST RESULTS ==="
echo -e "Total tests: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL MIGRATION TESTS PASSED!${NC}"
    echo "The new system is compatible with the existing implementation."
    echo
    echo "Next steps:"
    echo "1. Review any strategy differences in the comprehensive test"
    echo "2. Fix any identified issues"
    echo "3. Move to new_with_fallback mode for gradual rollout"
    echo "4. Eventually move to new_only mode"
else
    echo -e "${RED}❌ MIGRATION TESTS FAILED${NC}"
    echo "Issues found that need to be resolved before migration can proceed."
    echo
    echo "Troubleshooting steps:"
    echo "1. Review the detailed failure analysis above"
    echo "2. Check strategy mappings in field_conversion_strategy.rs"
    echo "3. Verify code generation logic in field_conversion_codegen.rs"
    echo "4. Run specific failing tests with RUST_LOG=debug for more details"
    exit 1
fi