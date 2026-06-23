/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Non-thread safe binding storage.
//!
//! If used from different threads then there will be runtime errors in debug mode and UB in release mode.

use std::sync::atomic::{AtomicBool, Ordering};

use super::GodotBinding;
use crate::ManualInitCell;

pub(super) struct BindingStorage {
    // Guards the binding-live window (set on init, cleared on deinit). `AtomicBool` instead of `Cell<bool>` so thread-safe FFI calls off the main
    // thread still have well-defined ordering: `Release`-stores on init/deinit pair with `Acquire`-loads on read, establishing happens-before.
    // Does *not* protect against concurrent teardown races -- the engine must join extension threads before unloading the library.
    initialized: AtomicBool,
    binding: ManualInitCell<GodotBinding>,
}

impl BindingStorage {
    /// Get the static binding storage.
    ///
    /// # Safety
    /// You must not access `binding` from a thread different from the thread [`initialize`](BindingStorage::initialize) was first called from,
    /// unless the accessed FFI function is itself thread-safe (see [`get_binding_unchecked`](BindingStorage::get_binding_unchecked)).
    #[inline(always)]
    unsafe fn storage() -> &'static Self {
        static BINDING: BindingStorage = BindingStorage {
            initialized: AtomicBool::new(false),
            binding: ManualInitCell::new(),
        };

        &BINDING
    }

    /// Returns whether the binding storage has already been initialized.
    ///
    /// It is recommended to use this function for that purpose as the field to check varies depending on the compilation target.
    fn initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Initialize the binding storage, this must be called before any other public functions.
    ///
    /// # Safety
    /// Must be called from the main thread.
    ///
    /// # Panics
    /// If called while already initialized. Note that calling it after `deinitialize()` is possible, e.g. for Linux hot-reload.
    pub unsafe fn initialize(binding: GodotBinding) {
        // SAFETY: Either we are the first call to `initialize` and so we are calling from the same thread as ourselves. Or we are a later call,
        // in which case we can tell that the storage has been initialized, and we don't access `binding`.
        let storage = unsafe { Self::storage() };

        assert!(!storage.initialized(), "already initialized");

        // SAFETY: We are the first thread to set this binding (possibly after deinitialize), as otherwise the above assert would fail. We also
        // know initialize() is not called concurrently with anything else that can call another method on the binding, since this method is
        // called from the main thread and so must any other methods.
        unsafe { storage.binding.set(binding) };

        // Publish the binding *before* marking it live: a reader observing `true` (with the corresponding `Acquire`-load) is then guaranteed to
        // see the fully-written binding.
        storage.initialized.store(true, Ordering::Release);
    }

    /// Deinitialize the binding storage.
    ///
    /// # Safety
    /// Must be called from the main thread.
    ///
    /// # Panics
    /// If called while not initialized.
    pub unsafe fn deinitialize() {
        // SAFETY: We only call this once no other operations happen anymore, i.e. no other access to the binding.
        let storage = unsafe { Self::storage() };

        assert!(
            storage.initialized(),
            "deinitialize without prior initialize"
        );

        // Mark the binding not-live *before* clearing it: a reader observing `false` will not dereference the binding.
        storage.initialized.store(false, Ordering::Release);

        // SAFETY: We are the only thread that can access the binding, and we know that it's initialized.
        unsafe { storage.binding.clear() };
    }

    /// Get the binding from the binding storage.
    ///
    /// This performs the "binding is live" check (turning before-init / after-deinit access into a clean panic), but does *not* assert that the
    /// caller is on the main thread -- so it is the right entry point for thread-safe FFI functions. Callers that touch engine/scene state must
    /// additionally go through [`ensure_main_thread`](BindingStorage::ensure_main_thread).
    ///
    /// # Safety
    /// - The binding must be initialized.
    #[inline(always)]
    pub unsafe fn get_binding_unchecked() -> &'static GodotBinding {
        let storage = unsafe { Self::storage() };

        // Live check: passes in ~100% of real calls. Compiled out under the disengaged safety profile, recovering unchecked speed for users who
        // promise the invariant. The actual check lives in a standalone function so it can be unit-tested without a real binding.
        #[cfg(safeguards_balanced)] #[cfg_attr(published_docs, doc(cfg(safeguards_balanced)))]
        assert_binding_live(&storage.initialized);

        // SAFETY: Per the safety contract the binding is initialized, so the cell holds a value.
        unsafe { storage.binding.get_unchecked() }
    }

    pub fn is_initialized() -> bool {
        // SAFETY: We don't access the binding.
        let storage = unsafe { Self::storage() };

        storage.initialized()
    }

    /// Asserts that the caller is on the main thread. Used by the restricted accessor (`sys::on_main()`) for FFI functions that touch
    /// engine/scene state; thread-safe functions skip this.
    pub(super) fn ensure_main_thread() {
        // Check that we're on the main thread. Only enabled with balanced+ safeguards and, for Wasm, in threaded builds.
        // In wasm_nothreads, there's only one thread, so no check is needed.
        #[cfg(all(safeguards_balanced, not(wasm_nothreads)))] #[cfg_attr(published_docs, doc(cfg(all(safeguards_balanced, not(wasm_nothreads)))))]
        if !crate::is_main_thread() {
            // If a binding is accessed the first time, this will panic and start unwinding. It can then happen that during unwinding,
            // another FFI call happens (e.g. Godot destructor), which would cause immediate abort, swallowing the error message.
            // Thus check if a panic is already in progress.

            if std::thread::panicking() {
                eprintln!(
                    "ERROR: Attempted to access binding from different thread than main thread; this is UB.\n\
                    Cannot panic because panic unwind is already in progress. Please check surrounding messages to fix the bug."
                );
            } else {
                panic!(
                    "attempted to access binding from different thread than main thread; \
                    this is UB - use the \"experimental-threads\" feature."
                )
            };
        }
    }
}

// SAFETY: We ensure that `binding` is only ever accessed from the same thread that initialized it.
unsafe impl Sync for BindingStorage {}
// SAFETY: We ensure that `binding` is only ever accessed from the same thread that initialized it.
unsafe impl Send for BindingStorage {}

/// Panics if the binding is not currently live, turning before-init / after-deinit access into a clean error instead of UB.
///
/// Standalone (not a method) so it can be unit-tested against a hand-made `AtomicBool` without a real binding behind it.
#[cfg(safeguards_balanced)] #[cfg_attr(published_docs, doc(cfg(safeguards_balanced)))]
#[inline(always)]
fn assert_binding_live(initialized: &AtomicBool) {
    if !initialized.load(Ordering::Acquire) {
        not_live_panic();
    }
}

/// Failure path for the live check; separated out and marked cold so the hot path stays a predicted-not-taken branch.
#[cfg(safeguards_balanced)] #[cfg_attr(published_docs, doc(cfg(safeguards_balanced)))]
#[cold]
#[inline(never)]
fn not_live_panic() -> ! {
    panic!(
        "Godot binding accessed before initialization or after deinitialization. \
        This typically means a `#[ctor]`/`#[dtor]` constructor, a library destructor, or a leftover user thread touched the Godot API \
        outside the engine's load/unload window."
    )
}

#[cfg(all(test, safeguards_balanced))] #[cfg_attr(published_docs, doc(cfg(all(test, safeguards_balanced))))]
mod tests {
    use super::*;

    #[test]
    fn live_check_passes_when_initialized() {
        // Must not panic.
        assert_binding_live(&AtomicBool::new(true));
    }

    #[test]
    #[should_panic(expected = "accessed before initialization or after deinitialization")]
    fn live_check_panics_when_not_initialized() {
        assert_binding_live(&AtomicBool::new(false));
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct GdextConfig {
    pub tool_only_in_editor: bool,
}

impl GdextConfig {
    pub fn new(tool_only_in_editor: bool) -> Self {
        Self {
            tool_only_in_editor,
        }
    }
}
