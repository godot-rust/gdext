#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Checks a Godot integration test log file for memory leaks and sets an outcome variable.
# Usage: check-godot-leaks.sh <log-file> <outcome-var> <leak-val>
#
#   log-file:    absolute path to the Godot output log file
#   outcome-var: name of the GitHub Actions env var to update, e.g. OUTCOME or EDITOR_OUTCOME
#   leak-val:    value written to outcome-var if a leak is detected, e.g. 'godot-leak'

set -euo pipefail

logFile="$1"
outcomeVar="$2"
leakVal="$3"

if grep -q "ObjectDB instances leaked at exit" "$logFile"; then
  echo "${outcomeVar}=${leakVal}" >> "$GITHUB_ENV"
  exit 3
fi
