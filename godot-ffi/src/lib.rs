/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Low level bindings to the provided C core API

#![cfg_attr(test, allow(unused))]

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes
)]
pub(crate) mod gen {
    pub mod table_builtins;
    pub mod table_builtins_lifecycle;
    pub mod table_servers_classes;
    pub mod table_scene_classes;
    pub mod table_editor_classes;
    pub mod table_utilities;

    pub mod central;
    pub mod gdextension_interface;
    pub mod interface;
}

mod compat;
mod extras;
mod global;
mod godot_ffi;
mod opaque;
mod plugins;
mod string_cache;
mod toolbox;

use compat::BindingCompat;

// See https://github.com/dtolnay/paste/issues/69#issuecomment-962418430
// and https://users.rust-lang.org/t/proc-macros-using-third-party-crate/42465/4
#[doc(hidden)]
pub use paste;

#[doc(hidden)]
#[cfg(target_family = "wasm")]
pub use gensym::gensym;

pub use crate::godot_ffi::{
    from_sys_init_or_init_default, GodotFfi, GodotNullableFfi, PrimitiveConversionError,
    PtrcallType,
};

// Method tables
pub use gen::table_builtins::*;
pub use gen::table_builtins_lifecycle::*;
pub use gen::table_editor_classes::*;
pub use gen::table_scene_classes::*;
pub use gen::table_servers_classes::*;
pub use gen::table_utilities::*;

// Other
pub use extras::*;
pub use gen::central::*;
pub use gen::gdextension_interface::*;
pub use gen::interface::*;
pub use global::*;
pub use string_cache::StringCache;
pub use toolbox::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API to access Godot via FFI

mod binding {
    // Ensure both crates are checked regardless of cfg, for the sake of development convenience.
    #[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
    mod multi_threaded;
    #[cfg_attr(feature = "experimental-threads", allow(dead_code))]
    mod single_threaded;

    use crate::{
        BuiltinLifecycleTable, BuiltinMethodTable, ClassEditorMethodTable, ClassSceneMethodTable,
        ClassServersMethodTable, GDExtensionInterface, GdextRuntimeMetadata, UtilityFunctionTable,
    };

    #[cfg(feature = "experimental-threads")]
    use multi_threaded::BindingStorage;
    #[cfg(feature = "experimental-threads")]
    pub use multi_threaded::{GdextConfig, GodotBinding};
    #[cfg(not(feature = "experimental-threads"))]
    use single_threaded::BindingStorage;
    #[cfg(not(feature = "experimental-threads"))]
    pub use single_threaded::{GdextConfig, GodotBinding};

    /// Newtype around `GDExtensionClassLibraryPtr` so we can implement `Sync` and `Send` manually for this.
    struct GDExtensionClassLibraryPtr(crate::GDExtensionClassLibraryPtr);

    // SAFETY: It is safe to have access to the library pointer from any thread, as we ensure any access is thread-safe through other means.
    // Even without "experimental-threads", there is no way without `unsafe` to cause UB by accessing the library from different threads.
    unsafe impl Sync for GDExtensionClassLibraryPtr {}
    // SAFETY: See `Sync` impl safety doc.
    unsafe impl Send for GDExtensionClassLibraryPtr {}

    /// Initializes the Godot binding.
    ///
    /// Most other functions in this module rely on this function being called first as a safety condition.
    pub(crate) fn initialize_binding(binding: GodotBinding) {
        BindingStorage::initialize(binding).expect("`initialize` must only be called once");
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn get_binding() -> &'static GodotBinding {
        BindingStorage::get_binding_unchecked()
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    pub(crate) unsafe fn initialize_class_server_method_table(table: ClassServersMethodTable) {
        // SAFETY: `get_binding` has the same preconditions as this function.
        let binding = unsafe { get_binding() };

        binding
            .class_server_method_table
            .set(table)
            .ok()
            .expect("server method table should only be initialized once")
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    pub(crate) unsafe fn initialize_class_scene_method_table(table: ClassSceneMethodTable) {
        // SAFETY: `get_binding` has the same preconditions as this function.
        let binding = unsafe { get_binding() };

        binding
            .class_scene_method_table
            .set(table)
            .ok()
            .expect("scene method table should only be initialized once")
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    pub(crate) unsafe fn initialize_class_editor_method_table(table: ClassEditorMethodTable) {
        // SAFETY: `get_binding` has the same preconditions as this function.
        let binding = unsafe { get_binding() };

        binding
            .class_editor_method_table
            .set(table)
            .ok()
            .expect("editor method table should only be initialized once")
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    pub(crate) unsafe fn initialize_builtin_method_table(table: BuiltinMethodTable) {
        // SAFETY: `get_binding` has the same preconditions as this function.
        let binding = unsafe { get_binding() };

        binding
            .builtin_method_table
            .set(table)
            .ok()
            .expect("builtin method table should only be initialized once")
    }

    pub fn is_initialized() -> bool {
        BindingStorage::is_initialized()
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn get_interface() -> &'static GDExtensionInterface {
        &get_binding().interface
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn get_library() -> crate::GDExtensionClassLibraryPtr {
        get_binding().library.0
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn builtin_lifecycle_api() -> &'static BuiltinLifecycleTable {
        &get_binding().global_method_table
    }

    /// # Safety
    ///
    /// - The Godot binding must have been initialized before calling this function.
    /// - [`initialize_class_server_method_table`] must have been called before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn class_servers_api() -> &'static ClassServersMethodTable {
        let table = get_binding().class_server_method_table.get();

        debug_assert!(
            table.is_some(),
            "cannot fetch classes; init level 'Servers' not yet loaded"
        );

        table.unwrap_unchecked()
    }

    /// # Safety
    ///
    /// - The Godot binding must have been initialized before calling this function.
    /// - [`initialize_class_scene_method_table`] must have been called before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn class_scene_api() -> &'static ClassSceneMethodTable {
        let table = get_binding().class_scene_method_table.get();

        debug_assert!(
            table.is_some(),
            "cannot fetch classes; init level 'Scene' not yet loaded"
        );

        table.unwrap_unchecked()
    }

    /// # Safety
    ///
    /// - The Godot binding must have been initialized before calling this function.
    /// - [`initialize_class_editor_method_table`] must have been called before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn class_editor_api() -> &'static ClassEditorMethodTable {
        let table = get_binding().class_editor_method_table.get();

        debug_assert!(
            table.is_some(),
            "cannot fetch classes; init level 'Editor' not yet loaded"
        );

        table.unwrap_unchecked()
    }

    /// # Safety
    ///
    /// - The Godot binding must have been initialized before calling this function.
    /// - [`initialize_builtin_method_table`] must have been called before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn builtin_method_table() -> &'static BuiltinMethodTable {
        get_binding().builtin_method_table.get().unwrap_unchecked()
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub unsafe fn utility_function_table() -> &'static UtilityFunctionTable {
        &get_binding().utility_function_table
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline(always)]
    pub(crate) unsafe fn runtime_metadata() -> &'static GdextRuntimeMetadata {
        &get_binding().runtime_metadata
    }

    /// # Safety
    ///
    /// The Godot binding must have been initialized before calling this function.
    ///
    /// If the "experimental-threads" is not enabled then this must be called from the same thread that the bindings were initialized from.
    #[inline]
    pub unsafe fn config() -> &'static GdextConfig {
        &get_binding().config
    }
}

pub use binding::*;
use binding::{
    initialize_binding, initialize_builtin_method_table, initialize_class_editor_method_table,
    initialize_class_scene_method_table, initialize_class_server_method_table, runtime_metadata,
};

#[derive(Debug)]
pub enum ClassApiLevel {
    Server,
    Scene,
    Editor,
}

pub struct GdextRuntimeMetadata {
    godot_version: GDExtensionGodotVersion,
}

impl GdextRuntimeMetadata {
    /// # Safety
    ///
    /// - The `string` field of `godot_version` must not be written to while this struct exists.
    /// - The `string` field of `godot_version` must be safe to read from while this struct exists.
    pub unsafe fn new(godot_version: GDExtensionGodotVersion) -> Self {
        Self { godot_version }
    }
}

// SAFETY: The `string` pointer in `godot_version` is only ever read from while the struct exists, so we cannot have any race conditions.
unsafe impl Sync for GdextRuntimeMetadata {}
// SAFETY: See `Sync` impl safety doc.
unsafe impl Send for GdextRuntimeMetadata {}

/// # Safety
///
/// - The `interface` pointer must be either:
///   - a data pointer to a [`GDExtensionInterface`] object (for Godot 4.0.x)
///   - a function pointer of type [`GDExtensionInterfaceGetProcAddress`] (for Godot 4.1+)
/// - The `library` pointer must be the pointer given by Godot at initialisation.
/// - This function must not be called from multiple threads.
/// - This function must be called before any use of [`get_library`].
pub unsafe fn initialize(
    compat: InitCompat,
    library: GDExtensionClassLibraryPtr,
    config: GdextConfig,
) {
    out!("Initialize gdext...");

    out!(
        "Godot version against which gdext was compiled: {}",
        GdextBuild::godot_static_version_string()
    );

    // Before anything else: if we run into a Godot binary that's compiled differently from gdext, proceeding would be UB -> panic.
    compat.ensure_static_runtime_compatibility();

    let version = compat.runtime_version();
    out!("Godot version of GDExtension API at runtime: {version:?}");

    let interface = compat.load_interface();
    out!("Loaded interface.");

    let global_method_table = BuiltinLifecycleTable::load(&interface);
    out!("Loaded global method table.");

    let mut string_names = StringCache::new(&interface, &global_method_table);

    let utility_function_table = UtilityFunctionTable::load(&interface, &mut string_names);
    out!("Loaded utility function table.");

    let runtime_metadata = GdextRuntimeMetadata::new(version);

    let builtin_method_table = {
        #[cfg(feature = "codegen-lazy-fptrs")]
        {
            None // loaded later
        }
        #[cfg(not(feature = "codegen-lazy-fptrs"))]
        {
            let table = BuiltinMethodTable::load(&interface, &mut string_names);
            out!("Loaded builtin method table.");
            Some(table)
        }
    };

    drop(string_names);

    initialize_binding(GodotBinding::new(
        interface,
        library,
        global_method_table,
        utility_function_table,
        runtime_metadata,
        config,
    ));

    if let Some(table) = builtin_method_table {
        initialize_builtin_method_table(table);
    }

    out!("Assigned binding.");

    // Lazy case: load afterwards because table's internal StringCache stores &'static references to the interface.
    #[cfg(feature = "codegen-lazy-fptrs")]
    {
        let table = BuiltinMethodTable::load();

        initialize_builtin_method_table(table);

        out!("Loaded builtin method table (lazily).");
    }

    print_preamble(version);
}

fn print_preamble(version: GDExtensionGodotVersion) {
    let api_version: &'static str = GdextBuild::godot_static_version_string();
    let runtime_version = read_version_string(&version);

    println!("Initialize godot-rust (API {api_version}, runtime {runtime_version})");
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline]
pub unsafe fn load_class_method_table(api_level: ClassApiLevel) {
    out!("Load class method table for level '{:?}'...", api_level);
    let begin = std::time::Instant::now();

    #[cfg(not(feature = "codegen-lazy-fptrs"))]
    let mut string_names = StringCache::new(get_interface(), builtin_lifecycle_api());

    let (class_count, method_count);
    match api_level {
        ClassApiLevel::Server => {
            #[cfg(feature = "codegen-lazy-fptrs")]
            {
                initialize_class_server_method_table(ClassServersMethodTable::load());
            }
            #[cfg(not(feature = "codegen-lazy-fptrs"))]
            {
                initialize_class_server_method_table(ClassServersMethodTable::load(
                    get_interface(),
                    &mut string_names,
                ));
            }
            class_count = ClassServersMethodTable::CLASS_COUNT;
            method_count = ClassServersMethodTable::METHOD_COUNT;
        }
        ClassApiLevel::Scene => {
            #[cfg(feature = "codegen-lazy-fptrs")]
            {
                initialize_class_scene_method_table(ClassSceneMethodTable::load());
            }
            #[cfg(not(feature = "codegen-lazy-fptrs"))]
            {
                initialize_class_scene_method_table(ClassSceneMethodTable::load(
                    get_interface(),
                    &mut string_names,
                ));
            }
            class_count = ClassSceneMethodTable::CLASS_COUNT;
            method_count = ClassSceneMethodTable::METHOD_COUNT;
        }
        ClassApiLevel::Editor => {
            #[cfg(feature = "codegen-lazy-fptrs")]
            {
                initialize_class_editor_method_table(ClassEditorMethodTable::load());
            }
            #[cfg(not(feature = "codegen-lazy-fptrs"))]
            {
                initialize_class_editor_method_table(ClassEditorMethodTable::load(
                    get_interface(),
                    &mut string_names,
                ));
            }
            class_count = ClassEditorMethodTable::CLASS_COUNT;
            method_count = ClassEditorMethodTable::METHOD_COUNT;
        }
    }

    let _elapsed = std::time::Instant::now() - begin;
    out!(
        "{:?} level: loaded {} classes and {} methods in {}s.",
        api_level,
        class_count,
        method_count,
        _elapsed.as_secs_f64()
    );
}

/// # Safety
///
/// Must be accessed from the main thread, and the interface must have been initialized.
/// `tag_string` must be a valid type pointer of a `String` instance.
#[inline]
pub unsafe fn godot_has_feature(
    os_class_sname: GDExtensionConstStringNamePtr,
    tag_string: GDExtensionConstTypePtr,
) -> bool {
    // Issue a raw C call to OS.has_feature(tag_string).

    let method_bind = class_scene_api().os__has_feature();
    let get_singleton = get_interface().global_get_singleton.unwrap();
    let class_ptrcall = get_interface().object_method_bind_ptrcall.unwrap();

    let object_ptr = get_singleton(os_class_sname);
    let mut return_ptr = false;
    let type_ptrs = [tag_string];

    class_ptrcall(
        method_bind.0,
        object_ptr,
        type_ptrs.as_ptr(),
        return_ptr.sys_mut(),
    );

    return_ptr
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to access low-level function bindings

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_fn {
    ($name:ident $(@1)?) => {
        $crate::builtin_lifecycle_api().$name
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_call {
        ($name:ident ( $($args:expr),* $(,)? )) => {
            ($crate::builtin_lifecycle_api().$name)( $($args),* )
        };
    }

#[macro_export]
#[doc(hidden)]
macro_rules! interface_fn {
    ($name:ident) => {{
        unsafe { $crate::get_interface().$name.unwrap_unchecked() }
    }};
}
