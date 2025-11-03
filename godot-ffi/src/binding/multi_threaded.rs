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

use std::sync::OnceLock;

use super::GodotBinding;
use crate::ManualInitCell;

pub(super) struct BindingStorage {
    binding: ManualInitCell<GodotBinding>,
}

impl BindingStorage {
    /// Get the static binding storage.
    #[inline(always)]
    fn storage() -> &'static Self {
        static BINDING: BindingStorage = BindingStorage {
            binding: ManualInitCell::new(),
        };
        &BINDING
    }

    /// Initialize the binding storage, this must be called before any other public functions.
    ///
    /// # Safety
    /// - Must be called on startup or strictly after [`deinitialize`](Self::deinitialize).
    /// - Must not be called concurrently with [`get_binding_unchecked`](Self::get_binding_unchecked).
    pub unsafe fn initialize(binding: GodotBinding) {
        let storage = Self::storage();

        assert!(
            !storage.binding.is_initialized(),
            "initialize must only be called at startup or after deinitialize"
        );

        // SAFETY: per declared invariants.
        unsafe { storage.binding.set(binding) }
    }

    /// Deinitialize the binding storage.
    ///
    /// # Safety
    /// See [`initialize`](BindingStorage::initialize).
    pub unsafe fn deinitialize() {
        let storage = Self::storage();

        assert!(
            storage.binding.is_initialized(),
            "deinitialize must only be called after initialize"
        );

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

        crate::strict_assert!(
            storage.binding.is_initialized(),
            "Godot engine not available; make sure you are not calling it from unit/doc tests"
        );

        // SAFETY: The binding has been initialized before calling this method.
        unsafe { storage.binding.get_unchecked() }
    }

    pub fn is_initialized() -> bool {
        let storage = Self::storage();
        storage.binding.is_initialized()
    }
}

pub struct GdextConfig {
    /// True if only `#[class(tool)]` classes are active in editor; false if all classes are.
    pub tool_only_in_editor: bool,

    /// Whether the extension is loaded in an editor.
    is_editor: OnceLock<bool>,
}

impl GdextConfig {
    pub fn new(tool_only_in_editor: bool) -> Self {
        Self {
            tool_only_in_editor,
            is_editor: OnceLock::new(),
        }
    }

    pub fn is_editor_or_init(&self, is_editor: impl FnOnce() -> bool) -> bool {
        *self.is_editor.get_or_init(is_editor)
    }
}
