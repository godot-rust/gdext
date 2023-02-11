/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use std::collections::btree_map::BTreeMap;

#[doc(hidden)]
// TODO consider body safe despite unsafe function, and explicitly mark unsafe {} locations
pub unsafe fn __gdext_load_library<E: ExtensionLibrary>(
    interface: *const sys::GDExtensionInterface,
    library: sys::GDExtensionClassLibraryPtr,
    init: *mut sys::GDExtensionInitialization,
) -> sys::GDExtensionBool {
    let init_code = || {
        sys::initialize(interface, library);

        let mut handle = InitHandle::new();

        let success = E::load_library(&mut handle);
        // No early exit, unclear if Godot still requires output parameters to be set

        let godot_init_params = sys::GDExtensionInitialization {
            minimum_initialization_level: handle.lowest_init_level().to_sys(),
            userdata: std::ptr::null_mut(),
            initialize: Some(ffi_initialize_layer),
            deinitialize: Some(ffi_deinitialize_layer),
        };

        *init = godot_init_params;
        INIT_HANDLE = Some(handle);

        success as u8
    };

    let ctx = || "error when loading GDExtension library";
    let is_success = crate::private::handle_panic(ctx, init_code);

    is_success.unwrap_or(0)
}

#[doc(hidden)]
pub fn __gdext_default_init(handle: &mut InitHandle) {
    handle.register_layer(InitLevel::Scene, DefaultLayer);
}

unsafe extern "C" fn ffi_initialize_layer(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let ctx = || {
        format!(
            "failed to initialize GDExtension layer `{:?}`",
            InitLevel::from_sys(init_level)
        )
    };

    crate::private::handle_panic(ctx, || {
        let handle = INIT_HANDLE.as_mut().unwrap();
        handle.run_init_function(InitLevel::from_sys(init_level));
    });
}

unsafe extern "C" fn ffi_deinitialize_layer(
    _userdata: *mut std::ffi::c_void,
    init_level: sys::GDExtensionInitializationLevel,
) {
    let ctx = || {
        format!(
            "failed to deinitialize GDExtension layer `{:?}`",
            InitLevel::from_sys(init_level)
        )
    };

    crate::private::handle_panic(ctx, || {
        let handle = INIT_HANDLE.as_mut().unwrap();
        handle.run_deinit_function(InitLevel::from_sys(init_level));
    });
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// FIXME make safe
#[doc(hidden)]
pub static mut INIT_HANDLE: Option<InitHandle> = None;

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
///
/// // This is just a type tag without any functionality
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
/// [gdextension]: crate::init::gdextension
/// [safety]: https://godot-rust.github.io/book/gdextension/safety.html
// FIXME intra-doc link
pub unsafe trait ExtensionLibrary {
    fn load_library(handle: &mut InitHandle) -> bool {
        handle.register_layer(InitLevel::Scene, DefaultLayer);
        true
    }
}

pub trait ExtensionLayer: 'static {
    fn initialize(&mut self);
    fn deinitialize(&mut self);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct DefaultLayer;

impl ExtensionLayer for DefaultLayer {
    fn initialize(&mut self) {
        crate::auto_register_classes();
    }

    fn deinitialize(&mut self) {
        // Nothing -- note that any cleanup task should be performed outside of this method,
        // as the user is free to use a different impl, so cleanup code may not be run.
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct InitHandle {
    layers: BTreeMap<InitLevel, Box<dyn ExtensionLayer>>,
    // success: bool,
}

impl InitHandle {
    pub fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
            // success: true,
        }
    }

    pub fn register_layer(&mut self, level: InitLevel, layer: impl ExtensionLayer) {
        self.layers.insert(level, Box::new(layer));
    }

    // pub fn mark_failed(&mut self) {
    //     self.success = false;
    // }

    pub fn lowest_init_level(&self) -> InitLevel {
        self.layers
            .iter()
            .next()
            .map(|(k, _v)| *k)
            .unwrap_or(InitLevel::Scene)
    }

    pub fn run_init_function(&mut self, level: InitLevel) {
        // if let Some(f) = self.init_levels.remove(&level) {
        //     f();
        // }
        if let Some(layer) = self.layers.get_mut(&level) {
            layer.initialize()
        }
    }

    pub fn run_deinit_function(&mut self, level: InitLevel) {
        if let Some(layer) = self.layers.get_mut(&level) {
            layer.deinitialize()
        }
    }
}

impl Default for InitHandle {
    fn default() -> Self {
        Self::new()
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum InitLevel {
    Core,
    Servers,
    Scene,
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
