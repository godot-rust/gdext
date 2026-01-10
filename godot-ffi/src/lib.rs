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

mod assertions;
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

// Other
pub use extras::*;
pub use gen::central::*;
pub use gen::gdextension_interface::*;
pub use gen::interface::*;
// Method tables
pub use gen::table_builtins::*;
pub use gen::table_builtins_lifecycle::*;
pub use gen::table_core_classes::*;
pub use gen::table_editor_classes::*;
pub use gen::table_scene_classes::*;
pub use gen::table_servers_classes::*;
pub use gen::table_utilities::*;
pub use global::*;
pub use init_level::*;
pub use string_cache::StringCache;
pub use toolbox::*;

pub use crate::godot_ffi::{
    ExtVariantType, GodotFfi, GodotNullableFfi, PrimitiveConversionError, PtrcallType,
};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API to access Godot via FFI

mod binding;
mod init_level;

pub use binding::*;
use binding::{
    initialize_binding, initialize_builtin_method_table, initialize_class_core_method_table,
    initialize_class_editor_method_table, initialize_class_scene_method_table,
    initialize_class_server_method_table, runtime_metadata,
};

#[cfg(not(wasm_nothreads))]
static MAIN_THREAD_ID: ManualInitCell<std::thread::ThreadId> = ManualInitCell::new();

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deferred editor messages

/// Warnings/errors collected during startup, and deferred until editor UI is ready.
static STARTUP_MESSAGES: Global<Vec<StartupMessage>> = Global::default();

/// A message to be displayed in the Godot editor once UI is ready.
struct StartupMessage {
    message: std::ffi::CString,
    function: std::ffi::CString,
    file: std::ffi::CString,
    line: i32,
    level: StartupMessageLevel,
}

#[derive(Clone, Debug)]
pub enum StartupMessageLevel {
    /// Warning with an ID that can be suppressed via `GODOT_RUST_NOWARN`.
    Warn { id: &'static str },
    /// Error that cannot be suppressed.
    Error,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct GdextRuntimeMetadata {
    version_string: String,
    version_triple: (u8, u8, u8),
    supports_deprecated_apis: bool,
}

impl GdextRuntimeMetadata {
    pub fn load(version: GDExtensionGodotVersion, supports_deprecated_apis: bool) -> Self {
        // SAFETY: GDExtensionGodotVersion always contains valid string.
        let version_string = unsafe { read_version_string(version.string) };

        let version_triple = (
            version.major as u8,
            version.minor as u8,
            version.patch as u8,
        );

        Self {
            version_string,
            version_triple,
            supports_deprecated_apis,
        }
    }

    // TODO(v0.5): CowStr, also in GdextBuild.
    pub fn version_string(&self) -> &str {
        &self.version_string
    }

    pub fn version_triple(&self) -> (u8, u8, u8) {
        self.version_triple
    }

    pub fn supports_deprecated_apis(&self) -> bool {
        self.supports_deprecated_apis
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
    out!("Initialize godot-rust...");

    out!(
        "Godot version against which godot-rust was compiled: {}",
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

    let (version, supports_deprecated_apis) = {
        let get_proc_address2 = get_proc_address.expect("get_proc_address unexpectedly null");
        // SAFETY: `ensure_static_runtime_compatibility` succeeded.
        unsafe { interface_init::runtime_version(get_proc_address2) }
    };

    out!("Godot version of GDExtension API at runtime: {:?}", version);

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

    let runtime_metadata = GdextRuntimeMetadata::load(version, supports_deprecated_apis);

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
    deinitialize_binding();

    // MACOS-PARTIAL-RELOAD: Clear the main thread ID to allow re-initialization during hot reload.
    #[cfg(not(wasm_nothreads))]
    {
        if MAIN_THREAD_ID.is_initialized() {
            MAIN_THREAD_ID.clear();
        }
    }
}

fn safeguards_level_string() -> &'static str {
    if cfg!(safeguards_strict) {
        "strict"
    } else if cfg!(safeguards_balanced) {
        "balanced"
    } else {
        "disengaged"
    }
}

/// Internal function to collect a message for deferred display in Godot editor UI. Called by macros.
#[doc(hidden)]
pub fn collect_startup_message(
    mut message: String,
    level: StartupMessageLevel,
    file: &str,
    line: u32,
    module_path: &str,
) {
    // Check if this warning should be suppressed (only warnings can be suppressed, not errors).
    if let StartupMessageLevel::Warn { id } = &level {
        if is_message_suppressed(id) {
            return;
        } else {
            message = format!(
                "{message}\n(Suppress this warning with env-var `GODOT_RUST_NOWARN={id},...`)"
            );
        }
    }

    let msg = StartupMessage {
        message: std::ffi::CString::new(message).expect("message contains null byte"),
        function: std::ffi::CString::new(module_path).expect("module_path contains null byte"),
        file: std::ffi::CString::new(file).expect("file contains null byte"),
        line: line as i32,
        level,
    };

    STARTUP_MESSAGES.lock().push(msg);
}

/// Check if a message ID is suppressed via the `GODOT_RUST_NOWARN` environment variable.
fn is_message_suppressed(id: &str) -> bool {
    if let Ok(nowarn) = std::env::var("GODOT_RUST_NOWARN") {
        nowarn
            .split(',')
            .any(|suppressed_id| suppressed_id.trim() == id)
    } else {
        false
    }
}

/// Flush all deferred messages to the Godot editor. Called during `MainLoop` initialization, when editor UI is ready.
pub fn print_deferred_startup_messages() {
    let mut messages = STARTUP_MESSAGES.lock();

    if messages.is_empty() {
        return;
    }

    for msg in messages.iter() {
        let print_fn = match msg.level {
            StartupMessageLevel::Warn { .. } => interface_fn!(print_warning),
            StartupMessageLevel::Error => interface_fn!(print_error),
        };

        // SAFETY: The binding has been initialized, so we can use interface functions.
        unsafe {
            print_fn(
                msg.message.as_ptr(),
                msg.function.as_ptr(),
                msg.file.as_ptr(),
                msg.line,
                conv::SYS_TRUE, // Notify editor.
            );
        }
    }

    messages.clear();
}

fn print_preamble(version: GDExtensionGodotVersion) {
    // SAFETY: GDExtensionGodotVersion always contains valid string.
    let runtime_version = unsafe { read_version_string(version.string) };

    let api_version: &'static str = GdextBuild::godot_static_version_string();
    let safeguards_level = safeguards_level_string();
    println!("Initialize godot-rust (API {api_version}, runtime {runtime_version}, safeguards {safeguards_level})");
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
            // SAFETY: The interface has been initialized and this function hasn't been called before.
            unsafe {
                #[cfg(feature = "codegen-lazy-fptrs")]
                initialize_class_core_method_table(ClassCoreMethodTable::load());
                #[cfg(not(feature = "codegen-lazy-fptrs"))]
                initialize_class_core_method_table(ClassCoreMethodTable::load(
                    interface,
                    &mut string_names,
                ));
            }
            class_count = ClassCoreMethodTable::CLASS_COUNT;
            method_count = ClassCoreMethodTable::METHOD_COUNT;
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

            // Check if we need to warn about deprecated APIs.
            // SAFETY: The binding has been initialized, so we can access runtime metadata.
            let supports_deprecated_apis = unsafe { runtime_metadata() }.supports_deprecated_apis();
            if !supports_deprecated_apis {
                defer_startup_warn!(
                    id: "GodotWithoutDeprecated",
                    "Your Godot version has disabled deprecated APIs (compiled with `deprecated=no`).\n\
                    This is generally a bad idea, as Godot can no longer run extensions compiled with older\n\
                    versions (e.g. from the asset store). Furthermore, godot-rust does not officially support\n\
                    non-standard builds and can break unexpectedly. This warning may become a hard error.\n\
                    To fix this, use an official stable release, or compile the engine with `deprecated=yes`."
                );
            }
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

            // Note: Deprecated API warning will be emitted at MainLoop init (Godot 4.5+).
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
    let method_bind = unsafe { class_core_api() }.os__has_feature();

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

/// Assign the current thread id to be the main thread.
///
/// This is required for platforms on which Godot runs the main loop on a different thread than the thread the library was loaded on.
/// Android is one such platform.
///
/// # Safety
///
/// - must only be called after [`initialize`] has been called.
pub unsafe fn discover_main_thread() {
    #[cfg(not(wasm_nothreads))]
    {
        if is_main_thread() {
            // we don't have to do anything if the current thread is already the main thread.
            return;
        }

        let thread_id = std::thread::current().id();

        // SAFETY: initialize must have already been called before this function is called. By clearing and setting the cell again we can reinitialize it.
        unsafe {
            MAIN_THREAD_ID.clear();
            MAIN_THREAD_ID.set(thread_id);
        }
    }
}

/// Construct Godot object.
///
/// "NOTIFICATION_POSTINITIALIZE" must be sent after construction since 4.4.
///
/// # Safety
/// `class_name` is assumed to be valid.
pub unsafe fn classdb_construct_object(
    class_name: GDExtensionConstStringNamePtr,
) -> GDExtensionObjectPtr {
    #[cfg(before_api = "4.4")]
    return interface_fn!(classdb_construct_object)(class_name);

    #[cfg(since_api = "4.4")]
    return interface_fn!(classdb_construct_object2)(class_name);
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deferred editor message macros

/// Store a warning for deferred display in Godot editor UI.
///
/// Captured during startup, displayed at `MainLoop` init. Will be visible in Godot editor's _Output_ tab.
/// Warnings can be suppressed via the `GODOT_RUST_NOWARN` environment variable.
///
/// # Example
/// ```no_run
/// use godot_ffi::defer_startup_warn;
/// # fn example() {
/// // Warning with ID (can be suppressed via GODOT_RUST_NOWARN env var).
/// defer_startup_warn!(id: "FeatureDeprecated", "Feature X is deprecated");
/// # }
/// ```
#[macro_export]
macro_rules! defer_startup_warn {
    (id: $id:literal, $fmt:literal $(, $args:expr)* $(,)?) => {{
        let message = format!($fmt $(, $args)*);
        $crate::collect_startup_message(
            message,
            $crate::StartupMessageLevel::Warn { id: $id },
            file!(),
            line!(),
            module_path!(),
        );
    }};
}

/// Store an error for deferred display in Godot editor UI.
///
/// Captured during startup, displayed at `MainLoop` init. Will be visible in Godot editor's _Output_ tab.
/// Errors cannot be suppressed.
///
/// # Example
/// ```no_run
/// use godot_ffi::defer_startup_error;
/// # fn example() {
/// # let reason = "some reason";
/// defer_startup_error!("Failed to initialize: {reason}");
/// # }
/// ```
#[macro_export]
macro_rules! defer_startup_error {
    ($fmt:literal $(, $args:expr)* $(,)?) => {{
        let message = format!($fmt $(, $args)*);
        $crate::collect_startup_message(
            message,
            $crate::StartupMessageLevel::Error,
            file!(),
            line!(),
            module_path!(),
        );
    }};
}
