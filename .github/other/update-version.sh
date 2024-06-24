#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Small utility to run update crate versions, used by godot-rust developers.

# No args specified: do everything.
if [[ "$#" -eq 0 ]]; then
    echo "Usage: update-version.sh <VER> [--no-tag]"
    exit 1
fi

# --help menu
args=("$@")
for arg in "${args[@]}"; do
    if [[ "$arg" == "--help" ]]; then
        echo "Usage: update-version.sh <VER> [--no-tag]"
        echo ""
        echo "Replaces currently published version with <newVersion>".
        echo "Does not git commit."
        exit 0
    fi

    if [[ "$arg" == "--no-tag" ]]; then
        noTag="true"
    fi
done

# Uncommitted changes, see https://stackoverflow.com/a/3879077.
#if git diff --quiet --exit-code; then
git diff-index --quiet HEAD -- || {
    echo "Repo contains uncommitted changes; make sure working tree is clean."
    exit 1
}

# https://stackoverflow.com/a/11114547
scriptFile=$(realpath "$0")
scriptPath=$(dirname "$scriptFile")
mainCargoToml="$scriptPath/../../godot/Cargo.toml"

newVersion="${args[0]}"
oldVersion=$(grep -Po '^version = "\K[^"]*' "$mainCargoToml")

# Keep in sync with release-version.yml.
publishedCrates=(
    "godot-bindings"
    "godot-codegen"
    "godot-ffi"
    "godot-cell"
    "godot-core"
    "godot-macros"
    "godot"
)

for crate in "${publishedCrates[@]}"; do
    # Don't just replace version string itself -- the following only replaces the crate's own version
    # (with 'version = "1.2.3"') and dependencies with "=1.2.3", which makes false positives unlikely
    sed -i "s!version = \"${oldVersion}\"!version = \"${newVersion}\"!g" "$scriptPath/../../$crate/Cargo.toml" || exit 2
    sed -i "s!\"=${oldVersion}\"!\"=${newVersion}\"!g" "$scriptPath/../../$crate/Cargo.toml" || exit 2
done

# For `godot` itself, update the `documentation` metadata.
sed -i "s!documentation = \"https://docs.rs/godot/$oldVersion\"!documentation = \"https://docs.rs/godot/$newVersion\"!g" "$mainCargoToml" || exit 2

git commit -am "Update crate version: $oldVersion -> $newVersion" || exit 2

if [[ "$noTag" == "true" ]]; then
    echo "Skipped creating tag."
else
    git tag "v$newVersion" || exit 2
    echo "Created tag v$newVersion."
fi

echo ""
echo "SUCCESS: Updated version $oldVersion -> $newVersion"
