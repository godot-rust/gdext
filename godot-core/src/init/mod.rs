/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use godot_ffi as sys;

use sys::GodotFfi;

use crate::builtin::{GString, StringName};
use crate::out;

pub use sys::GdextBuild;

#[doc(hidden)]
// TODO consider body safe despite unsafe function, and explicitly mark unsafe {} locations
pub unsafe fn __gdext_load_library<E: ExtensionLibrary>(
    get_proc_address: sys::GDExtensionInterfaceGetProcAddress,
    library: sys::GDExtensionClassLibraryPtr,
    init: *mut sys::GDExtensionInitialization,
) -> sys::GDExtensionBool {
    let init_code = || {
        // Make sure the first thing we do is check whether hot reloading should be enabled or not. This is to ensure that if we do anything to
        // cause TLS-destructors to run then we have a setting already for how to deal with them. Otherwise, this could cause the default
        // behavior to kick in and disable hot reloading.
        #[cfg(target_os = "linux")]
        match E::override_hot_reload() {
            None => sys::linux_reload_workaround::default_set_hot_reload(),
            Some(true) => sys::linux_reload_workaround::enable_hot_reload(),
            Some(false) => sys::linux_reload_workaround::disable_hot_reload(),
        }

        let tool_only_in_editor = match E::editor_run_behavior() {
            EditorRunBehavior::ToolClassesOnly => true,
            EditorRunBehavior::AllClasses => false,
        };

        let config = sys::GdextConfig::new(tool_only_in_editor);

        sys::initialize(get_proc_address, library, config);

        // Currently no way to express failure; could be exposed to E if necessary.
        // No early exit, unclear if Godot still requires output parameters to be set.
        let success = true;

        let godot_init_params = sys::GDExtensionInitialization {
            minimum_initialization_level: E::min_level().to_sys(),
            userdata: std::ptr::null_mut(),
            initialize: Some(ffi_initialize_layer::<E>),
            deinitialize: Some(ffi_deinitialize_layer::<E>),
        };

        *init = godot_init_params;

        success as u8
    };

    let ctx = || "error when loading GDExtension library";
    let is_success = crate::private::handle_panic(ctx, init_code);

    is_success.unwrap_or(0)
}

static LEVEL_SERVERS_CORE_LOADED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn ffi_initialize_layer<E: ExtensionLibrary>(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let level = InitLevel::from_sys(init_level);
    let ctx = || format!("failed to initialize GDExtension level `{:?}`", level);

    fn try_load<E: ExtensionLibrary>(level: InitLevel) {
        // Workaround for https://github.com/godot-rust/gdext/issues/629:
        // When using editor plugins, Godot may unload all levels but only reload from Scene upward.
        // Manually run initialization of lower levels.

        // TODO: Remove this workaround once after the upstream issue is resolved.
        if level == InitLevel::Scene {
            if !LEVEL_SERVERS_CORE_LOADED.load(Relaxed) {
                try_load::<E>(InitLevel::Core);
                try_load::<E>(InitLevel::Servers);
            }
        } else if level == InitLevel::Core {
            // When it's normal initialization, the `Servers` level is normally initialized.
            LEVEL_SERVERS_CORE_LOADED.store(true, Relaxed);
        }

        // SAFETY: Godot will call this from the main thread, after `__gdext_load_library` where the library is initialized,
        // and only once per level.
        unsafe { gdext_on_level_init(level) };
        E::on_level_init(level);
    }

    // Swallow panics. TODO consider crashing if gdext init fails.
    let _ = crate::private::handle_panic(ctx, || {
        try_load::<E>(level);
    });
}

unsafe extern "C" fn ffi_deinitialize_layer<E: ExtensionLibrary>(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let level = InitLevel::from_sys(init_level);
    let ctx = || format!("failed to deinitialize GDExtension level `{:?}`", level);

    // Swallow panics.
    let _ = crate::private::handle_panic(ctx, || {
        if level == InitLevel::Core {
            // Once the CORE api is unloaded, reset the flag to initial state.
            LEVEL_SERVERS_CORE_LOADED.store(false, Relaxed);
        }

        E::on_level_deinit(level);
        gdext_on_level_deinit(level);
    });
}

/// Tasks needed to be done by gdext internally upon loading an initialization level. Called before user code.
///
/// # Safety
///
/// - Must be called from the main thread.
/// - The interface must have been initialized.
/// - Must only be called once per level.
#[deny(unsafe_op_in_unsafe_fn)]
unsafe fn gdext_on_level_init(level: InitLevel) {
    // TODO: in theory, a user could start a thread in one of the early levels, and run concurrent code that messes with the global state
    // (e.g. class registration). This would break the assumption that the load_class_method_table() calls are exclusive.
    // We could maybe protect globals with a mutex until initialization is complete, and then move it to a directly-accessible, read-only static.

    // SAFETY: we are in the main thread, initialize has been called, has never been called with this level before.
    unsafe { sys::load_class_method_table(level) };

    match level {
        InitLevel::Scene => {
            // SAFETY: On the main thread, api initialized, `Scene` was initialized above.
            unsafe { ensure_godot_features_compatible() };
        }
        InitLevel::Editor => {
            #[cfg(all(since_api = "4.3", feature = "docs"))]
            // SAFETY: Godot binding is initialized, and this is called from the main thread.
            unsafe {
                crate::docs::register();
            }
        }
        _ => (),
    }

    crate::registry::class::auto_register_classes(level);
}

/// Tasks needed to be done by gdext internally upon unloading an initialization level. Called after user code.
fn gdext_on_level_deinit(level: InitLevel) {
    crate::registry::class::unregister_classes(level);

    if level == InitLevel::Core {
        // If lowest level is unloaded, call global deinitialization.
        // No business logic by itself, but ensures consistency if re-initialization (hot-reload on Linux) occurs.

        // Garbage-collect various statics.
        // SAFETY: this is the last time meta APIs are used.
        unsafe {
            crate::meta::cleanup();
        }

        // SAFETY: called after all other logic, so no concurrent access.
        // TODO: multithreading must make sure other threads are joined/stopped here.
        unsafe {
            sys::deinitialize();
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Defines the entry point for a GDExtension Rust library.
///
/// Every library should have exactly one implementation of this trait. It is always used in combination with the
/// [`#[gdextension]`][gdextension] proc-macro attribute.
///
/// The simplest usage is as follows. This will automatically perform the necessary init and cleanup routines, and register
/// all classes marked with `#[derive(GodotClass)]`, without needing to mention them in a central list. The order in which
/// classes are registered is not specified.
///
/// ```
/// # use godot::init::*;
/// // This is just a type tag without any functionality.
/// // Its name is irrelevant.
/// struct MyExtension;
///
/// #[gdextension]
/// unsafe impl ExtensionLibrary for MyExtension {}
/// ```
///
/// # Safety
/// By using godot-rust, you accept the safety considerations [as outlined in the book][safety].
/// Please make sure you fully understand the implications.
///
/// The library cannot enforce any safety guarantees outside Rust code, which means that **you as a user** are
/// responsible to uphold them: namely in GDScript code or other GDExtension bindings loaded by the engine.
/// Violating this may cause undefined behavior, even when invoking _safe_ functions.
///
/// [gdextension]: attr.gdextension.html
/// [safety]: https://godot-rust.github.io/book/gdext/advanced/safety.html
// FIXME intra-doc link
pub unsafe trait ExtensionLibrary {
    /// Determines if and how an extension's code is run in the editor.
    fn editor_run_behavior() -> EditorRunBehavior {
        EditorRunBehavior::ToolClassesOnly
    }

    /// Determines the initialization level at which the extension is loaded (`Scene` by default).
    ///
    /// If the level is lower than [`InitLevel::Scene`], the engine needs to be restarted to take effect.
    fn min_level() -> InitLevel {
        InitLevel::Scene
    }

    /// Custom logic when a certain init-level of Godot is loaded.
    ///
    /// This will only be invoked for levels >= [`Self::min_level()`], in ascending order. Use `if` or `match` to hook to specific levels.
    #[allow(unused_variables)]
    fn on_level_init(level: InitLevel) {
        // Nothing by default.
    }

    /// Custom logic when a certain init-level of Godot is unloaded.
    ///
    /// This will only be invoked for levels >= [`Self::min_level()`], in descending order. Use `if` or `match` to hook to specific levels.
    #[allow(unused_variables)]
    fn on_level_deinit(level: InitLevel) {
        // Nothing by default.
    }

    /// Whether to enable hot reloading of this library. Return `None` to use the default behavior.
    ///
    /// Enabling this will ensure that the library can be hot reloaded. If this is disabled then hot reloading may still work, but there is no
    /// guarantee. Enabling this may also lead to memory leaks, so it should not be enabled for builds that are intended to be final builds.
    ///
    /// By default, this is enabled for debug builds and disabled for release builds.
    ///
    /// Note that this is only checked *once* upon initializing the library. Changing this from `true` to `false` will be picked up as the
    /// library is then fully reloaded upon hot-reloading, however changing it from `false` to `true` is almost certainly not going to work
    /// unless hot-reloading is already working regardless of this setting.
    fn override_hot_reload() -> Option<bool> {
        None
    }
}

/// Determines if and how an extension's code is run in the editor.
///
/// By default, Godot 4 runs all virtual lifecycle callbacks (`_ready`, `_process`, `_physics_process`, ...)
/// for extensions. This behavior is different from Godot 3, where extension classes needed to be explicitly
/// marked as "tool".
///
/// In many cases, users write extension code with the intention to run in games, not inside the editor.
/// This is why the default behavior in gdext deviates from Godot: lifecycle callbacks are disabled inside the
/// editor (see [`ToolClassesOnly`][Self::ToolClassesOnly]). It is possible to configure this.
///
/// See also [`ExtensionLibrary::editor_run_behavior()`].
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum EditorRunBehavior {
    /// Only runs `#[class(tool)]` classes in the editor.
    ///
    /// All classes are registered, and calls from GDScript to Rust are possible. However, virtual lifecycle callbacks
    /// (`_ready`, `_process`, `_physics_process`, ...) are not run unless the class is annotated with `#[class(tool)]`.
    ToolClassesOnly,

    /// Runs the extension with full functionality in editor.
    ///
    /// Ignores any `#[class(tool)]` annotations.
    AllClasses,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Stage of the Godot initialization process.
///
/// Godot's initialization and deinitialization processes are split into multiple stages, like a stack. At each level,
/// a different amount of engine functionality is available. Deinitialization happens in reverse order.
///
/// See also:
/// - [`ExtensionLibrary::on_level_init()`]
/// - [`ExtensionLibrary::on_level_deinit()`]
pub type InitLevel = sys::InitLevel;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// # Safety
///
/// - Must be called from the main thread.
/// - The interface must be initialized.
/// - The `Scene` api level must have been initialized.
#[deny(unsafe_op_in_unsafe_fn)]
unsafe fn ensure_godot_features_compatible() {
    // The reason why we don't simply call Os::has_feature() here is that we might move the high-level engine classes out of godot-core
    // later, and godot-core would only depend on godot-sys. This makes future migrations easier. We still have access to builtins though.

    out!("Check Godot precision setting...");

    let os_class = StringName::from("OS");
    let single = GString::from("single");
    let double = GString::from("double");

    let gdext_is_double = cfg!(feature = "double-precision");

    // SAFETY: main thread, after initialize(), valid string pointers, `Scene` initialized.
    let godot_is_double = unsafe {
        let is_single = sys::godot_has_feature(os_class.string_sys(), single.sys());
        let is_double = sys::godot_has_feature(os_class.string_sys(), double.sys());

        assert_ne!(
            is_single, is_double,
            "Godot has invalid configuration: single={is_single}, double={is_double}"
        );

        is_double
    };

    let s = |is_double: bool| -> &'static str {
        if is_double {
            "double"
        } else {
            "single"
        }
    };

    out!(
        "Is double precision: Godot={}, gdext={}",
        s(godot_is_double),
        s(gdext_is_double)
    );

    if godot_is_double != gdext_is_double {
        panic!(
            "Godot runs with {} precision, but gdext was compiled with {} precision.\n\
            Cargo feature `double-precision` must be used if and only if Godot is compiled with `precision=double`.\n",
            s(godot_is_double), s(gdext_is_double),
        );
    }
}
