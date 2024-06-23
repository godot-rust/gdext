#!/bin/bash
# Copyright (c) godot-rust; Bromeon and contributors.
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Adds #[doc(cfg(...))] annotations to Rust docs, based on #[cfg(...)] in the source code.

# Keep in sync with the same file in website repo.

# Usage:
#   apply-doc-cfg.sh [--install-sd] [--rustfmt]

set -e


for arg in "$@"; do
  case "$arg" in
    --install-sd)
      installSd="true"
      ;;
    --rustfmt)
      rustfmt="true"
      ;;
    *)
      echo "Unknown argument: $arg"
      exit 1
      ;;
  esac
done

SD_VERSION="1.0.0"
PRE="DocCfg | "

# For gdext, add feature/cfg annotations in docs. This needs nightly rustdoc + custom preprocessing.
# Replace #[cfg(...)] with #[doc(cfg(...))], a nightly feature: https://doc.rust-lang.org/unstable-book/language-features/doc-cfg.html
# Potential alternative: https://docs.rs/doc-cfg/latest/doc_cfg
if [[ "$installSd" == "true" ]]; then
  # Install sd (modern sed). No point in waiting for eternal `cargo install` if we can fetch a prebuilt binary in 1s.
  echo "$PRE install sd (modern sed)..."
  curl -L https://github.com/chmln/sd/releases/download/v${SD_VERSION}/sd-v${SD_VERSION}-x86_64-unknown-linux-musl.tar.gz -o archive.tar.gz
  mkdir -p /tmp/tools
  tar -zxvf archive.tar.gz -C /tmp/tools --strip-components=1
  sd=/tmp/tools/sd
else
  sd=sd
fi

echo "$PRE preprocess docs..."

# Enable feature in each lib.rs file.
# Note: first command uses sed because it's easier, and only handful of files.
find . -type f -name "lib.rs" -exec sed -i '1s/^/#![cfg_attr(published_docs, feature(doc_cfg))]\n/' {} +

# Then do the actual replacements.
# Could use \( -path "..." -o -path "..." \) to limit to certain paths.
# Do NOT include './target/debug/build/*' because generated files cannot be modified -- rustdoc will rerun the generation.
# This is done by directly emitting #[cfg_attr(published_docs, doc(cfg(...)))] in the godot-codegen crate, and that cfg is passed below.
find . -type f -name '*.rs' \
  \( -path './godot' -o -path './godot-*' \) \
| while read -r file; do
    # Replace #[cfg(...)] with #[doc(cfg(...))]. Do not insert a newline, in case the #[cfg] is commented-out.
    # shellcheck disable=SC2016
    $sd '(\#\[(cfg\(.+?\))\])(\s*)([A-Za-z]|#\[)' '$1 #[cfg_attr(published_docs, doc($2))]$3$4' "$file"
    # $sd '(\#\[(cfg\(.+?\))\])\s*([A-Za-z]|#\[)' '$1 #[doc($2)]\n$3' "$file"
    #                               ^^^^^^^^^^^^^^^^^ require that #[cfg] is followed by an identifier or a #[ attribute start.
    # This avoids some usages of function-local #[cfg]s, although by far not all. Others generate warnings, which is fine.
done

if [[ "$rustfmt" == "true" ]]; then
  echo "$PRE Format code using rustfmt..."
  cargo fmt --all
fi

echo "$PRE Docs post-processed."
