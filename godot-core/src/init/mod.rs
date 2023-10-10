/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use std::cell;

#[doc(hidden)]
// TODO consider body safe despite unsafe function, and explicitly mark unsafe {} locations
pub unsafe fn __gdext_load_library<E: ExtensionLibrary>(
    interface_or_get_proc_address: sys::InitCompat,
    library: sys::GDExtensionClassLibraryPtr,
    init: *mut sys::GDExtensionInitialization,
) -> sys::GDExtensionBool {
    let init_code = || {
        let tool_only_in_editor = match E::editor_run_behavior() {
            EditorRunBehavior::ToolClassesOnly => true,
            EditorRunBehavior::AllClasses => false,
        };

        let config = sys::GdextConfig {
            tool_only_in_editor,
            is_editor: cell::OnceCell::new(),
        };

        sys::initialize(interface_or_get_proc_address, library, config);

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

unsafe extern "C" fn ffi_initialize_layer<E: ExtensionLibrary>(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let level = InitLevel::from_sys(init_level);
    let ctx = || format!("failed to initialize GDExtension level `{:?}`", level);

    // Swallow panics. TODO consider crashing if gdext init fails.
    let _ = crate::private::handle_panic(ctx, || {
        gdext_on_level_init(level);
        E::on_level_init(level);
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
        E::on_level_deinit(level);
        gdext_on_level_deinit(level);
    });
}

/// Tasks needed to be done by gdext internally upon loading an initialization level. Called before user code.
fn gdext_on_level_init(level: InitLevel) {
    // SAFETY: we are in the main thread, during initialization, no other logic is happening.
    // TODO: in theory, a user could start a thread in one of the early levels, and run concurrent code that messes with the global state
    // (e.g. class registration). This would break the assumption that the load_class_method_table() calls are exclusive.
    // We could maybe protect globals with a mutex until initialization is complete, and then move it to a directly-accessible, read-only static.
    unsafe {
        match level {
            InitLevel::Core => {}
            InitLevel::Servers => {
                sys::load_class_method_table(sys::ClassApiLevel::Server);
            }
            InitLevel::Scene => {
                sys::load_class_method_table(sys::ClassApiLevel::Scene);
            }
            InitLevel::Editor => {
                sys::load_class_method_table(sys::ClassApiLevel::Editor);
            }
        }
        crate::auto_register_classes(level);
    }
}

/// Tasks needed to be done by gdext internally upon unloading an initialization level. Called after user code.
fn gdext_on_level_deinit(level: InitLevel) {
    crate::unregister_classes(level);
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
    fn on_level_init(_level: InitLevel) {
        // Nothing by default.
    }

    /// Custom logic when a certain init-level of Godot is unloaded.
    ///
    /// This will only be invoked for levels >= [`Self::min_level()`], in descending order. Use `if` or `match` to hook to specific levels.
    fn on_level_deinit(_level: InitLevel) {
        // Nothing by default.
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
    pub fn from_sys(level: godot_ffi::GDExtensionInitializationLevel) -> Self {
        match level {
            sys::GDEXTENSION_INITIALIZATION_CORE => Self::Core,
            sys::GDEXTENSION_INITIALIZATION_SERVERS => Self::Servers,
            sys::GDEXTENSION_INITIALIZATION_SCENE => Self::Scene,
            sys::GDEXTENSION_INITIALIZATION_EDITOR => Self::Editor,
            _ => {
                eprintln!("WARNING: unknown initialization level {level}");
                Self::Scene
            }
        }
    }
    #[doc(hidden)]
    pub fn to_sys(self) -> godot_ffi::GDExtensionInitializationLevel {
        match self {
            Self::Core => sys::GDEXTENSION_INITIALIZATION_CORE,
            Self::Servers => sys::GDEXTENSION_INITIALIZATION_SERVERS,
            Self::Scene => sys::GDEXTENSION_INITIALIZATION_SCENE,
            Self::Editor => sys::GDEXTENSION_INITIALIZATION_EDITOR,
        }
    }
}
