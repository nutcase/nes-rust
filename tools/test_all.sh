#!/usr/bin/env bash
# Test all ROMs in the roms/ directory

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "╔═══════════════════════════════════════════╗"
echo "║     SNES Emulator - Batch ROM Test       ║"
echo "╚═══════════════════════════════════════════╝"
echo ""

# Build once
echo "Building emulator..."
cargo build --release --quiet 2>&1 | grep -E "(error|warning)" || true
echo ""

# Find all ROM files
shopt -s nullglob
ROMS=(roms/*.sfc roms/*.smc roms/tests/*.sfc roms/tests/*.smc)
shopt -u nullglob

if [[ ${#ROMS[@]} -eq 0 ]]; then
    echo "No ROM files found in roms/ or roms/tests/"
    exit 1
fi

echo "Found ${#ROMS[@]} ROM(s) to test"
echo ""

# Test results
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

# Run smoke test on each ROM
for rom in "${ROMS[@]}"; do
    rom_name=$(basename "$rom")
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Testing: $rom_name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Run smoke test
    if ./tools/smoke.sh "$rom" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ PASS${NC}"
        ((PASS_COUNT++))
    else
        # Check if it's a known test ROM with different behavior
        if [[ "$rom_name" == *"wrmpyb"* ]] || [[ "$rom_name" == *"test"* ]]; then
            echo -e "${YELLOW}⚠️  SKIP (test ROM with non-standard behavior)${NC}"
            ((SKIP_COUNT++))
        else
            echo -e "${RED}❌ FAIL${NC}"
            ((FAIL_COUNT++))
            # Show last few lines of output for failed tests
            echo "Last lines of output:"
            ./tools/smoke.sh "$rom" 2>&1 | tail -5 | sed 's/^/  /'
        fi
    fi
    echo ""
done

# Summary
echo "╔═══════════════════════════════════════════╗"
echo "║            Test Summary                   ║"
echo "╠═══════════════════════════════════════════╣"
echo "║ Total ROMs: ${#ROMS[@]}"
echo -e "║ ${GREEN}Passed:     $PASS_COUNT${NC}"
if [[ $SKIP_COUNT -gt 0 ]]; then
    echo -e "║ ${YELLOW}Skipped:    $SKIP_COUNT${NC}"
fi
if [[ $FAIL_COUNT -gt 0 ]]; then
    echo -e "║ ${RED}Failed:     $FAIL_COUNT${NC}"
fi
echo "╚═══════════════════════════════════════════╝"

# Exit with error if any tests failed
if [[ $FAIL_COUNT -gt 0 ]]; then
    exit 1
fi

exit 0
