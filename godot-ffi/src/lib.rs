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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Validations

// More validations in godot crate. #[cfg]s are checked in godot-core.

#[cfg(all(feature = "codegen-lazy-fptrs", feature = "experimental-threads"))]
compile_error!(
    "Cannot combine `lazy-function-tables` and `experimental-threads` features;\n\
    thread safety for lazy-loaded function pointers is not yet implemented."
);

#[cfg(all(
    feature = "experimental-wasm-nothreads",
    feature = "experimental-threads"
))]
compile_error!("Cannot use 'experimental-threads' with a nothreads Wasm build yet.");

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes,
)]
pub(crate) mod gen {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

pub mod conv;

mod extras;
mod global;
mod godot_ffi;
mod interface_init;
#[cfg(target_os = "linux")]
pub mod linux_reload_workaround;
mod opaque;
mod plugins;
mod string_cache;
mod toolbox;

#[doc(hidden)]
#[cfg(target_family = "wasm")]
pub use godot_macros::wasm_declare_init_fn;

// No-op otherwise.
#[doc(hidden)]
#[cfg(not(target_family = "wasm"))]
#[macro_export]
macro_rules! wasm_declare_init_fn {
    () => {};
}

pub use crate::godot_ffi::{
    ExtVariantType, GodotFfi, GodotNullableFfi, PrimitiveConversionError, PtrcallType,
};

// Method tables
pub use gen::table_builtins::*;
pub use gen::table_builtins_lifecycle::*;
pub use gen::table_editor_classes::*;
pub use gen::table_scene_classes::*;
pub use gen::table_servers_classes::*;
pub use gen::table_utilities::*;
pub use gen::virtual_consts as godot_virtual_consts;

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

mod binding;

pub use binding::*;

use binding::{
    initialize_binding, initialize_builtin_method_table, initialize_class_editor_method_table,
    initialize_class_scene_method_table, initialize_class_server_method_table, runtime_metadata,
};

#[cfg(not(wasm_nothreads))]
static MAIN_THREAD_ID: ManualInitCell<std::thread::ThreadId> = ManualInitCell::new();

/// Stage of the Godot initialization process.
///
/// Godot's initialization and deinitialization processes are split into multiple stages, like a stack. At each level,
/// a different amount of engine functionality is available. Deinitialization happens in reverse order.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum InitLevel {
    /// First level loaded by Godot. Builtin types are available, classes are not.
    Core,

    /// Second level loaded by Godot. Only server classes and builtins are available.
    Servers,

    /// Third level loaded by Godot. Most classes are available.
    Scene,

    /// Fourth level loaded by Godot, only in the editor. All classes are available.
    Editor,
}

impl InitLevel {
    #[doc(hidden)]
    pub fn from_sys(level: crate::GDExtensionInitializationLevel) -> Self {
        match level {
            crate::GDEXTENSION_INITIALIZATION_CORE => Self::Core,
            crate::GDEXTENSION_INITIALIZATION_SERVERS => Self::Servers,
            crate::GDEXTENSION_INITIALIZATION_SCENE => Self::Scene,
            crate::GDEXTENSION_INITIALIZATION_EDITOR => Self::Editor,
            _ => {
                eprintln!("WARNING: unknown initialization level {level}");
                Self::Scene
            }
        }
    }
    #[doc(hidden)]
    pub fn to_sys(self) -> crate::GDExtensionInitializationLevel {
        match self {
            Self::Core => crate::GDEXTENSION_INITIALIZATION_CORE,
            Self::Servers => crate::GDEXTENSION_INITIALIZATION_SERVERS,
            Self::Scene => crate::GDEXTENSION_INITIALIZATION_SCENE,
            Self::Editor => crate::GDEXTENSION_INITIALIZATION_EDITOR,
        }
    }
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
/// - The `get_proc_address` pointer must be a function pointer of type [`GDExtensionInterfaceGetProcAddress`] (valid for Godot 4.1+).
/// - The `library` pointer must be the pointer given by Godot at initialisation.
/// - This function must not be called from multiple threads.
/// - This function must be called before any use of [`get_library`].
pub unsafe fn initialize(
    get_proc_address: GDExtensionInterfaceGetProcAddress,
    library: GDExtensionClassLibraryPtr,
    config: GdextConfig,
) {
    out!("Initialize gdext...");

    out!(
        "Godot version against which gdext was compiled: {}",
        GdextBuild::godot_static_version_string()
    );

    // We want to initialize the main thread ID as early as possible.
    //
    // SAFETY: We set the main thread ID exactly once here and never again.
    #[cfg(not(wasm_nothreads))]
    unsafe {
        MAIN_THREAD_ID.set(std::thread::current().id())
    };

    // Before anything else: if we run into a Godot binary that's compiled differently from gdext, proceeding would be UB -> panic.
    interface_init::ensure_static_runtime_compatibility(get_proc_address);

    // SAFETY: `ensure_static_runtime_compatibility` succeeded.
    let version = unsafe { interface_init::runtime_version(get_proc_address) };
    out!("Godot version of GDExtension API at runtime: {version:?}");

    // SAFETY: `ensure_static_runtime_compatibility` succeeded.
    let interface = unsafe { interface_init::load_interface(get_proc_address) };
    out!("Loaded interface.");

    // SAFETY: The interface was successfully loaded from Godot, so we should be able to load the builtin lifecycle table.
    let global_method_table = unsafe { BuiltinLifecycleTable::load(&interface) };
    out!("Loaded global method table.");

    let mut string_names = StringCache::new(&interface, &global_method_table);

    // SAFETY: The interface was successfully loaded from Godot, so we should be able to load the utility function table.
    let utility_function_table =
        unsafe { UtilityFunctionTable::load(&interface, &mut string_names) };
    out!("Loaded utility function table.");

    // SAFETY: We do not touch `version` again after passing it to `new` here.
    let runtime_metadata = unsafe { GdextRuntimeMetadata::new(version) };

    let builtin_method_table = {
        #[cfg(feature = "codegen-lazy-fptrs")]
        {
            None // loaded later
        }
        #[cfg(not(feature = "codegen-lazy-fptrs"))]
        {
            // SAFETY: The interface was successfully loaded from Godot, so we should be able to load the builtin function table.
            let table = unsafe { BuiltinMethodTable::load(&interface, &mut string_names) };
            out!("Loaded builtin method table.");
            Some(table)
        }
    };

    drop(string_names);

    // SAFETY: This function is only called at initialization and not from multiple threads.
    unsafe {
        initialize_binding(GodotBinding::new(
            interface,
            library,
            global_method_table,
            utility_function_table,
            runtime_metadata,
            config,
        ))
    }

    if let Some(table) = builtin_method_table {
        // SAFETY: We initialized the bindings above and haven't called this function before.
        unsafe { initialize_builtin_method_table(table) }
    }

    out!("Assigned binding.");

    // Lazy case: load afterward because table's internal StringCache stores &'static references to the interface.
    #[cfg(feature = "codegen-lazy-fptrs")]
    {
        // SAFETY: The interface was successfully loaded from Godot, so we should be able to load the builtin function table.
        let table = unsafe { BuiltinMethodTable::load() };

        unsafe { initialize_builtin_method_table(table) }

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
/// - Must be called from the main thread.
/// - The interface must have been initialized with [`initialize`] before calling this function.
/// - Must only be called once for each `api_level`.
#[inline]
pub unsafe fn load_class_method_table(api_level: InitLevel) {
    out!("Load class method table for level '{:?}'...", api_level);
    let begin = std::time::Instant::now();

    #[cfg(not(feature = "codegen-lazy-fptrs"))]
    // SAFETY: The interface has been initialized.
    let interface = unsafe { get_interface() };

    #[cfg(not(feature = "codegen-lazy-fptrs"))]
    // SAFETY: The interface has been initialized.
    let mut string_names = StringCache::new(interface, unsafe { builtin_lifecycle_api() });

    let (class_count, method_count);
    match api_level {
        InitLevel::Core => {
            // Currently we don't need to do anything in `Core`, this may change in the future.
            class_count = 0;
            method_count = 0;
        }
        InitLevel::Servers => {
            // SAFETY: The interface has been initialized and this function hasn't been called before.
            unsafe {
                #[cfg(feature = "codegen-lazy-fptrs")]
                initialize_class_server_method_table(ClassServersMethodTable::load());
                #[cfg(not(feature = "codegen-lazy-fptrs"))]
                initialize_class_server_method_table(ClassServersMethodTable::load(
                    interface,
                    &mut string_names,
                ));
            }
            class_count = ClassServersMethodTable::CLASS_COUNT;
            method_count = ClassServersMethodTable::METHOD_COUNT;
        }
        InitLevel::Scene => {
            // SAFETY: The interface has been initialized and this function hasn't been called before.
            unsafe {
                #[cfg(feature = "codegen-lazy-fptrs")]
                initialize_class_scene_method_table(ClassSceneMethodTable::load());
                #[cfg(not(feature = "codegen-lazy-fptrs"))]
                initialize_class_scene_method_table(ClassSceneMethodTable::load(
                    interface,
                    &mut string_names,
                ));
            }
            class_count = ClassSceneMethodTable::CLASS_COUNT;
            method_count = ClassSceneMethodTable::METHOD_COUNT;
        }
        InitLevel::Editor => {
            // SAFETY: The interface has been initialized and this function hasn't been called before.
            unsafe {
                #[cfg(feature = "codegen-lazy-fptrs")]
                initialize_class_editor_method_table(ClassEditorMethodTable::load());
                #[cfg(not(feature = "codegen-lazy-fptrs"))]
                initialize_class_editor_method_table(ClassEditorMethodTable::load(
                    interface,
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
/// - Must be accessed from the main thread.
/// - The interface must have been initialized.
/// - The `Scene` api level must have been initialized.
/// - `os_class_sname` must be a valid `StringName` pointer.
/// - `tag_string` must be a valid type pointer of a `String` instance.
#[inline]
pub unsafe fn godot_has_feature(
    os_class_sname: GDExtensionConstStringNamePtr,
    tag_string: GDExtensionConstTypePtr,
) -> bool {
    // Issue a raw C call to OS.has_feature(tag_string).

    // SAFETY: Called from main thread, interface has been initialized, and the scene api has been initialized.
    let method_bind = unsafe { class_scene_api() }.os__has_feature();

    // SAFETY: Called from main thread, and interface has been initialized.
    let interface = unsafe { get_interface() };
    let get_singleton = interface.global_get_singleton.unwrap();
    let class_ptrcall = interface.object_method_bind_ptrcall.unwrap();

    // SAFETY: Interface has been initialized, and `Scene` has been initialized, so `get_singleton` can be called. `os_class_sname` is a valid
    // `StringName` pointer.
    let object_ptr = unsafe { get_singleton(os_class_sname) };
    let mut return_ptr = false;
    let type_ptrs = [tag_string];

    // SAFETY: We are properly passing arguments to make a ptrcall.
    unsafe {
        class_ptrcall(
            method_bind.0,
            object_ptr,
            type_ptrs.as_ptr(),
            return_ptr.sys_mut(),
        )
    }

    return_ptr
}

/// Get the [`ThreadId`](std::thread::ThreadId) of the main thread.
///
/// # Panics
/// - If it is called before the engine bindings have been initialized.
#[cfg(not(wasm_nothreads))]
pub fn main_thread_id() -> std::thread::ThreadId {
    assert!(
        MAIN_THREAD_ID.is_initialized(),
        "Godot engine not available; make sure you are not calling it from unit/doc tests"
    );

    // SAFETY: We initialized the cell during library initialization, before any other code is executed.
    let thread_id = unsafe { MAIN_THREAD_ID.get_unchecked() };

    *thread_id
}

/// Check if the current thread is the main thread.
///
/// # Panics
/// - If it is called before the engine bindings have been initialized.
pub fn is_main_thread() -> bool {
    #[cfg(not(wasm_nothreads))]
    {
        std::thread::current().id() == main_thread_id()
    }

    #[cfg(wasm_nothreads)]
    {
        true
    }
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
