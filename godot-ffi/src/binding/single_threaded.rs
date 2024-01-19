/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Non-thread safe binding storage.
//!
//! If used from different threads then there will be runtime errors in debug mode and UB in release mode.

use std::{cell::OnceCell, sync::OnceLock, thread::ThreadId};

use crate::{
    BuiltinLifecycleTable, BuiltinMethodTable, ClassEditorMethodTable, ClassSceneMethodTable,
    ClassServersMethodTable, GDExtensionInterface, GdextRuntimeMetadata, UtilityFunctionTable,
};

use super::GDExtensionClassLibraryPtr;

pub(super) struct BindingStorage {
    // Is used in to check that we've been called from the right thread, so must be thread-safe to access.
    main_thread: OnceLock<ThreadId>,
    binding: OnceCell<GodotBinding>,
}

impl BindingStorage {
    /// Get the static binding storage.
    ///
    /// # Safety
    ///
    /// You must not access `binding` from a thread different than the thread [`initialize`](BindingStorage::initialize) was first called from.
    #[inline(always)]
    unsafe fn storage() -> &'static Self {
        static BINDING: BindingStorage = BindingStorage {
            main_thread: OnceLock::new(),
            binding: OnceCell::new(),
        };

        &BINDING
    }

    /// Initialize the binding storage, this must be called before any other public functions.
    #[must_use]
    pub fn initialize(binding: GodotBinding) -> Option<()> {
        // SAFETY: Either we are the first call to `initialize` and so we are calling from the same thread as ourself. Or we are a later call,
        // in which case we can tell that the storage has been initialized and dont access `binding`.
        let storage = unsafe { Self::storage() };

        storage.main_thread.set(std::thread::current().id()).ok()?;
        storage
            .binding
            .set(binding)
            .ok()
            .expect("`main_thread` was unset so `binding` should also be unset");

        Some(())
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
            let main_thread = storage.main_thread.get().expect(
                "Godot engine not available; make sure you are not calling it from unit/doc tests",
            );
            assert_eq!(main_thread, &std::thread::current().id(), "attempted to access binding from different thread than main thread; this is UB - use the \"experimental-threads\" feature.");
            storage.binding.get().unwrap()
        } else {
            // SAFETY: This function can only be called when the binding is initialized and from the main thread, so we know that it's initialized.
            unsafe { storage.binding.get().unwrap_unchecked() }
        }
    }

    pub fn is_initialized() -> bool {
        // SAFETY: We do not access `binding`.
        let storage = unsafe { Self::storage() };
        storage.main_thread.get().is_some()
    }
}

// SAFETY: We ensure that `binding` is only ever accessed from the same thread that initialized it.
unsafe impl Sync for BindingStorage {}
// SAFETY: We ensure that `binding` is only ever accessed from the same thread that initialized it.
unsafe impl Send for BindingStorage {}

pub struct GodotBinding {
    pub(super) interface: GDExtensionInterface,
    pub(super) library: GDExtensionClassLibraryPtr,
    pub(super) global_method_table: BuiltinLifecycleTable,
    pub(super) class_server_method_table: OnceCell<ClassServersMethodTable>,
    pub(super) class_scene_method_table: OnceCell<ClassSceneMethodTable>,
    pub(super) class_editor_method_table: OnceCell<ClassEditorMethodTable>,
    pub(super) builtin_method_table: OnceCell<BuiltinMethodTable>,
    pub(super) utility_function_table: UtilityFunctionTable,
    pub(super) runtime_metadata: GdextRuntimeMetadata,
    pub(super) config: GdextConfig,
}

impl GodotBinding {
    pub fn new(
        interface: GDExtensionInterface,
        library: crate::GDExtensionClassLibraryPtr,
        global_method_table: BuiltinLifecycleTable,
        utility_function_table: UtilityFunctionTable,
        runtime_metadata: GdextRuntimeMetadata,
        config: GdextConfig,
    ) -> Self {
        Self {
            interface,
            library: GDExtensionClassLibraryPtr(library),
            global_method_table,
            class_server_method_table: OnceCell::new(),
            class_scene_method_table: OnceCell::new(),
            class_editor_method_table: OnceCell::new(),
            builtin_method_table: OnceCell::new(),
            utility_function_table,
            runtime_metadata,
            config,
        }
    }
}

pub struct GdextConfig {
    pub tool_only_in_editor: bool,
    is_editor: OnceCell<bool>,
}

impl GdextConfig {
    pub fn new(tool_only_in_editor: bool) -> Self {
        Self {
            tool_only_in_editor,
            is_editor: OnceCell::new(),
        }
    }

    pub fn is_editor_or_init(&self, is_editor: impl FnOnce() -> bool) -> bool {
        *self.is_editor.get_or_init(is_editor)
    }
}
