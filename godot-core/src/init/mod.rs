/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicBool, Ordering};

use godot_ffi as sys;
use sys::GodotFfi;

use crate::builtin::{GString, StringName};
use crate::out;

mod reexport_pub {
    #[cfg(not(wasm_nothreads))]
    pub use super::sys::main_thread_id;
    pub use super::sys::{is_main_thread, GdextBuild, InitStage};
}
pub use reexport_pub::*;

#[repr(C)]
struct InitUserData {
    library: sys::GDExtensionClassLibraryPtr,
    #[cfg(since_api = "4.5")]
    main_loop_callbacks: sys::GDExtensionMainLoopCallbacks,
}

#[cfg(since_api = "4.5")]
unsafe extern "C" fn startup_func<E: ExtensionLibrary>() {
    E::on_stage_init(InitStage::MainLoop);
}

#[cfg(since_api = "4.5")]
unsafe extern "C" fn frame_func<E: ExtensionLibrary>() {
    E::on_main_loop_frame();
}

#[cfg(since_api = "4.5")]
unsafe extern "C" fn shutdown_func<E: ExtensionLibrary>() {
    E::on_stage_deinit(InitStage::MainLoop);
}

#[doc(hidden)]
#[deny(unsafe_op_in_unsafe_fn)]
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
        sys::linux_reload_workaround::default_set_hot_reload();

        let tool_only_in_editor = match E::editor_run_behavior() {
            EditorRunBehavior::ToolClassesOnly => true,
            EditorRunBehavior::AllClasses => false,
        };

        let config = sys::GdextConfig::new(tool_only_in_editor);

        // SAFETY: no custom code has run yet + no other thread is accessing global handle.
        unsafe {
            sys::initialize(get_proc_address, library, config);
        }

        // With experimental-features enabled, we can always print panics to godot_print!
        #[cfg(feature = "experimental-threads")]
        crate::private::set_gdext_hook(|| true);

        // Without experimental-features enabled, we can only print panics with godot_print! if the panic occurs on the main (Godot) thread.
        #[cfg(not(feature = "experimental-threads"))]
        {
            let main_thread = std::thread::current().id();
            crate::private::set_gdext_hook(move || std::thread::current().id() == main_thread);
        }

        // Currently no way to express failure; could be exposed to E if necessary.
        // No early exit, unclear if Godot still requires output parameters to be set.
        let success = true;
        // Leak the userdata. It will be dropped in core level deinitialization.
        let userdata = Box::into_raw(Box::new(InitUserData {
            library,
            #[cfg(since_api = "4.5")]
            main_loop_callbacks: sys::GDExtensionMainLoopCallbacks {
                startup_func: Some(startup_func::<E>),
                frame_func: Some(frame_func::<E>),
                shutdown_func: Some(shutdown_func::<E>),
            },
        }));

        let godot_init_params = sys::GDExtensionInitialization {
            minimum_initialization_level: E::min_level().to_sys(),
            userdata: userdata.cast::<std::ffi::c_void>(),
            initialize: Some(ffi_initialize_layer::<E>),
            deinitialize: Some(ffi_deinitialize_layer::<E>),
        };

        // SAFETY: Godot is responsible for passing us a valid pointer.
        unsafe {
            *init = godot_init_params;
        }

        success as u8
    };

    // Use std::panic::catch_unwind instead of handle_panic: handle_panic uses TLS, which
    // calls `thread_atexit` on linux, which sets the hot reloading flag on linux.
    // Using std::panic::catch_unwind avoids this, although we lose out on context information
    // for debugging.
    let is_success = std::panic::catch_unwind(init_code);

    is_success.unwrap_or(0)
}

static LEVEL_SERVERS_CORE_LOADED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn ffi_initialize_layer<E: ExtensionLibrary>(
    userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let userdata = userdata.cast::<InitUserData>().as_ref().unwrap();
    let level = InitLevel::from_sys(init_level);
    let ctx = || format!("failed to initialize GDExtension level `{level:?}`");

    fn try_load<E: ExtensionLibrary>(level: InitLevel, userdata: &InitUserData) {
        // Workaround for https://github.com/godot-rust/gdext/issues/629:
        // When using editor plugins, Godot may unload all levels but only reload from Scene upward.
        // Manually run initialization of lower levels.

        // TODO: Remove this workaround once after the upstream issue is resolved.
        if level == InitLevel::Scene {
            if !LEVEL_SERVERS_CORE_LOADED.load(Ordering::Relaxed) {
                try_load::<E>(InitLevel::Core, userdata);
                try_load::<E>(InitLevel::Servers, userdata);
            }
        } else if level == InitLevel::Core {
            // When it's normal initialization, the `Servers` level is normally initialized.
            LEVEL_SERVERS_CORE_LOADED.store(true, Ordering::Relaxed);
        }

        // SAFETY: Godot will call this from the main thread, after `__gdext_load_library` where the library is initialized,
        // and only once per level.
        unsafe { gdext_on_level_init(level, userdata) };
        E::on_stage_init(level.to_stage());
    }

    // Swallow panics. TODO consider crashing if gdext init fails.
    let _ = crate::private::handle_panic(ctx, || {
        try_load::<E>(level, userdata);
    });
}

unsafe extern "C" fn ffi_deinitialize_layer<E: ExtensionLibrary>(
    userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let level = InitLevel::from_sys(init_level);
    let ctx = || format!("failed to deinitialize GDExtension level `{level:?}`");

    // Swallow panics.
    let _ = crate::private::handle_panic(ctx, || {
        if level == InitLevel::Core {
            // Once the CORE api is unloaded, reset the flag to initial state.
            LEVEL_SERVERS_CORE_LOADED.store(false, Ordering::Relaxed);

            // Drop the userdata.
            drop(Box::from_raw(userdata.cast::<InitUserData>()));
        }

        E::on_stage_deinit(level.to_stage());
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
unsafe fn gdext_on_level_init(level: InitLevel, userdata: &InitUserData) {
    // TODO: in theory, a user could start a thread in one of the early levels, and run concurrent code that messes with the global state
    // (e.g. class registration). This would break the assumption that the load_class_method_table() calls are exclusive.
    // We could maybe protect globals with a mutex until initialization is complete, and then move it to a directly-accessible, read-only static.

    // SAFETY: we are in the main thread, initialize has been called, has never been called with this level before.
    unsafe { sys::load_class_method_table(level) };

    match level {
        InitLevel::Core => {
            #[cfg(since_api = "4.5")]
            unsafe {
                sys::interface_fn!(register_main_loop_callbacks)(
                    userdata.library,
                    &raw const userdata.main_loop_callbacks,
                )
            };
        }
        InitLevel::Servers => {
            // SAFETY: called from the main thread, sys::initialized has already been called.
            unsafe { sys::discover_main_thread() };
        }
        InitLevel::Scene => {
            // SAFETY: On the main thread, api initialized, `Scene` was initialized above.
            unsafe { ensure_godot_features_compatible() };
        }
        InitLevel::Editor => {
            #[cfg(all(since_api = "4.3", feature = "register-docs"))]
            // SAFETY: Godot binding is initialized, and this is called from the main thread.
            unsafe {
                crate::docs::register();
            }
        }
    }

    crate::registry::class::auto_register_classes(level);
}

/// Tasks needed to be done by gdext internally upon unloading an initialization level. Called after user code.
fn gdext_on_level_deinit(level: InitLevel) {
    crate::registry::class::unregister_classes(level);

    if level == InitLevel::Core {
        // If lowest level is unloaded, call global deinitialization.
        // No business logic by itself, but ensures consistency if re-initialization (hot-reload on Linux) occurs.

        crate::task::cleanup();
        crate::tools::cleanup();

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
/// # Example
/// The simplest usage is as follows. This will automatically perform the necessary init and cleanup routines, and register
/// all classes marked with `#[derive(GodotClass)]`, without needing to mention them in a central list. The order in which
/// classes are registered is not specified.
///
/// ```
/// use godot::init::*;
///
/// // This is just a type tag without any functionality.
/// // Its name is irrelevant.
/// struct MyExtension;
///
/// #[gdextension]
/// unsafe impl ExtensionLibrary for MyExtension {}
/// ```
///
/// # Custom entry symbol
/// There is usually no reason to, but you can use a different entry point (C function in the dynamic library). This must match the key
/// that you specify in the `.gdextension` file. Let's say your `.gdextension` file has such a section:
/// ```toml
/// [configuration]
/// entry_symbol = "custom_name"
/// ```
/// then you can implement the trait like this:
/// ```no_run
/// # use godot::init::*;
/// struct MyExtension;
///
/// #[gdextension(entry_symbol = custom_name)]
/// unsafe impl ExtensionLibrary for MyExtension {}
/// ```
/// Note that this only changes the name. You cannot provide your own function -- use the [`on_level_init()`][ExtensionLibrary::on_level_init]
/// hook for custom startup logic.
///
/// # Availability of Godot APIs during init and deinit
// Init order: see also special_cases.rs > classify_codegen_level().
/// Godot loads functionality gradually during its startup routines, and unloads it during shutdown. As a result, Godot classes are only
/// available above a certain level. Trying to access a class API when it's not available will panic (if not, please report it as a bug).
///
/// A few singletons (`Engine`, `Os`, `Time`, `ProjectSettings`) are available from the `Core` level onward and can be used inside
/// this method. Most other singletons are **not available during init** at all, and will only become accessible once the first frame has
/// run.
///
/// The exact time a class is available depends on the Godot initialization logic, which is quite complex and may change between versions.
/// To get an up-to-date view, inspect the Godot source code of [main.cpp], particularly `Main::setup()`, `Main::setup2()` and
/// `Main::cleanup()` methods. Make sure to look at the correct version of the file.
///
/// In case of doubt, do not rely on classes being available during init/deinit.
///
/// [main.cpp]: https://github.com/godotengine/godot/blob/master/main/main.cpp
///
/// # Safety
/// The library cannot enforce any safety guarantees outside Rust code, which means that **you as a user** are
/// responsible to uphold them: namely in GDScript code or other GDExtension bindings loaded by the engine.
/// Violating this may cause undefined behavior, even when invoking _safe_ functions.
///
/// If you use the `disengaged` [safeguard level], you accept that UB becomes possible even **in safe Rust APIs**, if you use them wrong
/// (e.g. accessing a destroyed object).
///
/// [gdextension]: attr.gdextension.html
/// [safety]: https://godot-rust.github.io/book/gdext/advanced/safety.html
/// [safeguard level]: ../index.html#safeguard-levels
// FIXME intra-doc link
#[doc(alias = "entry_symbol", alias = "entry_point")]
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

    /// Custom logic when a certain initialization stage is loaded.
    ///
    /// This will be invoked for stages >= [`Self::min_level()`], in ascending order. Use `if` or `match` to hook to specific stages.
    ///
    /// The stages are loaded in order: `Core` → `Servers` → `Scene` → `Editor` (if in editor) → `MainLoop` (4.5+).  \
    /// The `MainLoop` stage represents the fully initialized state of Godot, after all initialization levels and classes have been loaded.
    ///
    /// See also [`on_main_loop_frame()`][Self::on_main_loop_frame] for per-frame processing.
    ///
    /// # Panics
    /// If the overridden method panics, an error will be printed, but GDExtension loading is **not** aborted.
    #[allow(unused_variables)]
    #[expect(deprecated)] // Fall back to older API.
    fn on_stage_init(stage: InitStage) {
        stage
            .try_to_level()
            .inspect(|&level| Self::on_level_init(level));

        #[cfg(since_api = "4.5")] // Compat layer.
        if stage == InitStage::MainLoop {
            Self::on_main_loop_startup();
        }
    }

    /// Custom logic when a certain initialization stage is unloaded.
    ///
    /// This will be invoked for stages >= [`Self::min_level()`], in descending order. Use `if` or `match` to hook to specific stages.
    ///
    /// The stages are unloaded in reverse order: `MainLoop` (4.5+) → `Editor` (if in editor) → `Scene` → `Servers` → `Core`.  \
    /// At the time `MainLoop` is deinitialized, all classes are still available.
    ///
    /// # Panics
    /// If the overridden method panics, an error will be printed, but GDExtension unloading is **not** aborted.
    #[allow(unused_variables)]
    #[expect(deprecated)] // Fall back to older API.
    fn on_stage_deinit(stage: InitStage) {
        #[cfg(since_api = "4.5")] // Compat layer.
        if stage == InitStage::MainLoop {
            Self::on_main_loop_shutdown();
        }

        stage
            .try_to_level()
            .inspect(|&level| Self::on_level_deinit(level));
    }

    /// Old callback before [`on_stage_init()`][Self::on_stage_deinit] was added. Does not support `MainLoop` stage.
    #[deprecated = "Use `on_stage_init()` instead, which also includes the MainLoop stage."]
    #[allow(unused_variables)]
    fn on_level_init(level: InitLevel) {
        // Nothing by default.
    }

    /// Old callback before [`on_stage_deinit()`][Self::on_stage_deinit] was added. Does not support `MainLoop` stage.
    #[deprecated = "Use `on_stage_deinit()` instead, which also includes the MainLoop stage."]
    #[allow(unused_variables)]
    fn on_level_deinit(level: InitLevel) {
        // Nothing by default.
    }

    #[cfg(since_api = "4.5")]
    #[deprecated = "Use `on_stage_init(InitStage::MainLoop)` instead."]
    #[doc(hidden)] // Added by mistake -- works but don't advertise.
    fn on_main_loop_startup() {
        // Nothing by default.
    }

    #[cfg(since_api = "4.5")]
    #[deprecated = "Use `on_stage_deinit(InitStage::MainLoop)` instead."]
    #[doc(hidden)] // Added by mistake -- works but don't advertise.
    fn on_main_loop_shutdown() {
        // Nothing by default.
    }

    /// Callback invoked for every process frame.
    ///
    /// This is called during the main loop, after Godot is fully initialized. It runs after all
    /// [`process()`][crate::classes::INode::process] methods on Node, and before the Godot-internal `ScriptServer::frame()`.
    /// This is intended to be the equivalent of [`IScriptLanguageExtension::frame()`][`crate::classes::IScriptLanguageExtension::frame()`]
    /// for GDExtension language bindings that don't use the script API.
    ///
    /// # Example
    /// To hook into startup/shutdown of the main loop, use [`on_stage_init()`][Self::on_stage_init] and
    /// [`on_stage_deinit()`][Self::on_stage_deinit] and watch for [`InitStage::MainLoop`].
    ///
    /// ```no_run
    /// # use godot::init::*;
    /// # struct MyExtension;
    /// #[gdextension]
    /// unsafe impl ExtensionLibrary for MyExtension {
    ///     fn on_stage_init(stage: InitStage) {
    ///         if stage == InitStage::MainLoop {
    ///             // Startup code after fully initialized.
    ///         }
    ///     }
    ///
    ///     fn on_main_loop_frame() {
    ///         // Per-frame logic.
    ///     }
    ///
    ///     fn on_stage_deinit(stage: InitStage) {
    ///         if stage == InitStage::MainLoop {
    ///             // Cleanup code before shutdown.
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    /// If the overridden method panics, an error will be printed, but execution continues.
    #[cfg(since_api = "4.5")]
    fn on_main_loop_frame() {
        // Nothing by default.
    }

    /// Whether to override the Wasm binary filename used by your GDExtension which the library should expect at runtime. Return `None`
    /// to use the default where gdext expects either `{YourCrate}.wasm` (default binary name emitted by Rust) or
    /// `{YourCrate}.threads.wasm` (for builds producing separate single-threaded and multi-threaded binaries).
    ///
    /// Upon exporting a game to the web, the library has to know at runtime the exact name of the `.wasm` binary file being used to load
    /// each GDExtension. By default, Rust exports the binary as `cratename.wasm`, so that is the name checked by godot-rust by default.
    ///
    /// However, if you need to rename that binary, you can make the library aware of the new binary name by returning
    /// `Some("newname.wasm")` (don't forget to **include the `.wasm` extension**).
    ///
    /// For example, to have two simultaneous versions, one supporting multi-threading and the other not, you could add a suffix to the
    /// filename of the Wasm binary of the multi-threaded version in your build process. If you choose the suffix `.threads.wasm`,
    /// you're in luck as godot-rust already accepts this suffix by default, but let's say you want to use a different suffix, such as
    /// `-with-threads.wasm`. For this, you can have a `"nothreads"` feature which, when absent, should produce a suffixed binary,
    /// which can be informed to gdext as follows:
    ///
    /// ```no_run
    /// # use godot::init::*;
    /// struct MyExtension;
    ///
    /// #[gdextension]
    /// unsafe impl ExtensionLibrary for MyExtension {
    ///     fn override_wasm_binary() -> Option<&'static str> {
    ///         // Binary name unchanged ("mycrate.wasm") without thread support.
    ///         #[cfg(feature = "nothreads")]
    ///         return None;
    ///
    ///         // Tell godot-rust we add a custom suffix to the binary with thread support.
    ///         // Please note that this is not needed if "mycrate.threads.wasm" is used.
    ///         // (You could return `None` as well in that particular case.)
    ///         #[cfg(not(feature = "nothreads"))]
    ///         Some("mycrate-with-threads.wasm")
    ///     }
    /// }
    /// ```
    /// Note that simply overriding this method won't change the name of the Wasm binary produced by Rust automatically: you'll still
    /// have to rename it by yourself in your build process, as well as specify the updated binary name in your `.gdextension` file.
    /// This is just to ensure gdext is aware of the new name given to the binary, avoiding runtime errors.
    fn override_wasm_binary() -> Option<&'static str> {
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

pub use sys::InitLevel;

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

    #[cfg(feature = "debug-log")] // Display safeguards level in debug log.
    let safeguards_level = if cfg!(safeguards_strict) {
        "strict"
    } else if cfg!(safeguards_balanced) {
        "balanced"
    } else {
        "disengaged"
    };
    out!("Safeguards: {safeguards_level}");

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
