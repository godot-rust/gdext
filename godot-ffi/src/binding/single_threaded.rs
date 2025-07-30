/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Non-thread safe binding storage.
//!
//! If used from different threads then there will be runtime errors in debug mode and UB in release mode.

use std::cell::Cell;

use super::GodotBinding;
use crate::ManualInitCell;

pub(super) struct BindingStorage {
    // No threading when linking against Godot with a nothreads Wasm build.
    // Therefore, we just need to check if the bindings were initialized, as all accesses are from the main thread.
    initialized: Cell<bool>,
    binding: ManualInitCell<GodotBinding>,
}

impl BindingStorage {
    /// Get the static binding storage.
    ///
    /// # Safety
    ///
    /// You must not access `binding` from a thread different from the thread [`initialize`](BindingStorage::initialize) was first called from.
    #[inline(always)]
    unsafe fn storage() -> &'static Self {
        static BINDING: BindingStorage = BindingStorage {
            initialized: Cell::new(false),
            binding: ManualInitCell::new(),
        };

        &BINDING
    }

    /// Returns whether the binding storage has already been initialized.
    ///
    /// It is recommended to use this function for that purpose as the field to check varies depending on the compilation target.
    fn initialized(&self) -> bool {
        self.initialized.get()
    }

    /// Marks the binding storage as initialized or deinitialized.
    /// We store the thread ID to ensure future accesses to the binding only come from the main thread.
    ///
    /// # Safety
    /// Must be called from the main thread. Additionally, the binding storage must be initialized immediately
    /// after this function if `initialized` is `true`, or deinitialized if it is `false`.
    ///
    /// # Panics
    /// If attempting to deinitialize before initializing, or vice-versa.
    unsafe fn set_initialized(&self, initialized: bool) {
        if initialized == self.initialized() {
            if initialized {
                panic!("already initialized");
            } else {
                panic!("deinitialize without prior initialize");
            }
        }

        self.initialized.set(initialized);
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

        // SAFETY: We are about to initialize the binding below, so marking the binding as initialized is correct.
        // If we can't initialize the binding at this point, we get a panic before changing the status, thus the
        // binding won't be set.
        unsafe { storage.set_initialized(true) };

        // SAFETY: We are the first thread to set this binding (possibly after deinitialize), as otherwise the above set() would fail and
        // return early. We also know initialize() is not called concurrently with anything else that can call another method on the binding,
        // since this method is called from the main thread and so must any other methods.
        unsafe { storage.binding.set(binding) };
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

        // SAFETY: We are about to deinitialize the binding below, so marking the binding as deinitialized is correct.
        // If we can't deinitialize the binding at this point, we get a panic before changing the status, thus the
        // binding won't be deinitialized.
        unsafe { storage.set_initialized(false) };

        // SAFETY: We are the only thread that can access the binding, and we know that it's initialized.
        unsafe {
            storage.binding.clear();
        }
    }

    /// Get the binding from the binding storage.
    ///
    /// # Safety
    /// - Must be called from the main thread.
    /// - The binding must be initialized.
    #[inline(always)]
    pub unsafe fn get_binding_unchecked() -> &'static GodotBinding {
        // SAFETY: The bindings were initialized on the main thread because `initialize` must be called from the main thread,
        // and this function is called from the main thread.
        let storage = unsafe { Self::storage() };

        // We only check if we are in the main thread in debug builds if we aren't building for a non-threaded Godot build,
        // since we could otherwise assume there won't be multi-threading.
        #[cfg(all(debug_assertions, not(wasm_nothreads)))]
        {
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

        // SAFETY: This function can only be called when the binding is initialized and from the main thread, so we know that it's initialized.
        unsafe { storage.binding.get_unchecked() }
    }

    pub fn is_initialized() -> bool {
        // SAFETY: We don't access the binding.
        let storage = unsafe { Self::storage() };

        storage.initialized()
    }
}

// SAFETY: We ensure that `binding` is only ever accessed from the same thread that initialized it.
unsafe impl Sync for BindingStorage {}
// SAFETY: We ensure that `binding` is only ever accessed from the same thread that initialized it.
unsafe impl Send for BindingStorage {}

pub struct GdextConfig {
    pub tool_only_in_editor: bool,
    is_editor: std::cell::OnceCell<bool>,
}

impl GdextConfig {
    pub fn new(tool_only_in_editor: bool) -> Self {
        Self {
            tool_only_in_editor,
            is_editor: std::cell::OnceCell::new(),
        }
    }

    pub fn is_editor_or_init(&self, is_editor: impl FnOnce() -> bool) -> bool {
        *self.is_editor.get_or_init(is_editor)
    }
}
