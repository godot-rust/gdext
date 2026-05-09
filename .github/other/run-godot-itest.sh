#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Runs Godot integration tests and sets an outcome environment variable.
# Called from the workspace root; changes to itest/godot before running Godot.
#
# Usage: run-godot-itest.sh <godot-extra-args> <log-file> <outcome-var> <failure-val> <abort-val> [user-args...]
#
#   godot-extra-args: extra args before '--headless', e.g. '' (normal run) or '-e' (editor mode)
#   log-file:         absolute path for the Godot output log file
#   outcome-var:      name of the GitHub Actions env var to update, e.g. OUTCOME or EDITOR_OUTCOME
#   failure-val:      value written to outcome-var on test failure, e.g. 'itest' or 'editor-itest'
#   abort-val:        value written to outcome-var on unrecoverable error, e.g. 'godot-runtime'
#   user-args:        optional remaining arguments passed after '--' to Godot

set -euo pipefail

godotExtraArgs="$1"
logFile="$2"
outcomeVar="$3"
failureVal="$4"
abortVal="$5"
shift 5

cd itest/godot

echo "${outcomeVar}=${failureVal}" >> "$GITHUB_ENV"

# Aborts immediately if Godot outputs certain keywords (would otherwise stall until CI runner times out).
# Explanation:
# * tee:      still output logs while scanning for errors
# * grep -q:  no output, use exit code 0 if found -> thus also &&
# * pkill:    stop Godot execution (since it hangs in headless mode); simple 'head -1' did not work as expected
#             since it's not available on Windows, use taskkill in that case.
# * exit:     the terminated process would return 143, but this is more explicit and future-proof
#
# --disallow-focus: fail if #[itest(focus)] is encountered, to prevent running only a few tests for full CI.
#
# shellcheck disable=SC2086 -- intentional word splitting for godotExtraArgs
$GDRUST_GODOT_BIN $godotExtraArgs --headless -- --disallow-focus "$@" 2>&1 \
| tee "$logFile" \
| tee >(grep -E "SCRIPT ERROR:|Can't open dynamic library|Error loading extension" -q && {
  printf "\n::error::godot-itest: unrecoverable Godot error, abort...\n";
  if [[ "$RUNNER_OS" == "Windows" ]]; then
    taskkill -f -im godot*
  else
    pkill godot
  fi
  echo "${outcomeVar}=${abortVal}" >> "$GITHUB_ENV"
  exit 2
})

echo "${outcomeVar}=success" >> "$GITHUB_ENV"
