#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Sets up a patch in Cargo.toml to use godot4-prebuilt artifacts.
# Input: $version

version="$1"

cat << HEREDOC >> Cargo.toml
[patch."https://github.com/godot-rust/godot4-prebuilt"]
godot4-prebuilt = { git = "https://github.com//godot-rust/godot4-prebuilt", branch = "$version" }
HEREDOC

echo "Patched Cargo.toml for version $version."
