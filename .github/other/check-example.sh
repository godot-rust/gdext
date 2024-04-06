#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Tests an example with Godot, by first importing it in the editor and letting it run for some time.
# Input: $exampleCrate

set -euo pipefail

# Opening in editor can take a while (import reosurces, load extensions for the first time, ...).
# Unlike EXAMPLE_TIMEOUT, this is an upper bound, after which CI job fails, so not the entire time is necessarily spent.
EXAMPLE_TIMEOUT=5
EDITOR_TIMEOUT=30 # already encountered 20s on macOS CI.

example="$1"
if [ -z "$example" ]; then
    echo "::error::required argument 'example' missing."
    exit 1
fi

PRE="# EXAMPLE $example |"
dir="examples/$example/godot"
logfile="stderr-$example.log"

# In .gdextension file, use paths to release artifacts.
sed -i'.bak' "s!/debug/!/release/!g" "$dir/rust.gdextension"

echo "$PRE Godot binary: $GODOT4_BIN"

# Open in editor to import resources (--quit exits once loaded).
echo "$PRE Briefly open Godot editor, to import resources..."
timeout "$EDITOR_TIMEOUT"s "$GODOT4_BIN" -e --headless --path "$dir" --quit || {
    echo "::error::$PRE Godot editor failed to open in time."
    exit 1
}

# Could also use `timeout -s9`, but there were some issues and this gives more control.
echo "$PRE Run example..."
$GODOT4_BIN --headless --path "$dir" 2> "$logfile" &
pid=$!

# Keep open for some time (even just main menu or so).
sleep $EXAMPLE_TIMEOUT

# Terminate (once shutdown in dodge-the-creeps is graceful without error messages, can also omit `-9`).
echo "$PRE Terminate Godot process $pid after ${EXAMPLE_TIMEOUT}s..."
kill $pid
wait $pid || true

echo "$PRE Output in stderr:"
echo "----------------------------------------------------"
cat "$logfile"
echo "----------------------------------------------------"

if grep --quiet "ERROR:" "$logfile"; then
    echo "::error::$PRE Godot engine encountered errors while running."
    exit 1
fi

echo "$PRE Example ran successfully."
