/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Thread safe binding storage.
//!
//! This can be used from different threads without issue, as late initialization must be synchronized.
//!
//! The user of these structs and functions must still ensure that multithreaded usage of the various pointers is safe.

use std::sync::atomic::{AtomicBool, Ordering};

use super::GodotBinding;
use crate::ManualInitCell;

pub(super) struct BindingStorage {
    // Guards the binding-live window (set on init, cleared on deinit), same as the single-threaded storage: `Release`-stores on
    // init/deinit pair with `Acquire`-loads on read, giving well-defined ordering for steady-state thread-safe reads. Does *not*
    // protect against concurrent teardown races -- the engine must join extension threads before unloading the library.
    initialized: AtomicBool,
    binding: ManualInitCell<GodotBinding>,
}

impl BindingStorage {
    /// Get the static binding storage.
    #[inline(always)]
    fn storage() -> &'static Self {
        static BINDING: BindingStorage = BindingStorage {
            initialized: AtomicBool::new(false),
            binding: ManualInitCell::new(),
        };
        &BINDING
    }

    fn initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Initialize the binding storage, this must be called before any other public functions.
    ///
    /// # Safety
    /// - Must be called on startup or strictly after [`deinitialize`](Self::deinitialize).
    /// - Must not be called concurrently with [`get_binding_unchecked`](Self::get_binding_unchecked).
    pub unsafe fn initialize(binding: GodotBinding) {
        let storage = Self::storage();

        assert!(
            !storage.initialized(),
            "initialize must only be called at startup or after deinitialize"
        );

        // SAFETY: per declared invariants.
        unsafe { storage.binding.set(binding) }

        // Publish the binding *before* marking it live, see single-threaded storage for rationale.
        storage.initialized.store(true, Ordering::Release);
    }

    /// Deinitialize the binding storage.
    ///
    /// # Safety
    /// See [`initialize`](BindingStorage::initialize).
    pub unsafe fn deinitialize() {
        let storage = Self::storage();

        assert!(
            storage.initialized(),
            "deinitialize must only be called after initialize"
        );

        // Mark the binding not-live *before* clearing it, see single-threaded storage for rationale.
        storage.initialized.store(false, Ordering::Release);

        // SAFETY: per declared invariants.
        unsafe { storage.binding.clear() };
    }

    /// Get the binding from the binding storage.
    ///
    /// # Safety
    ///
    /// - The binding must be initialized.
    #[inline(always)]
    pub unsafe fn get_binding_unchecked() -> &'static GodotBinding {
        let storage = Self::storage();

        // Live check: see single-threaded storage for rationale.
        #[cfg(safeguards_balanced)]
        super::assert_binding_live(&storage.initialized);

        // SAFETY: The binding has been initialized before calling this method.
        unsafe { storage.binding.get_unchecked() }
    }

    pub fn is_initialized() -> bool {
        let storage = Self::storage();
        storage.initialized()
    }

    /// No-op in multi-threaded builds: with "experimental-threads", FFI access from any thread is permitted, so there is no main-thread
    /// assertion to make. Exists for API parity with the single-threaded storage, which the shared `get_binding()` relies on.
    #[inline(always)]
    pub(super) fn ensure_main_thread() {}
}
