#!/bin/sh

# Must be in dodge-the-creep's rust directory in order to pick up the .cargo/config
cd `dirname "$0"`

# We build the host gdextension first so that the godot editor doesn't complain.
cargo +nightly build --package dodge-the-creeps &&
cargo +nightly build --package dodge-the-creeps --target wasm32-unknown-emscripten -Zbuild-std $@
