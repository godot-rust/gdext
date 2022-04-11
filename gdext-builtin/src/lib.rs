#![macro_use]

pub mod color;
pub mod godot_ffi;
pub mod macros;
pub mod string;
pub mod variant;
pub mod vector2;
pub mod vector3;

use std::collections::BTreeMap;

use crate::godot_ffi::GodotFfi;
pub use glam;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum InitLevel {
    Core,
    Servers,
    Scene,
    Editor,
    Driver,
}

impl InitLevel {
    #[doc(hidden)]
    pub fn from_sys(level: gdext_sys::GDNativeInitializationLevel) -> Self {
        match level {
            gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_CORE => Self::Core,
            gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SERVERS => Self::Servers,
            gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE => Self::Scene,
            gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_EDITOR => Self::Editor,
            gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_DRIVER => Self::Driver,
            _ => Self::Scene,
        }
    }
    #[doc(hidden)]
    pub fn to_sys(self) -> gdext_sys::GDNativeInitializationLevel {
        match self {
            Self::Core => gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_CORE,
            Self::Servers => gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SERVERS,
            Self::Scene => gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE,
            Self::Editor => gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_EDITOR,
            Self::Driver => gdext_sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_DRIVER,
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
            interface: *const gdext_sys::GDNativeInterface,
            library: gdext_sys::GDNativeExtensionClassLibraryPtr,
            init: *mut gdext_sys::GDNativeInitialization,
        ) {
            gdext_sys::set_interface(interface);
            gdext_sys::set_library(library);

            let mut init_options = $crate::InitOptions::new();

            ($f)(&mut init_options);

            *init = sys::GDNativeInitialization {
                minimum_initialization_level: init_options.lowest_init_level().to_sys(),
                userdata: std::ptr::null_mut(),
                initialize: Some(initialise),
                deinitialize: Some(deinitialise),
            };

            $crate::INIT_OPTIONS = Some(init_options);
        }

        unsafe extern "C" fn initialise(
            _userdata: *mut std::ffi::c_void,
            init_level: gdext_sys::GDNativeInitializationLevel,
        ) {
            let init_options = $crate::INIT_OPTIONS.as_mut().unwrap();
            init_options.run_init_function(InitLevel::from_sys(init_level));
        }

        unsafe extern "C" fn deinitialise(
            _userdata: *mut std::ffi::c_void,
            init_level: gdext_sys::GDNativeInitializationLevel,
        ) {
            let init_options = $crate::INIT_OPTIONS.as_mut().unwrap();
            init_options.run_deinit_function(InitLevel::from_sys(init_level));
        }
    };
}

pub trait PtrCallArg {
    /// Read an argument value from a ptrcall argument.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are expected as they are provided by Godot.
    unsafe fn ptrcall_read(arg: gdext_sys::GDNativeTypePtr) -> Self;

    /// Write a value to a ptrcall argument or return value.
    ///
    /// # Safety
    ///
    /// Implementations of this function will use pointer casting and must make
    /// sure that the proper types are provided as they are expected by Godot.
    unsafe fn ptrcall_write(self, ret: gdext_sys::GDNativeTypePtr);
}

// Blanket implementation for all `GodotFfi` classes
impl<T: GodotFfi> PtrCallArg for T {
    unsafe fn ptrcall_read(arg: gdext_sys::GDNativeTypePtr) -> Self {
        Self::from_sys(arg)
    }

    unsafe fn ptrcall_write(self, ret: gdext_sys::GDNativeTypePtr) {
        self.write_sys(ret);
        std::mem::forget(self); // TODO double-check
    }
}

macro_rules! impl_ptr_call_arg_num {
    ($t:ty) => {
        impl PtrCallArg for $t {
            unsafe fn ptrcall_read(arg: gdext_sys::GDNativeTypePtr) -> Self {
                *(arg as *mut $t)
            }

            unsafe fn ptrcall_write(self, ret: gdext_sys::GDNativeTypePtr) {
                *(ret as *mut $t) = self;
            }
        }
    };
}

impl_ptr_call_arg_num!(u8);
impl_ptr_call_arg_num!(u16);
impl_ptr_call_arg_num!(u32);
impl_ptr_call_arg_num!(u64);

impl_ptr_call_arg_num!(i8);
impl_ptr_call_arg_num!(i16);
impl_ptr_call_arg_num!(i32);
impl_ptr_call_arg_num!(i64);

impl_ptr_call_arg_num!(f32);
impl_ptr_call_arg_num!(f64);

impl PtrCallArg for () {
    unsafe fn ptrcall_read(_arg: gdext_sys::GDNativeTypePtr) -> Self {}

    unsafe fn ptrcall_write(self, _arg: gdext_sys::GDNativeTypePtr) {
        // do nothing
    }
}
