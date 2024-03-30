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
use std::thread::ThreadId;

use super::GodotBinding;
use crate::ManualInitCell;

pub(super) struct BindingStorage {
    // Is used in to check that we've been called from the right thread, so must be thread-safe to access.
    main_thread_id: Cell<Option<ThreadId>>,
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
            main_thread_id: Cell::new(None),
            binding: ManualInitCell::new(),
        };

        &BINDING
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

        assert!(
            storage.main_thread_id.get().is_none(),
            "initialize must only be called at startup or after deinitialize"
        );
        storage
            .main_thread_id
            .set(Some(std::thread::current().id()));

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

        storage
            .main_thread_id
            .get()
            .expect("deinitialize without prior initialize");

        storage.main_thread_id.set(None);

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
        let storage = Self::storage();

        if cfg!(debug_assertions) {
            let main_thread_id = storage.main_thread_id.get().expect(
                "Godot engine not available; make sure you are not calling it from unit/doc tests",
            );

            assert_eq!(
                main_thread_id,
                std::thread::current().id(),
                "attempted to access binding from different thread than main thread; this is UB - use the \"experimental-threads\" feature."
            );
        }

        // SAFETY: This function can only be called when the binding is initialized and from the main thread, so we know that it's initialized.
        unsafe { storage.binding.get_unchecked() }
    }

    pub fn is_initialized() -> bool {
        // SAFETY: We don't access the binding.
        let storage = unsafe { Self::storage() };
        storage.main_thread_id.get().is_some()
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
