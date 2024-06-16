/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Internal crate of [**godot-rust**](https://godot-rust.github.io)
//!
//! Do not depend on this crate directly, instead use the `godot` crate.
//! No SemVer or other guarantees are provided.
//!
//! # Contributor docs
//!
//! Low level bindings to the provided C core API.

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
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

mod compat;
mod extras;
mod global;
mod godot_ffi;
#[cfg(target_os = "linux")]
pub mod linux_reload_workaround;
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
    new_with_uninit_or_init, GodotFfi, GodotNullableFfi, PrimitiveConversionError, PtrcallType,
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

#[cfg(before_api = "4.1")]
mod godot_4_0_imported {
    // SAFETY: In Godot 4.0.4, the extension interface stores a c_char pointer. This is safe to access from different threads, as no
    // mutation happens after initialization. This was changed in 4.1, so we don't need to manually implement `Sync` or `Send` after 4.0.
    // Instead, we rely on Rust to infer that it is `Sync` and `Send`.
    unsafe impl Sync for super::GDExtensionInterface {}

    // SAFETY: See `Sync` impl.
    unsafe impl Send for super::GDExtensionInterface {}

    // Re-import polyfills so that code can use the symbols as if 4.0 would natively define them.
    pub use super::compat::InitCompat;
    pub(crate) use super::compat::{
        GDExtensionInterfaceClassdbGetMethodBind, GDExtensionInterfaceVariantGetPtrBuiltinMethod,
    };
}

#[cfg(before_api = "4.1")]
pub use godot_4_0_imported::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API to access Godot via FFI

mod binding;

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

/// Initializes the library.
///
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

/// Deinitializes the library.
///
/// Does not perform much logic, mostly used for consistency:
/// - Ensure that the binding is not accessed after it has been deinitialized.
/// - Allow re-initialization for hot-reloading on Linux.
///
/// # Safety
/// See [`initialize`].
pub unsafe fn deinitialize() {
    deinitialize_binding()
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
