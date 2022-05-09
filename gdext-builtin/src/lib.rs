#![macro_use]

pub mod macros;

mod color;
mod others;
mod string;
mod variant;
mod vector2;
mod vector3;

pub use color::*;
pub use others::*;
pub use string::*;
pub use variant::*;
pub use vector2::*;
pub use vector3::*;

pub use glam;

use gdext_sys as sys;
use std::collections::BTreeMap;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum InitLevel {
    Core,
    Servers,
    Scene,
    Editor,
}

impl InitLevel {
    #[doc(hidden)]
    pub fn from_sys(level: gdext_sys::GDNativeInitializationLevel) -> Self {
        match level {
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_CORE => Self::Core,
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SERVERS => Self::Servers,
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE => Self::Scene,
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_EDITOR => Self::Editor,
            _ => Self::Scene,
        }
    }
    #[doc(hidden)]
    pub fn to_sys(self) -> gdext_sys::GDNativeInitializationLevel {
        match self {
            Self::Core => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_CORE,
            Self::Servers => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SERVERS,
            Self::Scene => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE,
            Self::Editor => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_EDITOR,
        }
    }
}

pub struct InitOptions {
    init_levels: BTreeMap<InitLevel, Box<dyn FnOnce() + 'static>>,
    deinit_levels: BTreeMap<InitLevel, Box<dyn FnOnce() + 'static>>,
    lowest_level: InitLevel,
}

impl InitOptions {
    pub fn new() -> Self {
        Self {
            init_levels: Default::default(),
            deinit_levels: Default::default(),
            lowest_level: InitLevel::Scene,
        }
    }

    pub fn register_init_function(&mut self, level: InitLevel, f: impl FnOnce() + 'static) {
        self.init_levels.insert(level, Box::new(f));
        self.lowest_level = self.lowest_level.min(level);
    }

    pub fn register_deinit_function(&mut self, level: InitLevel, f: impl FnOnce() + 'static) {
        self.deinit_levels.insert(level, Box::new(f));
    }

    pub fn lowest_init_level(&self) -> InitLevel {
        self.lowest_level
    }

    pub fn run_init_function(&mut self, level: InitLevel) {
        if let Some(f) = self.init_levels.remove(&level) {
            f();
        }
    }

    pub fn run_deinit_function(&mut self, level: InitLevel) {
        if let Some(f) = self.deinit_levels.remove(&level) {
            f();
        }
    }
}

impl Default for InitOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub static mut INIT_OPTIONS: Option<InitOptions> = None;

#[macro_export]
macro_rules! gdext_init {
    ($name:ident, $f:expr) => {
        #[no_mangle]
        unsafe extern "C" fn gdext_rust_test(
            interface: *const ::gdext_sys::GDNativeInterface,
            library: ::gdext_sys::GDNativeExtensionClassLibraryPtr,
            init: *mut ::gdext_sys::GDNativeInitialization,
        ) {
            ::gdext_sys::set_interface(interface);
            ::gdext_sys::set_library(library);

            let mut init_options = $crate::InitOptions::new();

            ($f)(&mut init_options);

            *init = ::gdext_sys::GDNativeInitialization {
                minimum_initialization_level: init_options.lowest_init_level().to_sys(),
                userdata: std::ptr::null_mut(),
                initialize: Some(initialise),
                deinitialize: Some(deinitialise),
            };

            $crate::INIT_OPTIONS = Some(init_options);
        }

        unsafe extern "C" fn initialise(
            _userdata: *mut std::ffi::c_void,
            init_level: ::gdext_sys::GDNativeInitializationLevel,
        ) {
            let init_options = $crate::INIT_OPTIONS.as_mut().unwrap();
            init_options.run_init_function($crate::InitLevel::from_sys(init_level));
        }

        unsafe extern "C" fn deinitialise(
            _userdata: *mut std::ffi::c_void,
            init_level: ::gdext_sys::GDNativeInitializationLevel,
        ) {
            let init_options = $crate::INIT_OPTIONS.as_mut().unwrap();
            init_options.run_deinit_function($crate::InitLevel::from_sys(init_level));
        }
    };
}
