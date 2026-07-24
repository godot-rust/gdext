/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Diagnostics for multiple godot-rust extensions loaded into the same address space.
//!
//! On most platforms, each GDExtension is a separate shared library with its own copy of all `static` variables, so several godot-rust based
//! extensions can coexist in one process. On Wasm/Emscripten this is *not* true: extensions are linked as *side modules*, and any symbol with
//! external linkage and default visibility is resolved to a single address by Emscripten's dynamic linker. Two extensions that both link
//! godot-rust then share its globals -- class-ID cache, string cache, registries -- which corrupts state and is undefined behavior.
//!
//! The symptom is a confusing panic during load, e.g. `already initialized` or
//! `insert_class_name() called for already-existing string: RefCounted`. [`MULTI_EXTENSION_HINT`] is appended to those panics to name the
//! likely cause. Sharing is only detected where it actually manifests; there is no separate probe, since whether two extensions collide
//! depends on their mangled symbol names, i.e. on godot-rust version and enabled features.
//!
//! See <https://github.com/godot-rust/gdext/issues/968>.

/// Explains the Wasm side-module global-sharing problem and how to work around it.
///
/// Appended to panic messages that are plausibly caused by it, so users don't have to guess what the real cause is. Empty outside Wasm, where
/// each extension has its own globals and the hint would only be misleading noise.
#[cfg(not(target_family = "wasm"))]
pub const MULTI_EXTENSION_HINT: &str = "";

/// See [non-Wasm version][MULTI_EXTENSION_HINT] (`cfg`-dependent).
#[cfg(target_family = "wasm")]
pub const MULTI_EXTENSION_HINT: &str = "\n\n\
    If your project loads several GDExtensions that are built with godot-rust, note that on Wasm/Emscripten they share global variables \
    (they are linked as side modules, so equally-named symbols resolve to one address). godot-rust state is then corrupted across \
    extensions. Workarounds:\n\
    1. Build *every* godot-rust extension with `-Zdefault-visibility=hidden` (nightly rustc), in .cargo/config.toml:\n   \
         [target.wasm32-unknown-emscripten]\n   \
         rustflags = [\"-Zdefault-visibility=hidden\", ...]\n\
    2. Or merge all your Rust code into a single GDExtension library.\n\
    See https://github.com/godot-rust/gdext/issues/968";
