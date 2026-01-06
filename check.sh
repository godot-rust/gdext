#!/usr/bin/env bash
# Copyright (c) godot-rust; Bromeon and contributors.
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
    test          run unit tests (no Godot needed)
    itest         run integration tests (from within Godot)
    clippy        validate clippy lints
    klippy        validate + fix clippy
    doc           generate docs for 'godot' crate
    dok           generate docs and open in browser

Options:
    -h, --help               print this help text
    --double                 run check with double-precision (implies 'api-custom' feature)
    -f, --filter <arg>       only run integration tests which contain any of the
                             args (comma-separated). requires itest.
    -a, --api-version <ver>  specify the Godot API version to use (e.g. 4.3, 4.3.1).

Examples:
    check.sh fmt clippy
    check.sh
    check.sh --double clippy
    check.sh test itest -f variant,static
    RUSTUP_TOOLCHAIN=nightly check.sh
EOF

# Terminal color codes.
RED='\033[1;31m'
CYAN='\033[1;36m'
YELLOW='\033[1;33m'
END='\033[0m'

################################################################################
# Helper functions
################################################################################

# Drop-in replacement for `echo` that outputs to stderr and adds a newline.
function log() {
    echo "$@" >&2
}

# Converts a x.x.x version string to a feature string.
# e.g. 4.3.0 -> godot/api-4-3, 4.3.1 -> godot/api-4-3-1
function version_to_feature() {
    echo "godot/api-$(echo "$1" | sed 's/\./-/g' | sed 's/-0$//')"
}

# Validates that the given string is a valid x.x.x version string.
# Allow for .0 to be dropped (e.g. 4.3 is equivalent to 4.3.0).
function validate_version_string() {
    if [[ ! "$1" =~ ^4\.[0-9]+(\.[0-9]+)?$ ]]; then
        log "Invalid Godot version string '$1'."
        log "The version string should be in the form 'x.x.x' or 'x.x' and the major version should be at least 4."
        exit 2
    fi
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
    fi

    # User-defined GODOT4_BIN.
    if [[ -n "$GODOT4_BIN" ]]; then
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
    # Run rustfmt in nightly toolchain if available.
    if [[ $(rustup toolchain list) =~ nightly ]]; then
        run cargo +nightly fmt --all -- --check
    else
        log -e "${YELLOW}Warning: nightly toolchain not found; stable rustfmt might not pass CI.${END}"
        run cargo fmt --all -- --check
    fi
}

function cmd_clippy() {
    run cargo clippy --all-targets "${extraCargoArgs[@]}" -- \
        -D clippy::suspicious \
        -D clippy::style \
        -D clippy::complexity \
        -D clippy::perf \
        -D clippy::dbg_macro \
        -D clippy::todo \
        -D clippy::unimplemented \
        -D warnings
}

function cmd_klippy() {
    run cargo clippy --fix --all-targets "${extraCargoArgs[@]}" -- \
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
        run cargo build -p itest "${extraCargoArgs[@]}" || return 1

    # Logic to abort immediately if Godot outputs certain keywords (would otherwise fail only in CI).
    # Keep in sync with: .github/composite/godot-itest/action.yml (steps "Run Godot integration tests" and "Check for memory leaks").

    local logFile
    logFile=$(mktemp)

    cd itest/godot

    # Explanation:
    # * tee:      still output logs while scanning for errors.
    # * grep -q:  no output, use exit code 0 if found -> thus also &&.
    # * pkill:    stop Godot execution (since it hangs in headless mode); simple 'head -1' did not work as expected
    #             since it's not available on Windows, use taskkill in that case.
    # * exit:     the terminated process would return 143, but this is more explicit and future-proof.
    "$godotBin" --headless -- "[${extraArgs[@]}]" 2>&1 \
    | tee "$logFile" \
    | tee >(grep -E "SCRIPT ERROR:|Can't open dynamic library" -q && {
      printf "\n${RED}Error: Script or dlopen error, abort...${END}\n" >&2;
      # Unlike CI; do not kill processes called "godot" on user machine.
      exit 2
    })

    local exitCode=$?

    # Check for unrecoverable errors in log.
    if grep -qE "SCRIPT ERROR:|Can't open dynamic library" "$logFile"; then
      log -e "\n${RED}Error: Unrecoverable Godot error detected in logs.${END}"
      exitCode=2
    fi

    # Check for memory leaks.
    if grep -q "ObjectDB instances leaked at exit" "$logFile"; then
      log -e "\n${RED}Error: Memory leak detected.${END}"
      exitCode=3
    fi

    rm -f "$logFile"
    cd ../..

    return $exitCode
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

# By default, disable `codegen-full` to reduce compile times and prevent flip-flopping between
# `itest` compilations and `check.sh` runs. Note that this means some runs are different from CI.
extraCargoArgs=("--no-default-features")
cmds=()
extraArgs=()
apiVersion=""

while [[ $# -gt 0 ]]; do
    arg="$1"
    case "$arg" in
        -h | --help | help)
            echo "$HELP_TEXT"
            exit 0
            ;;
        --use-serde)
            extraCargoArgs+=("--features" "serde")
            ;;
        --double)
            extraCargoArgs+=("--features" "godot/double-precision,godot/api-custom")
            ;;
        fmt | test | itest | clippy | klippy | doc | dok)
            cmds+=("$arg")
            ;;
        -f | --filter)
            if [[ "${cmds[*]}" =~ itest ]]; then
                if [[ -z "$2" ]]; then
                    log "-f/--filter requires an argument."
                    exit 2
                fi

                extraArgs+=("$2")
                shift
            else
                log "-f/--filter requires 'itest' to be specified as a command."
                exit 2
            fi
            ;;
        -a | --api-version)
            if [[ -z "$2" || "$2" == -* ]]; then
                log "-a/--api-version requires an argument."
                exit 2
            fi

            apiVersion="$2"
            validate_version_string "$apiVersion"

            apiFeature=$(version_to_feature "$apiVersion")
            extraCargoArgs+=("--features" "$apiFeature")

            log "Using Godot API version $apiVersion with feature $apiFeature"

            # Remove "clippy" from the default commands if the API version is specified
            # since it can produce unexpected errors.
            DEFAULT_COMMANDS=("${DEFAULT_COMMANDS[@]/clippy}")

            shift
            ;;
        *)
            log "Unrecognized argument '$arg'. Use '$0 --help' to see what's available."
            exit 2
            ;;
    esac
    shift
done

# Default if no commands are explicitly given.
if [[ ${#cmds[@]} -eq 0 ]]; then
    cmds=("${DEFAULT_COMMANDS[@]}")
fi

# Filter out any empty strings and note if clippy will be run.
filtered_commands=()
runClippy=0
for cmd in "${cmds[@]}"; do
    if [[ -n "$cmd" ]]; then
        filtered_commands+=("$cmd")

        if [[ "$cmd" == "clippy" ]]; then
            runClippy=1
        fi
    fi
done
cmds=("${filtered_commands[@]}")

# Display warning about using clippy if an API version was provided.
if [[ "${#apiFeature[@]}" -ne 0 ]]; then
    log
    # Show different warning depending on if clippy was explicitly requested.
    if [[ "$runClippy" -eq 1 ]]; then
        log -e "${YELLOW}Warning: Clippy may produce unexpected errors when testing against a specific API version.${END}"
    else
        log -e "${YELLOW}Warning: Clippy is disabled by default when using a specific Godot API version.${END}"
    fi
    log -e "${YELLOW}For more information, see ${CYAN}https://github.com/godot-rust/gdext/pull/1016#issuecomment-2629002047${END}"
    log
fi

log "Checks to run: ${cmds[*]}"
log

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
        log -ne "$RED\n=========================="
        log -ne "\ngodot-rust: checks FAILED."
        log -ne "\n==========================\n$END"
        log -ne "\nTotal duration: $elapsed.\n"
        exit 1
    }
done

compute_elapsed
log -ne "$CYAN\n=============================="
log -ne "\ngodot-rust: checks SUCCESSFUL."
log -ne "\n==============================\n$END"
log -ne "\nTotal duration: $elapsed.\n"

# If invoked with sh instead of bash, pressing Up arrow after executing `sh check.sh` may cause a `[A` to appear.
# See https://unix.stackexchange.com/q/103608.
