#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Small utility to run tests locally
# Similar to minimal-ci

# Note: at the moment, there is some useless recompilation, which could be improved.

################################################################################
# Constants
################################################################################

# Commands to run (in that order) if none are given on the command line.
DEFAULT_COMMANDS=("fmt" "clippy" "test" "itest")

# Store help text in a variable $HELP_TEXT so we don't need weird indentation later on.
read -r -d '' HELP_TEXT <<EOF
Usage: check.sh [OPTION|COMMAND...]

Each specified command will be run (until one fails).
If no commands are specified, the following commands will be run:
    ${DEFAULT_COMMANDS[@]}

Commands:
    fmt           format code, fail if bad
    clippy        validate clippy lints
    test          run unit tests (no Godot needed)
    itest         run integration tests (from within Godot)
    doc           generate docs for 'godot' crate
    dok           generate docs and open in browser

Options:
    -h, --help    print this help text
    --double      run check with double-precision

Examples:
    check.sh fmt clippy
    check.sh
    check.sh --double clippy
    RUSTUP_TOOLCHAIN=nightly check.sh
EOF

# Terminal color codes.
RED='\033[1;31m'
CYAN='\033[1;36m'
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
    # https://stackoverflow.com/a/76153233/14637
    echo -n '>' >&2
    for arg in "$@"; do
        printf " %q" "$arg" >&2
    done
    echo >&2
    "$@"
}

# Finds the Godot binary and stores its path in $godotBin. Logs an error and returns with nonzero
# exit status if not found.
function findGodot() {
    # $godotBin previously detected.
    if [[ -v godotBin ]]; then
        return

    # User-defined GODOT4_BIN.
    elif [[ -n "$GODOT4_BIN" ]]; then
        log "Using environment variable GODOT4_BIN=$(printf %q "$GODOT4_BIN")"
        godotBin="$GODOT4_BIN"

    # Executable in path.
    elif command -v godot4 >/dev/null; then
        log "Found 'godot4' executable"
        godotBin="godot4"

    # Special case for Windows when there is a .bat file.
    # Also consider that 'cmd /c' would need 'cmd //c' (https://stackoverflow.com/q/21357813)
    elif godot4.bat --version 2>/dev/null; then
        log "Found 'godot4.bat' script"
        godotBin="godot4.bat"

    # This should come last: only use this as a last resort as `godot` may refer to a 
    # Godot 3.x installation.
    elif command -v godot >/dev/null; then
        # Check if `godot` actually is Godot 4.x
        godotVersion="$(command godot --version)"
        if [[ "$godotVersion" =~ ^4\. ]]; then
            log "Found 'godot' executable with version $godotVersion"
            godotBin="godot"
        else
            log "Found 'godot' executable, but it has incompatible version $godotVersion"
            return 1
        fi

    # Error case.
    else
        log "Godot executable not found; try setting GODOT4_BIN to the full path to the executable"
        return 1
    fi
}

################################################################################
# Commands
################################################################################

# Surrogate namespacing: all commands are prefixed with `cmd_` to avoid confusion with shell
# builtins like `test`.

function cmd_fmt() {
    run cargo fmt --all -- --check
}

function cmd_clippy() {
    run cargo clippy "${extraCargoArgs[@]}" -- \
        -D clippy::suspicious \
        -D clippy::style \
        -D clippy::complexity \
        -D clippy::perf \
        -D clippy::dbg_macro \
        -D clippy::todo \
        -D clippy::unimplemented \
        -D warnings
}

function cmd_test() {
    run cargo test "${extraCargoArgs[@]}"
}

function cmd_itest() {
    findGodot && \
        run cargo build -p itest "${extraCargoArgs[@]}" && \
        run "$godotBin" --path itest/godot --headless
}

function cmd_doc() {
    run cargo doc --lib -p godot --no-deps "${extraCargoArgs[@]}"
}

function cmd_dok() {
    run cargo doc --lib -p godot --no-deps "${extraCargoArgs[@]}" --open
}

################################################################################
# Argument parsing
################################################################################

# By default, disable `codegen-full` to reduce compile times and prevent flip-flopping
# between `itest` compilations and `check.sh` runs.
extraCargoArgs=("--no-default-features")
cmds=()

for arg in "$@"; do
    case "$arg" in
        -h | --help | help)
            echo "$HELP_TEXT"
            exit 0
            ;;
        --double)
            extraCargoArgs+=("--features" "godot/double-precision")
            ;;
        fmt | clippy | test | itest | doc | dok)
            cmds+=("$arg")
            ;;
        *)
            log "Unrecognized argument '$arg'. Use '$0 --help' to see what's available."
            exit 2
            ;;
    esac
done

# Default if no commands are explicitly given.
if [[ ${#cmds[@]} -eq 0 ]]; then
    cmds=("${DEFAULT_COMMANDS[@]}")
fi

################################################################################
# Execution and summary
################################################################################

function compute_elapsed() {
    local total=$SECONDS
    local min=$(("$total" / 60))
    if [[ "$min" -gt 0 ]]; then
        min="${min}min "
    else
        min=""
    fi
    local sec=$(("$total" % 60))

    # Don't use echo and call it with $(compute_elapsed), it messes with stdout
    elapsed="${min}${sec}s"
}

for cmd in "${cmds[@]}"; do
    "cmd_${cmd}" || {
        compute_elapsed
        log -ne "$RED\n====================="
        log -ne "\ngdext: checks FAILED."
        log -ne "\n=====================\n$END"
        log -ne "\nTotal duration: $elapsed.\n"
        exit 1
    }
done

compute_elapsed
log -ne "$CYAN\n========================="
log -ne "\ngdext: checks SUCCESSFUL."
log -ne "\n=========================\n$END"
log -ne "\nTotal duration: $elapsed.\n"

# If invoked with sh instead of bash, pressing Up arrow after executing `sh check.sh` may cause a `[A` to appear.
# See https://unix.stackexchange.com/q/103608.
