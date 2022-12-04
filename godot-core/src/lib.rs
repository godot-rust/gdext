/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod registry;
mod storage;

pub mod bind;
pub mod builder;
pub mod builtin;
#[cfg(not(feature = "unit-test"))]
pub mod engine;
pub mod init;
pub mod log;
pub mod macros;
pub mod obj;

pub use registry::*;

pub use godot_ffi as sys;

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[cfg(not(feature = "unit-test"))]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
mod gen {
    pub mod central;
    pub mod classes;
    pub mod utilities;
}

#[cfg(feature = "unit-test")]
mod gen {
    pub mod central {
        pub mod global {}
    }
    pub mod classes {
        pub struct Node {}
        pub struct Resource {}

        pub mod class_macros {}
    }
    pub mod utilities {}
}

#[cfg(feature = "unit-test")]
pub mod engine {
    use super::sys;
    use crate::obj::{Gd, GodotClass};

    pub struct Object {}
    pub struct RefCounted {}
    impl RefCounted {
        pub fn init_ref(&self) -> bool {
            sys::panic_no_godot!(RefCounted::init_ref)
        }
        pub fn reference(&self) -> bool {
            sys::panic_no_godot!(RefCounted::reference)
        }
        pub fn unreference(&self) -> bool {
            sys::panic_no_godot!(RefCounted::unreference)
        }
    }

    impl GodotClass for Object {
        type Base = ();
        type Declarer = crate::obj::dom::EngineDomain;
        type Mem = crate::obj::mem::DynamicRefCount;
        const CLASS_NAME: &'static str = "";
    }
    impl GodotClass for RefCounted {
        type Base = Object;
        type Declarer = crate::obj::dom::EngineDomain;
        type Mem = crate::obj::mem::StaticRefCount;
        const CLASS_NAME: &'static str = "";
    }

    pub mod utilities {
        use super::sys;

        pub fn is_instance_id_valid(id: i64) -> bool {
            sys::panic_no_godot!(is_instance_id_valid)
        }
    }

    #[allow(non_camel_case_types)]
    pub mod global {
        use super::sys;

        #[derive(Debug)]
        pub enum PropertyHint {
            PROPERTY_HINT_NONE,
        }
        impl PropertyHint {
            pub fn ord(&self) -> i32 {
                sys::panic_no_godot!(PropertyHint::ord)
            }
        }

        #[derive(Debug)]
        pub enum PropertyUsageFlags {
            PROPERTY_USAGE_DEFAULT,
        }
        impl PropertyUsageFlags {
            pub fn ord(&self) -> i32 {
                sys::panic_no_godot!(PropertyUsageFlags::ord)
            }
        }
    }

    pub(crate) fn debug_string<T: GodotClass>(
        ptr: &Gd<T>,
        f: &mut std::fmt::Formatter<'_>,
        ty: &str,
    ) -> std::fmt::Result {
        sys::panic_no_godot!(Debug)
    }

    pub(crate) fn display_string<T: GodotClass>(
        ptr: &Gd<T>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        sys::panic_no_godot!(Display)
    }
}

pub mod callbacks {
    use super::sys;
    use crate::obj::{Base, GodotClass};

    pub unsafe extern "C" fn create<T>(
        _class_userdata: *mut std::ffi::c_void,
    ) -> sys::GDNativeObjectPtr {
        sys::panic_no_godot!(create)
    }

    pub(crate) fn create_custom<T, F>(_make_user_instance: F) -> sys::GDNativeObjectPtr
    where
        T: GodotClass,
        F: FnOnce(Base<T::Base>) -> T,
    {
        sys::panic_no_godot!(create_custom)
    }
}

#[doc(hidden)]
pub mod private {
    // If someone forgets #[godot_api], this causes a compile error, rather than virtual functions not being called at runtime.
    #[allow(non_camel_case_types)]
    pub trait You_forgot_the_attribute__godot_api {}

    pub use crate::gen::classes::class_macros;
    pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginComponent};
    pub use crate::storage::as_storage;
    pub use crate::{
        gdext_register_method, gdext_register_method_inner, gdext_virtual_method_callback,
    };

    use crate::{log, sys};

    sys::plugin_registry!(__GODOT_PLUGIN_REGISTRY: ClassPlugin);

    pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
        sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
    }

    pub fn print_panic(err: Box<dyn std::any::Any + Send>) {
        if let Some(s) = err.downcast_ref::<&'static str>() {
            log::godot_error!("rust-panic:  {}", s);
        } else if let Some(s) = err.downcast_ref::<String>() {
            log::godot_error!("rust-panic:  {}", s);
        } else {
            log::godot_error!("rust-panic of type ID {:?}", err.type_id());
        }
    }
}

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! out {
    ()                          => (eprintln!());
    ($fmt:literal)              => (eprintln!($fmt));
    ($fmt:literal, $($arg:tt)*) => (eprintln!($fmt, $($arg)*));
}

#[cfg(not(feature = "trace"))]
// TODO find a better way than sink-writing to avoid warnings, #[allow(unused_variables)] doesn't work
#[macro_export]
macro_rules! out {
    ()                          => ({});
    ($fmt:literal)              => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt); });
    ($fmt:literal, $($arg:tt)*) => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt, $($arg)*); };)
}
