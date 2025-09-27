#!/usr/bin/env bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

rel="."

# If argument is 'api-custom', set 'cargoArgs' to use that feature. If 'stable', set 'cargoArgs' to empty string. Otherwise, error.
if [[ $1 == "api-custom" ]]; then
  cargoArgs="--features godot/api-custom"
elif [[ $1 == "stable" ]]; then
  cargoArgs=""
elif [[ $1 == api-4-* ]]; then
  cargoArgs="--features godot/$1"
else
  echo "[Bash]      Error: Unknown argument '$1'. Expected 'stable', 'api-custom' or 'api-4.*'."
  exit 1
fi

# Restore un-reloaded files on exit (for local testing).
cleanedUp=0 # avoid recursion if cleanup fails
godotPid=0 # kill previous instance if necessary

cleanup() {
  if [[ $cleanedUp -eq 0 ]]; then
    cleanedUp=1
    if [[ $godotPid -ne 0 ]]; then
      echo "[Bash]      Kill Godot (PID $godotPid)..."
      kill $godotPid || true # ignore errors here
    fi
    echo "[Bash]      Cleanup..."
    git checkout --quiet $rel/../rust/src/lib.rs $rel/rust.gdextension $rel/MainScene.tscn || true # ignore errors here
  fi
}

set -euo pipefail
trap cleanup EXIT

echo "[Bash]      Start hot-reload integration test..."

# Restore un-reloaded file (for local testing).
git checkout --quiet $rel/../rust/src/lib.rs $rel/rust.gdextension

# Set up editor file which has scene open, so @tool script loads at startup. Also copy scene file that holds a script.
mkdir -p $rel/.godot/editor
cp editor_layout.cfg $rel/.godot/editor/editor_layout.cfg
#cp MainScene.tscn $rel/MainScene.tscn

# Compile original Rust source.
#cargoArgs="--features godot/__debug-log"
cargo build -p hot-reload $cargoArgs

# Wait briefly so artifacts are present on file system.
sleep 0.5

$GODOT4_BIN -e --headless --path $rel &
godotPid=$!
echo "[Bash]      Wait for Godot ready (PID $godotPid)..."

$GODOT4_BIN --headless --no-header --script ReloadOrchestrator.gd -- await
$GODOT4_BIN --headless --no-header --script ReloadOrchestrator.gd -- replace

# Compile updated Rust source.
cargo build -p hot-reload $cargoArgs

$GODOT4_BIN --headless --no-header --script ReloadOrchestrator.gd -- notify

echo "[Bash]      Wait for Godot exit..."
wait $godotPid
status=$?
echo "[Bash]      Godot (PID $godotPid) has completed with status $status."



