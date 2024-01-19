/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Thread safe binding storage.
//!
//! This can be used from different threads without issue, as late initialization uses `OnceLock`.
//!
//! The user of these structs and functions must still ensure that multi-threaded usage of the various pointers is safe.

use std::sync::OnceLock;

use crate::{
    BuiltinLifecycleTable, BuiltinMethodTable, ClassEditorMethodTable, ClassSceneMethodTable,
    ClassServersMethodTable, GDExtensionInterface, GdextRuntimeMetadata, UtilityFunctionTable,
};

use super::GDExtensionClassLibraryPtr;

pub(super) struct BindingStorage {
    binding: OnceLock<GodotBinding>,
}

impl BindingStorage {
    /// Get the static binding storage.
    #[inline(always)]
    fn storage() -> &'static Self {
        static BINDING: BindingStorage = BindingStorage {
            binding: OnceLock::new(),
        };

        &BINDING
    }

    /// Initialize the binding storage, this must be called before any other public functions.
    #[must_use]
    pub fn initialize(binding: GodotBinding) -> Option<()> {
        let storage = Self::storage();

        storage.binding.set(binding).ok()?;

        Some(())
    }

    /// Get the binding from the binding storage.
    ///
    /// # Safety
    /// - The binding must be initialized.
    #[inline(always)]
    pub unsafe fn get_binding_unchecked() -> &'static GodotBinding {
        let storage = Self::storage();
        let binding = storage.binding.get();

        debug_assert!(
            binding.is_some(),
            "Godot engine not available; make sure you are not calling it from unit/doc tests"
        );

        // SAFETY: `binding` is `None` when the binding is uninitialized, but the safety invariant of this method is that
        // the binding is initialized.
        unsafe { binding.unwrap_unchecked() }
    }

    pub fn is_initialized() -> bool {
        let storage = Self::storage();
        storage.binding.get().is_some()
    }
}

// Note, this is `Sync` and `Send` because all its fields are. We have avoided implementing `Sync` and `Send` for `GodotBinding`
// as that could hide issues if any of the field types are changed to no longer be sync/send, but the manual implementation for
// `GodotBinding` wouldn't detect that.
pub struct GodotBinding {
    pub(super) interface: GDExtensionInterface,
    pub(super) library: GDExtensionClassLibraryPtr,
    pub(super) global_method_table: BuiltinLifecycleTable,
    pub(super) class_server_method_table: OnceLock<ClassServersMethodTable>,
    pub(super) class_scene_method_table: OnceLock<ClassSceneMethodTable>,
    pub(super) class_editor_method_table: OnceLock<ClassEditorMethodTable>,
    pub(super) builtin_method_table: OnceLock<BuiltinMethodTable>,
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
            class_server_method_table: OnceLock::new(),
            class_scene_method_table: OnceLock::new(),
            class_editor_method_table: OnceLock::new(),
            builtin_method_table: OnceLock::new(),
            utility_function_table,
            runtime_metadata,
            config,
        }
    }
}

pub struct GdextConfig {
    pub tool_only_in_editor: bool,
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
