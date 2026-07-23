/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{
    BuiltinLifecycleTable, BuiltinMethodTable, ClassCoreMethodTable, ClassEditorMethodTable,
    ClassSceneMethodTable, ClassServersMethodTable, GDExtensionClassLibraryPtr,
    GDExtensionConstTypePtr, GDExtensionInterface, GDExtensionTypePtr,
    GDExtensionUninitializedTypePtr, GDExtensionUninitializedVariantPtr, GDExtensionVariantPtr,
    GdextRuntimeMetadata, ManualInitCell, UtilityFunctionTable,
};

#[cfg(feature = "experimental-threads")]
mod multi_threaded;
#[cfg(not(feature = "experimental-threads"))]
mod single_threaded;

#[cfg(feature = "experimental-threads")]
use multi_threaded::BindingStorage;
#[cfg(not(feature = "experimental-threads"))]
use single_threaded::BindingStorage;

pub struct GdextConfig {
    /// True if only `#[class(tool)]` classes are active in editor; false if all classes are.
    pub tool_only_in_editor: bool,
}

impl GdextConfig {
    pub fn new(tool_only_in_editor: bool) -> Self {
        Self {
            tool_only_in_editor,
        }
    }
}

/// Panics if the binding is not currently live. Shared by both binding storages for a consistent live check.
#[cfg(safeguards_balanced)]
#[inline(always)]
pub(super) fn assert_binding_live(initialized: &std::sync::atomic::AtomicBool) {
    if !initialized.load(std::sync::atomic::Ordering::Acquire) {
        not_live_panic();
    }
}

/// Cold failure path, so the hot check stays a predicted-not-taken branch.
#[cfg(safeguards_balanced)]
#[cold]
#[inline(never)]
fn not_live_panic() -> ! {
    panic!(
        "Godot binding accessed before initialization or after deinitialization. \
        This typically means a `#[ctor]`/`#[dtor]` constructor, a library destructor, or a leftover user thread touched the Godot API \
        outside the engine's load/unload window."
    )
}

#[cfg(all(test, safeguards_balanced))]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::assert_binding_live;

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

// Note, this is `Sync` and `Send` when "experimental-threads" is enabled because all its fields are. We have avoided implementing `Sync`
// and `Send` for `GodotBinding` as that could hide issues if any of the field types are changed to no longer be sync/send, but the manual
// implementation for `GodotBinding` wouldn't detect that.
pub(crate) struct GodotBinding {
    interface: GDExtensionInterface,
    get_proc_address: crate::GDExtensionInterfaceGetProcAddress,
    library: ClassLibraryPtr,
    global_method_table: BuiltinLifecycleTable,
    class_core_method_table: ManualInitCell<ClassCoreMethodTable>,
    class_servers_method_table: ManualInitCell<ClassServersMethodTable>,
    class_scene_method_table: ManualInitCell<ClassSceneMethodTable>,
    class_editor_method_table: ManualInitCell<ClassEditorMethodTable>,
    builtin_method_table: ManualInitCell<BuiltinMethodTable>,
    utility_function_table: UtilityFunctionTable,
    runtime_metadata: GdextRuntimeMetadata,
    config: GdextConfig,
    thread_safe_lifecycle: ThreadSafeLifecycle,
}

// The reviewed thread-safe subset of `BuiltinLifecycleTable`. Field names must match the table 1:1; listed once here, so the struct
// declaration and `from_table` copy cannot drift. Each entry being a separate field is the guardrail: macros can only reach reviewed functions.
macro_rules! thread_safe_lifecycle {
    ($( $field:ident: $sig:ty ),* $(,)?) => {
        #[derive(Copy, Clone)]
        pub struct ThreadSafeLifecycle {
            $( pub $field: $sig, )*
        }

        impl ThreadSafeLifecycle {
            /// Picks the reviewed thread-safe subset out of the full lifecycle table. Built once at binding init, handed out by reference.
            fn from_table(table: &BuiltinLifecycleTable) -> Self {
                Self { $( $field: table.$field, )* }
            }
        }
    };
}

thread_safe_lifecycle! {
    string_construct_default: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_construct_copy: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_destroy: unsafe extern "C" fn(GDExtensionTypePtr),
    string_operator_equal: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    string_operator_less: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    string_from_string_name: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_from_node_path: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_to_variant: unsafe extern "C" fn(GDExtensionUninitializedVariantPtr, GDExtensionTypePtr),
    string_from_variant: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, GDExtensionVariantPtr),
    string_name_construct_default: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_name_construct_copy: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_name_destroy: unsafe extern "C" fn(GDExtensionTypePtr),
    string_name_operator_equal: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    string_name_operator_less: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    string_name_from_string: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
    string_name_to_variant: unsafe extern "C" fn(GDExtensionUninitializedVariantPtr, GDExtensionTypePtr),
    string_name_from_variant: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, GDExtensionVariantPtr),
}

impl GodotBinding {
    pub fn new(
        interface: GDExtensionInterface,
        get_proc_address: crate::GDExtensionInterfaceGetProcAddress,
        library: GDExtensionClassLibraryPtr,
        global_method_table: BuiltinLifecycleTable,
        utility_function_table: UtilityFunctionTable,
        runtime_metadata: GdextRuntimeMetadata,
        config: GdextConfig,
    ) -> Self {
        let thread_safe_lifecycle = ThreadSafeLifecycle::from_table(&global_method_table);

        Self {
            interface,
            get_proc_address,
            library: ClassLibraryPtr(library),
            global_method_table,
            thread_safe_lifecycle,
            class_core_method_table: ManualInitCell::new(),
            class_servers_method_table: ManualInitCell::new(),
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
unsafe fn initialize_table<T>(table: &ManualInitCell<T>, value: T, _what: &str) {
    crate::strict_assert!(
        !table.is_initialized(),
        "method table for {_what} should only be initialized once"
    );

    // SAFETY: One-time, non-shared access during init.
    table.set(value)
}

/// # Safety
/// The table must have been initialized.
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
unsafe fn get_table<T>(table: &'static ManualInitCell<T>, _msg: &str) -> &'static T {
    crate::strict_assert!(table.is_initialized(), "{_msg}");

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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn get_interface() -> &'static GDExtensionInterface {
    &get_binding().interface
}

/// Interface for FFI functions that touch shared engine/scene state and must run on the main thread.
///
/// Asserts main thread (non-disengaged profiles) plus binding-live check. For `classdb_*`, `object_method_bind_*`, etc.
/// Not a thread-safe alternative; use [`thread_safe()`] for functions that only touch caller-owned memory.
///
/// # Safety
/// The Godot binding must have been initialized before calling this function.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn on_main() -> &'static GDExtensionInterface {
    &get_binding().interface
}

/// Access the interface for thread-safe FFI functions (they only touch caller-owned memory).
///
/// Skips the main-thread assertion, keeping only the binding-live check. Default to [`on_main`] until a function is reviewed thread-safe.
/// Returns the full interface, so the restriction holds by convention only. TODO(v0.6): codegen a typed subset like [`thread_safe_lifecycle`],
/// so misuse becomes a compile error.
///
/// # Safety
/// The Godot binding must have been initialized before calling this function, and the accessed function must not touch shared engine state.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn thread_safe() -> &'static GDExtensionInterface {
    &get_binding_thread_safe().interface
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn get_library() -> crate::GDExtensionClassLibraryPtr {
    get_binding().library.0
}

/// Looks up an FFI function by name using `get_proc_address`.
///
/// Argument must be null-terminated, e.g. `b"my_ffi_function\0"`.
///
/// Returns `None` if the function is not available (e.g. the runtime Godot version predates the function). Necessary only in niche cases
/// where polyfill/cross-version behavior needs to be emulated. Likely obsolete once there's `gdextension_interface.json`.
///
/// # Safety
/// The Godot binding must have been initialized before calling this function, and the function must be run on the main thread.
#[inline]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn get_ffi_ptr_by_cstr(name: &[u8]) -> crate::GDExtensionInterfaceFunctionPtr {
    let get_proc_address = get_binding()
        .get_proc_address
        .expect("get_proc_address should be available");

    get_proc_address(crate::c_str(name))
}

/// Access the builtin lifecycle table.
///
/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn builtin_lifecycle_api() -> &'static BuiltinLifecycleTable {
    &get_binding().global_method_table
}

/// Access the reviewed builtin lifecycle functions that are thread-safe.
///
/// Intentionally a narrow subset, not the full lifecycle table; additions must be reviewed individually.
///
/// # Safety
/// The Godot binding must have been initialized before calling this function, and the accessed function must not touch shared engine state.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn thread_safe_lifecycle() -> &'static ThreadSafeLifecycle {
    &get_binding_thread_safe().thread_safe_lifecycle
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - The class servers method table must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn class_servers_api() -> &'static ClassServersMethodTable {
    get_table(
        &get_binding().class_servers_method_table,
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn utility_function_table() -> &'static UtilityFunctionTable {
    &get_binding().utility_function_table
}

/// Access the utility-function table for functions reviewed as thread-safe (currently the print group and `str`).
///
/// Skips the main-thread assertion, keeping only the binding-live check. Most utility functions touch shared engine state and must therefore
/// keep going through [`utility_function_table`]; codegen routes only the individually reviewed ones here (see `is_utility_function_thread_safe`
/// in `godot-codegen`).
///
/// # Safety
///
/// The Godot binding must have been initialized before calling this function, and the accessed function must not touch shared engine state.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn utility_function_table_thread_safe() -> &'static UtilityFunctionTable {
    &get_binding_thread_safe().utility_function_table
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub unsafe fn config() -> &'static GdextConfig {
    &get_binding().config
}

/// Returns true if godot-rust bindings are initialized at a very low level.
///
/// The bindings are initialized when the library is loaded and deinitialized when it is unloaded (which may happen e.g. during a hot reload).
/// Do not use this as a general check whether certain Godot APIs can be used -- this is more complex and may depend on class/singleton.
#[inline]
pub fn is_godot_initialized() -> bool {
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn initialize_binding(binding: GodotBinding) {
    BindingStorage::initialize(binding);
}

/// Deinitializes the Godot binding.
///
/// # Safety
///
/// See [`initialize_binding`].
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn deinitialize_binding() {
    BindingStorage::deinitialize();
}

/// # Safety
///
/// The Godot binding must have been initialized before calling this function.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn get_binding() -> &'static GodotBinding {
    // Restricted access: assert main thread (no-op in multi-threaded builds), then perform the binding-live check.
    BindingStorage::ensure_main_thread();
    BindingStorage::get_binding_unchecked()
}

/// Like [`get_binding`], but without the main-thread assertion -- for thread-safe FFI functions.
///
/// # Safety
/// The Godot binding must have been initialized before calling this function. The accessed FFI function must only touch caller-owned memory
/// (e.g. builtin value-type ctors/dtors, mem alloc/free, type constructors), not shared engine or scene state.
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn get_binding_thread_safe() -> &'static GodotBinding {
    BindingStorage::get_binding_unchecked()
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn initialize_class_servers_method_table(table: ClassServersMethodTable) {
    initialize_table(
        &get_binding().class_servers_method_table,
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn runtime_metadata() -> &'static GdextRuntimeMetadata {
    &get_binding().runtime_metadata
}

/// # Safety
///
/// - The Godot binding must have been initialized before calling this function.
/// - Must only be called once.
///
/// If "experimental-threads" is not enabled, then this must be called from the same thread that the bindings were initialized from.
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
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
#[allow(unsafe_op_in_unsafe_fn)] // Safety preconditions forwarded 1:1.
pub(crate) unsafe fn initialize_builtin_method_table(table: BuiltinMethodTable) {
    initialize_table(&get_binding().builtin_method_table, table, "builtins")
}
