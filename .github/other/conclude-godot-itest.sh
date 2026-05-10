#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Evaluates an integration-test outcome and writes a GITHUB_STEP_SUMMARY entry on failure.
# Usage: conclude-godot-itest.sh <outcome> [<mode>]
#
#   outcome: the outcome value to evaluate, e.g. 'success', 'itest', 'godot-editor-runtime', ''
#   mode:    optional mode label, e.g. '' (normal run) or 'editor' (editor mode)
#
# Outcome values use consistent suffixes so one case block covers both modes:
#   *-runtime  godot-runtime | godot-editor-runtime  -- unrecoverable Godot error
#   *-leak     godot-leak    | godot-editor-leak     -- memory leak detected
#   *itest     itest         | editor-itest          -- test suite failure

set -euo pipefail

outcome="$1"
mode="${2:-}"

# "Godot" for normal mode, "Godot editor" for editor mode.
label="Godot${mode:+ $mode}"

case "$outcome" in
  "success" | "")
    ;;

  *-runtime)
    echo "### :x: $label runtime error" >> "$GITHUB_STEP_SUMMARY"
    echo "$GODOT_BUILT_FROM" >> "$GITHUB_STEP_SUMMARY"
    echo "Aborted due to an error during $label execution." >> "$GITHUB_STEP_SUMMARY"
    exit 2
    ;;

  *-leak)
    echo "### :x: Memory leak${mode:+ ($mode)}" >> "$GITHUB_STEP_SUMMARY"
    echo "$GODOT_BUILT_FROM" >> "$GITHUB_STEP_SUMMARY"
    echo "$label integration tests cause memory leaks." >> "$GITHUB_STEP_SUMMARY"
    exit 3
    ;;

  *itest)
    echo "### :x: $label integration tests failed" >> "$GITHUB_STEP_SUMMARY"
    echo "$GODOT_BUILT_FROM" >> "$GITHUB_STEP_SUMMARY"
    exit 4
    ;;

  "header-diff")
    # already written.
    ;;

  *)
    echo "### :x: Unknown${mode:+ $mode} error occurred" >> "$GITHUB_STEP_SUMMARY"
    echo "$GODOT_BUILT_FROM" >> "$GITHUB_STEP_SUMMARY"
    exit 5
    ;;
esac
