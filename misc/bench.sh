#!/bin/bash

set -e

# Godot binary (release template for benchmarking)
godotBinary=/home/jan/gamedev/godot/bin/godot.linuxbsd.template_release.x86_64

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Benchmark Script ===${NC}"
echo ""

# Step 1: Verify we're not on master
currentBranch=$(git branch | grep '^\*' | sed 's/^\* //')
if [ "$currentBranch" == "master" ]; then
    echo -e "${RED}ERROR: You are currently on the master branch. Please check out a different branch first.${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Current branch: $currentBranch"

# Step 2: Check for uncommitted changes
UNCOMMITTED=$(git status --porcelain | grep -v '??' | wc -l)
if [ "$UNCOMMITTED" -gt 0 ]; then
    echo -e "${YELLOW}Warning: You have uncommitted changes:${NC}"
    git status --porcelain | grep -v '??'
    echo -e "${YELLOW}Please commit or stash these changes before benchmarking.${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} No uncommitted changes"

# Create tmp directory for logs
mkdir -p ./tmp

echo ""
echo -e "${YELLOW}=== Phase 1: Current Branch ===${NC}"
echo -e "${GREEN}Branch: $currentBranch${NC}"
echo ""

# Phase 1a: BALANCED
echo -e "${YELLOW}[1/4] Building BALANCED configuration...${NC}"
cargo build -p itest --release 2>&1 | tail -5
echo -e "Run BALANCED tests..."
$godotBinary --path itest/godot --headless 2>&1 | tee ./tmp/bench_current_balanced.log > /dev/null
echo -e "${GREEN}✓${NC} BALANCED complete"
echo ""

# Phase 1b: DISENGAGED
echo -e "${YELLOW}[2/4] Building DISENGAGED configuration...${NC}"
cargo build -p itest --release --features godot/safeguards-release-disengaged 2>&1 | tail -5
echo -e "Run DISENGAGED tests..."
$godotBinary --path itest/godot --headless 2>&1 | tee ./tmp/bench_current_disengaged.log > /dev/null
echo -e "${GREEN}✓${NC} DISENGAGED complete"

echo ""
echo -e "${YELLOW}=== Phase 2: Master Branch ===${NC}"

# Switch to master
echo -e "${YELLOW}Switching to master branch...${NC}"
git checkout master
MASTER_BRANCH=$(git branch | grep '^\*' | sed 's/^\* //')
echo -e "${GREEN}✓${NC} Now on: $MASTER_BRANCH"

# Restore benchmark files from the feature branch
echo -e "Restore benchmark files from $currentBranch..."
git restore --source=$currentBranch -- itest/rust/src/benchmarks/
echo -e "${GREEN}✓${NC} Benchmarks restored"
echo ""

# Force rebuild of itest package to pick up restored benchmarks
echo -e "Clean itest package to force rebuild..."
cargo clean -p itest
echo -e "${GREEN}✓${NC} Package cleaned"
echo ""

# Phase 2a: DISENGAGED (opposite order for cache optimization)
echo -e "${YELLOW}[3/4] Building DISENGAGED configuration...${NC}"
cargo build -p itest --release --features godot/safeguards-release-disengaged 2>&1 | tail -5
echo -e "Run DISENGAGED tests..."
$godotBinary --path itest/godot --headless 2>&1 | tee ./tmp/bench_master_disengaged.log > /dev/null
echo -e "${GREEN}✓${NC} DISENGAGED complete"
echo ""

# Phase 2b: BALANCED
echo -e "${YELLOW}[4/4] Building BALANCED configuration...${NC}"
cargo build -p itest --release 2>&1 | tail -5
echo -e "Run BALANCED tests..."
$godotBinary --path itest/godot --headless 2>&1 | tee ./tmp/bench_master_balanced.log > /dev/null
echo -e "${GREEN}✓${NC} BALANCED complete"
echo ""

# Clean up restored benchmark files before switching back
echo -e "Clean up benchmark files..."
git restore -- itest/rust/src/benchmarks/
# Remove files that were added on the feature branch but don't exist on master
git diff --name-only --diff-filter=A master $currentBranch -- itest/rust/src/benchmarks/ | xargs rm
echo -e "${GREEN}✓${NC} Cleaned up"
echo ""

# Switch back to original branch
echo ""
echo -e "${YELLOW}Switch back to $currentBranch...${NC}"
git checkout -
RESTORED_BRANCH=$(git branch | grep '^\*' | sed 's/^\* //')
echo -e "${GREEN}✓${NC} Restored to: $RESTORED_BRANCH"

echo ""
echo -e "${YELLOW}=== Extracting Results ===${NC}"

# Function to extract benchmark results from log (just the lines, no headers)
extract_benchmarks() {
    local logfile=$1
    # Extract benchmark timing lines from color.rs and mod.rs sections (with timing values in μs)
    rg '^\s+--\s+\S+.*μs' "$logfile" 2>/dev/null || echo ""
}

# Generate bench_results.md
resultsFile="bench_results.md"
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")

# Extract benchmark data
echo -e "${YELLOW}Parsing benchmark results...${NC}"

# Extract just the benchmark lines
CURRENT_BALANCED_LINES=$(extract_benchmarks "./tmp/bench_current_balanced.log")
CURRENT_DISENGAGED_LINES=$(extract_benchmarks "./tmp/bench_current_disengaged.log")
MASTER_BALANCED_LINES=$(extract_benchmarks "./tmp/bench_master_balanced.log")
MASTER_DISENGAGED_LINES=$(extract_benchmarks "./tmp/bench_master_disengaged.log")

# Generate results file with side-by-side comparison
cat > "$resultsFile" << EOF
# Benchmark Comparison: $currentBranch vs master

**Date:** $TIMESTAMP  \
**Current branch:** $currentBranch  \
**Base branch:** master

---

## Benchmark Results

| Benchmark | Current BALANCED | Master BALANCED | Current DISENGAGED | Master DISENGAGED |
|-----------|-----------------|-------------------|-----------------|-------------------|
EOF

# Create temporary files for parsing
tmpdir=$(mktemp -d)
echo "$CURRENT_BALANCED_LINES" > "$tmpdir/cb.txt"
echo "$CURRENT_DISENGAGED_LINES" > "$tmpdir/cd.txt"
echo "$MASTER_BALANCED_LINES" > "$tmpdir/mb.txt"
echo "$MASTER_DISENGAGED_LINES" > "$tmpdir/md.txt"

# Parse each benchmark line and find matches in other configs
while IFS= read -r line; do
    [ -z "$line" ] && continue

    # Extract benchmark name and min value from current balanced
    name=$(echo "$line" | sed 's/^[[:space:]]*--[[:space:]]*//; s/[[:space:]]*\.\.\..*//')
    cb_min=$(echo "$line" | awk '{print $(NF-1)}')

    # Find matching lines in other configs using rg (case-insensitive, word-boundary search)
    cd_line=$(rg -F "$name" "$tmpdir/cd.txt" 2>/dev/null | head -1)
    mb_line=$(rg -F "$name" "$tmpdir/mb.txt" 2>/dev/null | head -1)
    md_line=$(rg -F "$name" "$tmpdir/md.txt" 2>/dev/null | head -1)

    # Extract min values
    cd_min=$(echo "$cd_line" | awk '{print $(NF-1)}')
    mb_min=$(echo "$mb_line" | awk '{print $(NF-1)}')
    md_min=$(echo "$md_line" | awk '{print $(NF-1)}')

    # Add row if all values found -- check no longer needed, as change from branch is backported (git restore) to master.
    #if [ -n "$cb_min" ] && [ -n "$cd_min" ] && [ -n "$mb_min" ] && [ -n "$md_min" ]; then
    printf "| \`%s\` | %s | %s | %s | %s |\n" "$name" "$cb_min" "$mb_min" "$cd_min" "$md_min" >> "$resultsFile"
    #fi
done <<< "$CURRENT_BALANCED_LINES"

# Cleanup temp files
rm -rf "$tmpdir"

cat >> "$resultsFile" << EOF

---

## Notes

- All benchmarks extracted from integration test output.
- Timing values shown are min measurements.
- Both Godot engine and Rust code use release builds.
- Review variations carefully; some variance is expected.

EOF

echo -e "${GREEN}✓${NC} Results written to $resultsFile"
echo ""
echo -e "${GREEN}=== Benchmark Complete ===${NC}"
echo ""
echo "Results saved to: $resultsFile"
echo "Log files saved to: ./tmp/"
