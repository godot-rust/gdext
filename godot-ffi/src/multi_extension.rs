/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Detection of multiple godot-rust extensions loaded into the same address space.
//!
//! On most platforms, each GDExtension is a separate shared library with its own copy of all `static` variables, so several godot-rust based
//! extensions can coexist in one process. On Wasm/Emscripten this is *not* true: extensions are linked as *side modules*, and any symbol with
//! external linkage and default visibility is resolved to a single address by Emscripten's dynamic linker. Two extensions that both link
//! godot-rust (same version) therefore share godot-rust's globals -- class-ID cache, string cache, registries -- which corrupts state and is
//! undefined behavior.
//!
//! The symptom users see is a confusing panic somewhere during load, e.g. `already initialized` or
//! `insert_class_name() called for already-existing string: RefCounted`. This module turns that into an explicit, actionable diagnosis.
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

/// Registers a library load and panics if another godot-rust extension is detected in the same address space.
///
/// See [module docs][self] for what is being detected. No-op outside Wasm.
pub(crate) fn on_library_init() {
    #[cfg(target_family = "wasm")]
    probe::on_init();
}

/// Counterpart to [`on_library_init()`], called on library unload (also on hot-reload).
pub(crate) fn on_library_deinit() {
    #[cfg(target_family = "wasm")]
    probe::on_deinit();
}

#[cfg(target_family = "wasm")]
mod probe {
    use std::sync::atomic::{AtomicU32, Ordering};

    // Deliberately exported (external linkage, default visibility) -- this is the *probe*: if several godot-rust side modules are loaded and
    // globals are shared, all of them increment this very counter. The version suffix keeps semver-incompatible godot-rust versions apart;
    // those don't share their mangled statics either, so they are not affected by the problem. Must be updated on minor version bumps.
    #[unsafe(no_mangle)]
    #[allow(non_upper_case_globals)] // Symbol name is user-visible in .wasm; keep it C-style.
    pub static gdext_wasm_library_count_v0_5: AtomicU32 = AtomicU32::new(0);

    /// Per-module counter. Declared inside a function so it keeps *internal* linkage and is never merged across side modules -- the whole
    /// detection rests on this asymmetry with the exported counter above.
    fn local_count() -> &'static AtomicU32 {
        static LOCAL_COUNT: AtomicU32 = AtomicU32::new(0);
        &LOCAL_COUNT
    }

    pub fn on_init() {
        let shared = gdext_wasm_library_count_v0_5.fetch_add(1, Ordering::SeqCst) + 1;
        let local = local_count().fetch_add(1, Ordering::SeqCst) + 1;

        // If the user applied `-Zdefault-visibility=hidden`, the exported counter is module-local as well, both stay in sync and nothing is
        // reported -- which is correct, since then the globals are properly isolated.
        if shared != local {
            // Roll back, so a caught panic doesn't leave the counters further apart than necessary.
            gdext_wasm_library_count_v0_5.fetch_sub(1, Ordering::SeqCst);
            local_count().fetch_sub(1, Ordering::SeqCst);

            panic!(
                "Detected {shared} godot-rust extensions sharing one address space (this one is #{local} of its own module).{hint}",
                hint = super::MULTI_EXTENSION_HINT
            );
        }
    }

    pub fn on_deinit() {
        gdext_wasm_library_count_v0_5.fetch_sub(1, Ordering::SeqCst);
        local_count().fetch_sub(1, Ordering::SeqCst);
    }
}
