#!/usr/bin/env bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

set -e

################################################################################
# Help text
################################################################################

HELP_TEXT="Usage: $0 <godot-binary-path>

Run benchmarks comparing the current branch against master.

Arguments:
    <godot-binary-path>    Path to a Godot release template binary

The script will verify the binary is a release export template and exit with
an error if ripgrep (rg) is not available.
"

################################################################################
# Constants
################################################################################

# Terminal color codes.
RED='\033[1;31m'
CYAN='\033[1;36m'
GREEN='\033[1;32m'
YELLOW='\033[1;33m'
END='\033[0m'

################################################################################
# Helper functions
################################################################################

# Drop-in replacement for `echo` that outputs to stderr and adds a newline.
function log() {
    echo "$@" >&2
}

# Echoes the given command to stderr, then executes it.
function run() {
    echo -n '>' >&2
    for arg in "$@"; do
        printf " %q" "$arg" >&2
    done
    echo >&2
    "$@"
}

# Extract benchmark results from log (just the lines, no headers)
function extract_benchmarks() {
    local logfile=$1
    # Extract benchmark timing lines from color.rs and mod.rs sections (with timing values in μs)
    rg '^\s+--\s+\S+.*μs' "$logfile" 2>/dev/null || echo ""
}

################################################################################
# Argument parsing
################################################################################

if [[ $# -ne 1 ]]; then
    echo "$HELP_TEXT"
    exit 2
fi

GODOT_BINARY="$1"

################################################################################
# Validation
################################################################################

log -ne "${YELLOW}=== Benchmark Script ===${END}\n"
log ""

# Verify ripgrep is available
if ! command -v rg &> /dev/null; then
    log -ne "${RED}ERROR: ripgrep (rg) is required but not found in PATH${END}\n"
    exit 1
fi

# Verify Godot binary exists
if [[ ! -f "$GODOT_BINARY" ]]; then
    log -ne "${RED}ERROR: Godot binary not found at: $GODOT_BINARY${END}\n"
    exit 1
fi
log "Using Godot binary: ${GODOT_BINARY##*/}"

# Godot outputs "Option legend (this build = release export template):" for --help if release. The --version doesn't always specify it.
if ! "$GODOT_BINARY" --help 2>/dev/null | rg -q "release export template"; then
    log ""
    log -ne "${YELLOW}WARNING: Provided Godot binary is a Debug build, not Release export template.${END}\n"
    log "Benchmark results may not be meaningful."
    log ""
fi

# Ensure this is run on another branch, to compare with master.
currentBranch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$currentBranch" == "master" ]]; then
    log -ne "${RED}ERROR: Currently on master; check out a different branch to compare with master.${END}\n"
    exit 1
fi
log -e "${GREEN}✓${END} Current branch: $currentBranch"

# Since we switch branches back and forth, it's safer to have a clean tree.
UNCOMMITTED=$(git status --porcelain | rg -v '^\?\?' | wc -l)
if [[ "$UNCOMMITTED" -gt 0 ]]; then
    log -ne "${RED}ERROR: You have uncommitted changes:${END}\n"
    git status --porcelain | rg -v '^\?\?' >&2
    log -ne "${YELLOW}Please commit or stash these changes before benchmarking.${END}\n"
    exit 1
fi
log -e "${GREEN}✓${END} No uncommitted changes"
log ""

# Create tmp directory for logs
mkdir -p ./tmp

################################################################################
# Phase 1: Current Branch
################################################################################

log -ne "${YELLOW}=== Phase 1: Current Branch ===${END}\n"
log -ne "Branch: ${GREEN}${currentBranch}${END}"
log ""

# Phase 1a: BALANCED
log -ne "${YELLOW}[1/4] Building BALANCED configuration...${END}\n"
cargo build -p itest --release 2>&1 | tail -5 >&2
log "Run BALANCED tests..."
run "$GODOT_BINARY" --path itest/godot --headless 2>&1 | tee ./tmp/bench_current_balanced.log > /dev/null
log -ne "${GREEN}✓${END} BALANCED complete"
log ""

# Phase 1b: DISENGAGED
log -ne "${YELLOW}[2/4] Building DISENGAGED configuration...${END}\n"
cargo build -p itest --release --features godot/safeguards-release-disengaged 2>&1 | tail -5 >&2
log "Run DISENGAGED tests..."
run "$GODOT_BINARY" --path itest/godot --headless 2>&1 | tee ./tmp/bench_current_disengaged.log > /dev/null
log -ne "${GREEN}✓${END} DISENGAGED complete"
log ""

################################################################################
# Phase 2: Master Branch
################################################################################

log -ne "${YELLOW}=== Phase 2: Master Branch ===${END}\n"

# Switch to master
log "Switch to master branch..."
git checkout master
MASTER_BRANCH=$(git rev-parse --abbrev-ref HEAD)
log -ne "${GREEN}✓${END} Now on: $MASTER_BRANCH"

# Restore benchmark files from the feature branch
log "Restore benchmark files from ${currentBranch}..."
git restore --source="$currentBranch" -- itest/rust/src/benchmarks/
log -ne "${GREEN}✓${END} Benchmarks restored"
log ""

# Phase 2a: DISENGAGED (opposite order for cache optimization)
log -ne "${YELLOW}[3/4] Building DISENGAGED configuration...${END}\n"
cargo build -p itest --release --features godot/safeguards-release-disengaged 2>&1 | tail -5 >&2
log "Run DISENGAGED tests..."
run "$GODOT_BINARY" --path itest/godot --headless 2>&1 | tee ./tmp/bench_master_disengaged.log > /dev/null
log -ne "${GREEN}✓${END} DISENGAGED complete"
log ""

# Phase 2b: BALANCED
log -ne "${YELLOW}[4/4] Building BALANCED configuration...${END}\n"
cargo build -p itest --release 2>&1 | tail -5 >&2
log "Run BALANCED tests..."
run "$GODOT_BINARY" --path itest/godot --headless 2>&1 | tee ./tmp/bench_master_balanced.log > /dev/null
log -ne "${GREEN}✓${END} BALANCED complete"
log ""

# Clean up restored benchmark files before switching back
log "Clean up benchmark files..."
git restore -- itest/rust/src/benchmarks/
# Remove files that were added on the feature branch but don't exist on master
git diff --name-only --diff-filter=A master "$currentBranch" -- itest/rust/src/benchmarks/ | xargs rm
log -ne "${GREEN}✓${END} Cleaned up"
log ""

# Switch back to original branch
log "Switch back to ${currentBranch}..."
git checkout -
RESTORED_BRANCH=$(git rev-parse --abbrev-ref HEAD)
log -ne "${GREEN}✓${END} Restored to: $RESTORED_BRANCH"
log ""

################################################################################
# Results extraction
################################################################################

log -ne "${YELLOW}=== Extract Results ===${END}\n"

# Generate bench_results.md
resultsFile="bench_results.md"
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")

# Extract benchmark data
log "Parse benchmark results..."
log ""

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
# Collect rows first, then sort them alphabetically
rows=""
while IFS= read -r line; do
    [[ -z "$line" ]] && continue

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
    rows+="$(printf "| \`%s\` | %s | %s | %s | %s |\n" "$name" "$cb_min" "$mb_min" "$cd_min" "$md_min")"
done <<< "$CURRENT_BALANCED_LINES"

# Sort rows alphabetically by benchmark name (second column, using | as delimiter)
printf '%s\n' "$rows" | sort -t '|' -k 2 >> "$resultsFile"

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

log -ne "${GREEN}✓${END} Results written to $resultsFile"
log ""
log -ne "${CYAN}=============================="
log -ne "\ngodot-rust: benchmark COMPLETE."
log -ne "\n==============================${END}\n"
log ""
log "Results saved to: $resultsFile"
log "Log files saved to: ./tmp/"
