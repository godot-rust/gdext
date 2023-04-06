#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Small utility to run tests locally
# Similar to minimal-ci

# Note: at the moment, there is some useless recompilation, which could be improved.

# --help menu
for arg in $@; do
    if [ "$arg" == "--help" ]; then
        echo "Usage: check.sh [--double] [<commands>]"
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
        echo "Options:"
        echo "    --double      run check with double-precision"
        echo ""
        echo "Examples:"
        echo "    check.sh fmt clippy"
        echo "    check.sh"
        echo "    check.sh --double clippy"
        exit 0
    fi
done

firstArg=1
toolchain=""
extraArgs=()

if [[ "$1" == "--double" ]]; then
    firstArg=2
    extraArgs+=("--features double-precision")
fi

args=()

for arg in "${@:$firstArg}"; do
    args+=("$arg")
done

# No args specified: do everything
if [ ${#args[@]} -eq 0 ]; then
    args=("fmt" "clippy" "test" "itest")
fi

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

    # This should come last: only use this as a last resort as usually `godot`
    # refers to a Godot 3.x installation.
    elif command -v godot &>/dev/null; then
        # Check if `godot` actually is Godot 4.x
        if godot --version | grep -qE "^4\\."; then
            echo "Found 'godot' executable with version $(godot --version)"
            godotBin="godot"
        else
            echo "Found 'godot' executable, but it has the incompatible version $(godot --version)"
            exit 2
        fi

    # Error case
    else
        echo "Godot executable not found"
        exit 2
    fi
}

cmds=()
extraArgs="${extraArgs[@]}"

for arg in "${args[@]}"; do
    case "$arg" in
    fmt)
        cmds+=("cargo $toolchain fmt --all -- --check")
        ;;
    clippy)
        cmds+=("cargo $toolchain clippy $extraArgs -- -D clippy::suspicious -D clippy::style -D clippy::complexity -D clippy::perf -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D warnings")
        ;;
    test)
        cmds+=("cargo $toolchain test $extraArgs")
        ;;
    itest)
        findGodot

        cmds+=("cargo $toolchain build -p itest $extraArgs")
        cmds+=("$godotBin --path itest/godot --headless")
        ;;
    doc)
        cmds+=("cargo $toolchain doc --lib -p godot --no-deps $extraArgs")
        ;;
    dok)
        cmds+=("cargo $toolchain doc --lib -p godot --no-deps $extraArgs --open")
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
        printf "$RED\n====================="
        printf "\ngdext: checks FAILED."
        printf "\n=====================\n$END"
        exit 1
    }
done

printf "$GREEN\n========================="
printf "\ngdext: checks SUCCESSFUL."
printf "\n=========================\n$END"
