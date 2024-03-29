/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{
    BuiltinLifecycleTable, BuiltinMethodTable, ClassEditorMethodTable, ClassSceneMethodTable,
    ClassServersMethodTable, GDExtensionClassLibraryPtr, GDExtensionInterface,
    GdextRuntimeMetadata, UnsafeOnceCell, UtilityFunctionTable,
};

#[cfg(feature = "experimental-threads")]
mod multi_threaded;
#[cfg(not(feature = "experimental-threads"))]
mod single_threaded;

#[cfg(feature = "experimental-threads")]
use multi_threaded::BindingStorage;
#[cfg(not(feature = "experimental-threads"))]
use single_threaded::BindingStorage;

#[cfg(feature = "experimental-threads")]
pub use multi_threaded::GdextConfig;
#[cfg(not(feature = "experimental-threads"))]
pub use single_threaded::GdextConfig;

// Note, this is `Sync` and `Send` when "experimental-threads" is enabled because all its fields are. We have avoided implementing `Sync`
// and `Send` for `GodotBinding` as that could hide issues if any of the field types are changed to no longer be sync/send, but the manual
// implementation for `GodotBinding` wouldn't detect that.
pub(crate) struct GodotBinding {
    interface: GDExtensionInterface,
    library: ClassLibraryPtr,
    global_method_table: BuiltinLifecycleTable,
    class_server_method_table: UnsafeOnceCell<ClassServersMethodTable>,
    class_scene_method_table: UnsafeOnceCell<ClassSceneMethodTable>,
    class_editor_method_table: UnsafeOnceCell<ClassEditorMethodTable>,
    builtin_method_table: UnsafeOnceCell<BuiltinMethodTable>,
    utility_function_table: UtilityFunctionTable,
    runtime_metadata: GdextRuntimeMetadata,
    config: GdextConfig,
}

impl GodotBinding {
    pub fn new(
        interface: GDExtensionInterface,
        library: GDExtensionClassLibraryPtr,
        global_method_table: BuiltinLifecycleTable,
        utility_function_table: UtilityFunctionTable,
        runtime_metadata: GdextRuntimeMetadata,
        config: GdextConfig,
    ) -> Self {
        Self {
            interface,
            library: ClassLibraryPtr(library),
            global_method_table,
            class_server_method_table: UnsafeOnceCell::new(),
            class_scene_method_table: UnsafeOnceCell::new(),
            class_editor_method_table: UnsafeOnceCell::new(),
            builtin_method_table: UnsafeOnceCell::new(),
            utility_function_table,
            runtime_metadata,
            config,
        }
    }
}

/// Newtype around `GDExtensionClassLibraryPtr` so we can implement `Sync` and `Send` manually for this.
struct ClassLibraryPtr(crate::GDExtensionClassLibraryPtr);

// SAFETY: This implementation of `Sync` and `Send` does not guarantee that reading from or writing to the pointer is actually
// thread safe. It merely means we can send/share the pointer itself between threads. Which is safe since any place that actually
// reads/writes to this pointer must ensure they do so in a thread safe manner.
//
// So these implementations effectively just pass the responsibility for thread safe usage of the library pointer onto whomever
// reads/writes to the pointer from a different thread. Since doing so requires `unsafe` anyway this is something we can do soundly.
unsafe impl Sync for ClassLibraryPtr {}
// SAFETY: See `Sync` impl safety doc.
unsafe impl Send for ClassLibraryPtr {}

/// Initializes the Godot binding.
///
/// Most other functions in this module rely on this function being called first as a safety condition.
///
/// # Safety
///
/// Must not be called concurrently with other functions that interact with the bindings - this is trivially true if "experimental-threads"
/// is not enabled.
///
/// If "experimental-threads" is enabled, then must be called from the main thread.
pub(crate) unsafe fn initialize_binding(binding: GodotBinding) {
    unsafe {
        BindingStorage::initialize(binding);
    }
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub(crate) unsafe fn get_binding() -> &'static GodotBinding {
    BindingStorage::get_binding_unchecked()
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_class_server_method_table(table: ClassServersMethodTable) {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        !binding.class_editor_method_table.is_initialized(),
        "server method table should only be initialized once"
    );

    // SAFETY: Is only called once, and is called before any accesses to this table.
    unsafe { binding.class_server_method_table.set(table) }
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_class_scene_method_table(table: ClassSceneMethodTable) {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        !binding.class_scene_method_table.is_initialized(),
        "scene method table should only be initialized once"
    );

    // SAFETY: Is only called once, and is called before any accesses to this table.
    unsafe { binding.class_scene_method_table.set(table) }
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_class_editor_method_table(table: ClassEditorMethodTable) {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        !binding.class_editor_method_table.is_initialized(),
        "editor method table should only be initialized once"
    );

    // SAFETY: Is only called once, and is called before any accesses to this table.
    unsafe { binding.class_editor_method_table.set(table) }
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_builtin_method_table(table: BuiltinMethodTable) {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        !binding.builtin_method_table.is_initialized(),
        "builtin method table should only be initialized once"
    );

    // SAFETY: Is only called once, and is called before any accesses to this table.
    unsafe { binding.builtin_method_table.set(table) }
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn get_interface() -> &'static GDExtensionInterface {
    &get_binding().interface
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn get_library() -> crate::GDExtensionClassLibraryPtr {
    get_binding().library.0
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn builtin_lifecycle_api() -> &'static BuiltinLifecycleTable {
    &get_binding().global_method_table
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class servers method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn class_servers_api() -> &'static ClassServersMethodTable {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        binding.class_server_method_table.is_initialized(),
        "cannot fetch classes; init level 'Servers' not yet loaded"
    );

    // SAFETY: `initialize_class_server_method_table` has been called.
    unsafe { binding.class_server_method_table.get_unchecked() }
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class scene method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn class_scene_api() -> &'static ClassSceneMethodTable {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        binding.class_scene_method_table.is_initialized(),
        "cannot fetch classes; init level 'Scene' not yet loaded"
    );

    // SAFETY: `initialize_class_scene_method_table` has been called.
    unsafe { binding.class_scene_method_table.get_unchecked() }
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class editor method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn class_editor_api() -> &'static ClassEditorMethodTable {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(
        binding.class_editor_method_table.is_initialized(),
        "cannot fetch classes; init level 'Editor' not yet loaded"
    );

    // SAFETY: `initialize_class_editor_method_table` has been called.
    unsafe { binding.class_editor_method_table.get_unchecked() }
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The builtin method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn builtin_method_table() -> &'static BuiltinMethodTable {
    // SAFETY: `get_binding` has the same preconditions as this function.
    let binding = unsafe { get_binding() };

    debug_assert!(binding.builtin_method_table.is_initialized());

    // SAFETY: `initialize_builtin_method_table` has been called.
    unsafe { binding.builtin_method_table.get_unchecked() }
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn utility_function_table() -> &'static UtilityFunctionTable {
    &get_binding().utility_function_table
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub(crate) unsafe fn runtime_metadata() -> &'static GdextRuntimeMetadata {
    &get_binding().runtime_metadata
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline]
pub unsafe fn config() -> &'static GdextConfig {
    &get_binding().config
}

#[inline]
pub fn is_initialized() -> bool {
    BindingStorage::is_initialized()
}
