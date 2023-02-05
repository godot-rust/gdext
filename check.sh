#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Small utility to run tests locally
# Similar to minimal-ci

# Note: at the moment, there is a lot of useless recompilation.
# This should be better once unit tests and #[cfg] are sorted out.

# No args specified: do everything
if [ "$#" -eq 0 ]; then
    args=("fmt" "clippy" "test" "itest")
else
    args=("$@")
fi

# --help menu
for arg in "${args[@]}"; do
    if [ "$arg" == "--help" ]; then
        echo "Usage: check.sh [<commands>]"
        echo ""
        echo "Each specified command will be run (until one fails)."
        echo "If no commands are specified, all checks are run (no doc; may take several minutes)."
        echo ""
        echo "Commands:"
        echo "    fmt           format code, fail if bad"
        echo "    clippy        validate clippy lints"
        echo "    test          run unit tests (no Godot)"
        echo "    itest         run integration tests (Godot)"
        echo "    doc           generate docs for 'godot' crate"
        echo "    dok           generate docs and open in browser"
        echo ""
        echo "Examples:"
        echo "    check.sh fmt clippy"
        echo "    check.sh"
        exit 0
    fi
done

# For integration tests
function findGodot() {
    # User-defined GODOT4_BIN
    if [ -n "$GODOT4_BIN" ]; then
        echo "Found GODOT4_BIN env var ($GODOT4_BIN)"
        godotBin="$GODOT4_BIN"

    #  Executable in path
    elif command -v godot4 &>/dev/null; then
        echo "Found 'godot4' executable"
        godotBin="godot4"

    # Special case for Windows when there is a .bat file
    # Also consider that 'cmd /c' would need 'cmd //c' (https://stackoverflow.com/q/21357813)
    elif
        godot4.bat --version
        [[ $? -eq 0 ]]
    then
        echo "Found 'godot4.bat' script"
        godotBin="godot4.bat"

    # Error case
    else
        echo "Godot executable not found"
        exit 2
    fi
}

#features="--features crate/feature"
features=""
cmds=()

for arg in "${args[@]}"; do
    case "$arg" in
    fmt)
        cmds+=("cargo fmt --all -- --check")
        ;;
    clippy)
        cmds+=("cargo clippy $features -- -D clippy::suspicious -D clippy::style -D clippy::complexity -D clippy::perf -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D warnings")
        ;;
    test)
        cmds+=("cargo test $features")
        ;;
    itest)
        findGodot
        
        cmds+=("cargo build -p itest")
        cmds+=("$godotBin --path itest/godot --headless")
        ;;
    doc)
        cmds+=("cargo doc --lib -p godot --no-deps $features")
        ;;
    dok)
        cmds+=("cargo doc --lib -p godot --no-deps $features --open")
        ;;
    *)
        echo "Unrecognized command '$arg'"
        exit 2
        ;;
    esac
done

RED='\033[1;31m'
GREEN='\033[1;36m'
END='\033[0m'
for cmd in "${cmds[@]}"; do
    echo "> $cmd"
    $cmd || {
        printf "$RED\n=========================="
        printf "\ngodot-rust checker FAILED."
        printf "\n==========================\n$END"
        exit 1
    }
done

printf "$GREEN\n=============================="
printf "\ngodot-rust checker SUCCESSFUL."
printf "\n==============================\n$END"
