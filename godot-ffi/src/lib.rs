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
mod gdextension_plus;
mod godot_ffi;
mod opaque;
mod plugins;
mod string_cache;
mod toolbox;

use compat::BindingCompat;
use std::cell;
use std::ffi::CStr;

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
pub use gdextension_plus::*;
pub use gen::central::*;
pub use gen::gdextension_interface::*;
pub use gen::interface::*;
pub use string_cache::StringCache;
pub use toolbox::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API to access Godot via FFI

#[derive(Debug)]
pub enum ClassApiLevel {
    Server,
    Scene,
    Editor,
}

struct GodotBinding {
    interface: GDExtensionInterface,
    library: GDExtensionClassLibraryPtr,
    global_method_table: BuiltinLifecycleTable,
    class_server_method_table: Option<ClassServersMethodTable>, // late-init
    class_scene_method_table: Option<ClassSceneMethodTable>,    // late-init
    class_editor_method_table: Option<ClassEditorMethodTable>,  // late-init
    builtin_method_table: Option<BuiltinMethodTable>,           // late-init
    utility_function_table: UtilityFunctionTable,
    runtime_metadata: GdextRuntimeMetadata,
    config: GdextConfig,
}

struct GdextRuntimeMetadata {
    godot_version: GDExtensionGodotVersion,
}

pub struct GdextConfig {
    pub tool_only_in_editor: bool,
    pub is_editor: cell::OnceCell<bool>,
}

/// Late-init globals
// Note: static mut is _very_ dangerous. Here a bit less so, since modification happens only once (during init) and no
// &mut references are handed out (except for registry, see below). Overall, UnsafeCell/RefCell + Sync might be a safer abstraction.
static mut BINDING: Option<GodotBinding> = None;

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

    let runtime_metadata = GdextRuntimeMetadata {
        godot_version: version,
    };

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

    BINDING = Some(GodotBinding {
        interface,
        global_method_table,
        class_server_method_table: None,
        class_scene_method_table: None,
        class_editor_method_table: None,
        builtin_method_table,
        utility_function_table,
        library,
        runtime_metadata,
        config,
    });
    out!("Assigned binding.");

    // Lazy case: load afterwards because table's internal StringCache stores &'static references to the interface.
    #[cfg(feature = "codegen-lazy-fptrs")]
    {
        let builtin_method_table = BuiltinMethodTable::load();
        BINDING.as_mut().unwrap().builtin_method_table = Some(builtin_method_table);
        out!("Loaded builtin method table (lazily).");
    }

    println!(
        "Initialize GDExtension API for Rust: {}",
        CStr::from_ptr(version.string)
            .to_str()
            .expect("unknown Godot version")
    );
}

/// # Safety
///
/// Must be called from the same thread as `initialize()` previously.
pub unsafe fn is_initialized() -> bool {
    BINDING.is_some()
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn get_interface() -> &'static GDExtensionInterface {
    &unwrap_ref_unchecked(&BINDING).interface
}

/// # Safety
///
/// The library must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn get_library() -> GDExtensionClassLibraryPtr {
    unwrap_ref_unchecked(&BINDING).library
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn method_table() -> &'static BuiltinLifecycleTable {
    &unwrap_ref_unchecked(&BINDING).global_method_table
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn class_servers_api() -> &'static ClassServersMethodTable {
    let table = &unwrap_ref_unchecked(&BINDING).class_server_method_table;
    debug_assert!(
        table.is_some(),
        "cannot fetch classes; init level 'Servers' not yet loaded"
    );

    table.as_ref().unwrap_unchecked()
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn class_scene_api() -> &'static ClassSceneMethodTable {
    let table = &unwrap_ref_unchecked(&BINDING).class_scene_method_table;
    debug_assert!(
        table.is_some(),
        "cannot fetch classes; init level 'Scene' not yet loaded"
    );

    table.as_ref().unwrap_unchecked()
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn class_editor_api() -> &'static ClassEditorMethodTable {
    let table = &unwrap_ref_unchecked(&BINDING).class_editor_method_table;
    debug_assert!(
        table.is_some(),
        "cannot fetch classes; init level 'Editor' not yet loaded"
    );

    table.as_ref().unwrap_unchecked()
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn builtin_method_table() -> &'static BuiltinMethodTable {
    unwrap_ref_unchecked(&BINDING)
        .builtin_method_table
        .as_ref()
        .unwrap_unchecked()
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn utility_function_table() -> &'static UtilityFunctionTable {
    &unwrap_ref_unchecked(&BINDING).utility_function_table
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn load_class_method_table(api_level: ClassApiLevel) {
    let binding = unwrap_ref_unchecked_mut(&mut BINDING);

    out!("Load class method table for level '{:?}'...", api_level);
    let begin = std::time::Instant::now();

    #[cfg(not(feature = "codegen-lazy-fptrs"))]
    let mut string_names = StringCache::new(&binding.interface, &binding.global_method_table);

    let (class_count, method_count);
    match api_level {
        ClassApiLevel::Server => {
            #[cfg(feature = "codegen-lazy-fptrs")]
            {
                binding.class_server_method_table = Some(ClassServersMethodTable::load());
            }
            #[cfg(not(feature = "codegen-lazy-fptrs"))]
            {
                binding.class_server_method_table = Some(ClassServersMethodTable::load(
                    &binding.interface,
                    &mut string_names,
                ));
            }
            class_count = ClassServersMethodTable::CLASS_COUNT;
            method_count = ClassServersMethodTable::METHOD_COUNT;
        }
        ClassApiLevel::Scene => {
            #[cfg(feature = "codegen-lazy-fptrs")]
            {
                binding.class_scene_method_table = Some(ClassSceneMethodTable::load());
            }
            #[cfg(not(feature = "codegen-lazy-fptrs"))]
            {
                binding.class_scene_method_table = Some(ClassSceneMethodTable::load(
                    &binding.interface,
                    &mut string_names,
                ));
            }
            class_count = ClassSceneMethodTable::CLASS_COUNT;
            method_count = ClassSceneMethodTable::METHOD_COUNT;
        }
        ClassApiLevel::Editor => {
            #[cfg(feature = "codegen-lazy-fptrs")]
            {
                binding.class_editor_method_table = Some(ClassEditorMethodTable::load());
            }
            #[cfg(not(feature = "codegen-lazy-fptrs"))]
            {
                binding.class_editor_method_table = Some(ClassEditorMethodTable::load(
                    &binding.interface,
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
#[inline(always)]
pub(crate) unsafe fn runtime_metadata() -> &'static GdextRuntimeMetadata {
    &BINDING.as_ref().unwrap().runtime_metadata
}

/// # Safety
///
/// Must be accessed from the main thread, and the interface must have been initialized.
#[inline]
pub unsafe fn config() -> &'static GdextConfig {
    &BINDING.as_ref().unwrap().config
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to access low-level function bindings

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_fn {
    ($name:ident $(@1)?) => {
        $crate::method_table().$name
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_call {
        ($name:ident ( $($args:expr),* $(,)? )) => {
            ($crate::method_table().$name)( $($args),* )
        };
    }

#[macro_export]
#[doc(hidden)]
macro_rules! interface_fn {
    ($name:ident) => {{
        unsafe { $crate::get_interface().$name.unwrap_unchecked() }
    }};
}
