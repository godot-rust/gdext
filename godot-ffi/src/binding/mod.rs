/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{
    BuiltinLifecycleTable, BuiltinMethodTable, ClassCoreMethodTable, ClassEditorMethodTable,
    ClassSceneMethodTable, ClassServersMethodTable, GDExtensionClassLibraryPtr,
    GDExtensionInterface, GdextRuntimeMetadata, ManualInitCell, UtilityFunctionTable,
};

#[cfg(feature = "experimental-threads")]
mod multi_threaded;
#[cfg(not(feature = "experimental-threads"))]
mod single_threaded;

#[cfg(feature = "experimental-threads")]
use multi_threaded::BindingStorage;
// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public re-exports
#[cfg(feature = "experimental-threads")]
pub use multi_threaded::GdextConfig;
#[cfg(not(feature = "experimental-threads"))]
use single_threaded::BindingStorage;
#[cfg(not(feature = "experimental-threads"))]
pub use single_threaded::GdextConfig;

// Note, this is `Sync` and `Send` when "experimental-threads" is enabled because all its fields are. We have avoided implementing `Sync`
// and `Send` for `GodotBinding` as that could hide issues if any of the field types are changed to no longer be sync/send, but the manual
// implementation for `GodotBinding` wouldn't detect that.
pub(crate) struct GodotBinding {
    interface: GDExtensionInterface,
    library: ClassLibraryPtr,
    global_method_table: BuiltinLifecycleTable,
    class_core_method_table: ManualInitCell<ClassCoreMethodTable>,
    class_server_method_table: ManualInitCell<ClassServersMethodTable>,
    class_scene_method_table: ManualInitCell<ClassSceneMethodTable>,
    class_editor_method_table: ManualInitCell<ClassEditorMethodTable>,
    builtin_method_table: ManualInitCell<BuiltinMethodTable>,
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
            class_core_method_table: ManualInitCell::new(),
            class_server_method_table: ManualInitCell::new(),
            class_scene_method_table: ManualInitCell::new(),
            class_editor_method_table: ManualInitCell::new(),
            builtin_method_table: ManualInitCell::new(),
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// # Safety
/// The table must not have been initialized yet.
unsafe fn initialize_table<T>(table: &ManualInitCell<T>, value: T, what: &str) {
    debug_assert!(
        !table.is_initialized(),
        "method table for {what} should only be initialized once"
    );

    table.set(value)
}

/// # Safety
/// The table must have been initialized.
unsafe fn get_table<T>(table: &'static ManualInitCell<T>, msg: &str) -> &'static T {
    debug_assert!(table.is_initialized(), "{msg}");

    table.get_unchecked()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public API

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
    get_table(
        &get_binding().class_server_method_table,
        "cannot fetch classes; init level 'Servers' not yet loaded",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class core method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn class_core_api() -> &'static ClassCoreMethodTable {
    get_table(
        &get_binding().class_core_method_table,
        "cannot fetch classes; init level 'Core' not yet loaded",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class scene method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn class_scene_api() -> &'static ClassSceneMethodTable {
    get_table(
        &get_binding().class_scene_method_table,
        "cannot fetch classes; init level 'Scene' not yet loaded",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class editor method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn class_editor_api() -> &'static ClassEditorMethodTable {
    get_table(
        &get_binding().class_editor_method_table,
        "cannot fetch classes; init level 'Editor' not yet loaded",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The builtin method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
pub unsafe fn builtin_method_table() -> &'static BuiltinMethodTable {
    get_table(
        &get_binding().builtin_method_table,
        "cannot fetch builtin methods; table not ready",
    )
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
#[inline]
pub unsafe fn config() -> &'static GdextConfig {
    &get_binding().config
}

#[inline]
pub fn is_initialized() -> bool {
    BindingStorage::is_initialized()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Crate-local implementation

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
    BindingStorage::initialize(binding);
}

/// Deinitializes the Godot binding.
///
/// # Safety
///
/// See [`initialize_binding`].
pub(crate) unsafe fn deinitialize_binding() {
    BindingStorage::deinitialize();
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
pub(crate) unsafe fn initialize_class_core_method_table(table: ClassCoreMethodTable) {
    initialize_table(
        &get_binding().class_core_method_table,
        table,
        "classes (Core level)",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_class_server_method_table(table: ClassServersMethodTable) {
    initialize_table(
        &get_binding().class_server_method_table,
        table,
        "classes (Server level)",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_class_scene_method_table(table: ClassSceneMethodTable) {
    initialize_table(
        &get_binding().class_scene_method_table,
        table,
        "classes (Scene level)",
    )
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
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_class_editor_method_table(table: ClassEditorMethodTable) {
    initialize_table(
        &get_binding().class_editor_method_table,
        table,
        "classes (Editor level)",
    )
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
pub(crate) unsafe fn initialize_builtin_method_table(table: BuiltinMethodTable) {
    initialize_table(&get_binding().builtin_method_table, table, "builtins")
}
